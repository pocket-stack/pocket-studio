//! HTTP and disk implementations of the static catalog port. Cloud credentials
//! are never accepted here; the only configured key material is public.
mod installed;
mod migrations;
mod packages;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fs2::FileExt;
use reqwest::{Client, StatusCode, Url, header};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as AsyncMutex;

use crate::application::store::{
    CatalogOrigin, CatalogRead, CatalogRepository, DownloadControl, StoreError, StoreFuture,
    VerifiedDownload,
};
use crate::domain::now_millis;
use crate::domain::store::{
    BlobRef, CatalogError, CatalogPointer, MAX_BLOB_BYTES, MAX_CATALOG_BYTES, Trust,
    VerifiedCatalog, is_digest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
    pub base_url: String,
    pub allow_local_http: bool,
    pub trust: Trust,
}
impl SourceConfig {
    pub fn from_environment() -> Result<Self, StoreError> {
        let path = std::env::var_os("POCKET_STORE_CONFIG");
        Self::from_path(path.as_deref().map(Path::new))
    }
    fn from_path(path: Option<&Path>) -> Result<Self, StoreError> {
        let Some(path) = path else {
            return serde_json::from_str(include_str!("store/official-source.json"))
                .map_err(|_| StoreError::InvalidSource);
        };
        if std::fs::metadata(path)
            .map_err(|_| StoreError::InvalidSource)?
            .len()
            > 64 * 1024
        {
            return Err(StoreError::InvalidSource);
        }
        let bytes = std::fs::read(path).map_err(|_| StoreError::InvalidSource)?;
        let config: Self = serde_json::from_slice(&bytes).map_err(|_| StoreError::InvalidSource)?;
        Ok(config)
    }
    fn url(&self) -> Result<Url, StoreError> {
        let url = Url::parse(&self.base_url).map_err(|_| StoreError::InvalidSource)?;
        let local = url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
        if !(url.scheme() == "https" || (url.scheme() == "http" && self.allow_local_http && local))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.path().ends_with('/')
            || self.trust.keys.is_empty()
            || uuid::Uuid::parse_str(&self.trust.repository_id).is_err()
        {
            return Err(StoreError::InvalidSource);
        }
        Ok(url)
    }
}
pub struct StoreCache {
    root: PathBuf,
    db: Mutex<Connection>,
}
struct CachedCatalog {
    base_url: String,
    pointer: String,
    bytes: Vec<u8>,
    etag: Option<String>,
    sequence: u64,
    checked_at: u64,
    digest: String,
}
impl StoreCache {
    pub fn open(root: PathBuf) -> Result<Self, StoreError> {
        std::fs::create_dir_all(root.join("blobs")).map_err(|_| StoreError::Storage)?;
        std::fs::create_dir_all(root.join("partial")).map_err(|_| StoreError::Storage)?;
        let mut db =
            Connection::open(root.join("store.sqlite")).map_err(|_| StoreError::Storage)?;
        db.busy_timeout(Duration::from_secs(5))
            .map_err(|_| StoreError::Storage)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS catalog_cache(repository_id TEXT PRIMARY KEY,base_url TEXT NOT NULL,pointer_json TEXT NOT NULL,catalog_bytes BLOB NOT NULL,etag TEXT,sequence INTEGER NOT NULL,checked_at INTEGER NOT NULL,catalog_digest TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS package_jobs(operation_id TEXT PRIMARY KEY,record_json TEXT NOT NULL,updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS installed_observations(device_key TEXT NOT NULL,repository_id TEXT NOT NULL,observation_json TEXT NOT NULL,PRIMARY KEY(device_key,repository_id));
            CREATE TABLE IF NOT EXISTS downloads(sha256 TEXT PRIMARY KEY,expected_size INTEGER NOT NULL,etag TEXT,downloaded_bytes INTEGER NOT NULL DEFAULT 0,status TEXT NOT NULL,updated_at INTEGER NOT NULL);").map_err(|_|StoreError::Storage)?;
        migrations::migrate(&mut db)?;
        Ok(Self {
            root,
            db: Mutex::new(db),
        })
    }
    fn load(&self, repository_id: &str) -> Result<Option<CachedCatalog>, StoreError> {
        self.db.lock().map_err(|_|StoreError::Storage)?.query_row("SELECT base_url,substr(pointer_json,1,8193),substr(catalog_bytes,1,1000001),etag,sequence,checked_at,catalog_digest FROM catalog_cache WHERE repository_id=?",[repository_id],|row|Ok(CachedCatalog{base_url:row.get(0)?,pointer:row.get(1)?,bytes:row.get(2)?,etag:row.get(3)?,sequence:row.get(4)?,checked_at:row.get(5)?,digest:row.get(6)?})).optional().map_err(|_|StoreError::Storage)
    }
    fn save(
        &self,
        config: &SourceConfig,
        pointer: &CatalogPointer,
        bytes: &[u8],
        etag: Option<&str>,
    ) -> Result<u64, StoreError> {
        let now = now_millis();
        let changed=self.db.lock().map_err(|_|StoreError::Storage)?.execute("INSERT INTO catalog_cache VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(repository_id) DO UPDATE SET base_url=excluded.base_url,pointer_json=excluded.pointer_json,catalog_bytes=excluded.catalog_bytes,etag=excluded.etag,sequence=excluded.sequence,checked_at=excluded.checked_at,catalog_digest=excluded.catalog_digest WHERE excluded.sequence>catalog_cache.sequence OR (excluded.sequence=catalog_cache.sequence AND excluded.catalog_digest=catalog_cache.catalog_digest)",params![config.trust.repository_id,config.base_url,serde_json::to_string(pointer).map_err(|_|StoreError::Storage)?,bytes,etag,pointer.sequence,now,pointer.catalog_sha256]).map_err(|_|StoreError::Storage)?;
        if changed == 0 {
            return Err(CatalogError::CatalogRollback.into());
        }
        Ok(now)
    }
    fn download_etag(&self, sha: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .db
            .lock()
            .map_err(|_| StoreError::Storage)?
            .query_row("SELECT etag FROM downloads WHERE sha256=?", [sha], |r| {
                r.get::<_, Option<String>>(0)
            })
            .optional()
            .map_err(|_| StoreError::Storage)?
            .flatten())
    }
    fn save_download(
        &self,
        blob: &BlobRef,
        etag: Option<&str>,
        bytes: u64,
        status: &str,
    ) -> Result<(), StoreError> {
        self.db.lock().map_err(|_|StoreError::Storage)?.execute("INSERT INTO downloads VALUES(?,?,?,?,?,?) ON CONFLICT(sha256) DO UPDATE SET etag=excluded.etag,downloaded_bytes=excluded.downloaded_bytes,status=excluded.status,updated_at=excluded.updated_at",params![blob.sha256,blob.size_bytes,etag,bytes,status,now_millis()]).map_err(|_|StoreError::Storage)?;
        Ok(())
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}
pub struct StaticCatalogRepository {
    config: Option<(SourceConfig, Url)>,
    cache: Arc<StoreCache>,
    client: Client,
    refresh_lock: AsyncMutex<()>,
    download_lock: AsyncMutex<()>,
}
impl StaticCatalogRepository {
    pub fn new(cache: Arc<StoreCache>, config: Option<SourceConfig>) -> Result<Self, StoreError> {
        let config = config
            .map(|config| {
                let url = config.url()?;
                Ok::<_, StoreError>((config, url))
            })
            .transpose()?;
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none());
        if config.as_ref().is_some_and(|(_, url)| {
            matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        }) {
            builder = builder.no_proxy();
        }
        let client = builder.build().map_err(|_| StoreError::Network)?;
        Ok(Self {
            config,
            cache,
            client,
            refresh_lock: AsyncMutex::new(()),
            download_lock: AsyncMutex::new(()),
        })
    }
    fn cached(
        &self,
        config: &SourceConfig,
    ) -> Result<Option<(Arc<VerifiedCatalog>, CachedCatalog)>, StoreError> {
        let Some(record) = self.cache.load(&config.trust.repository_id)? else {
            return Ok(None);
        };
        if record.pointer.len() > 8192 {
            return Err(CatalogError::InvalidCatalog.into());
        }
        let pointer =
            serde_json::from_str(&record.pointer).map_err(|_| CatalogError::InvalidCatalog)?;
        let verified =
            VerifiedCatalog::verify(pointer, &record.bytes, &config.trust, record.sequence)?;
        Ok(Some((Arc::new(verified), record)))
    }
    async fn read_catalog(&self, refresh: bool) -> Result<CatalogRead, StoreError> {
        let Some((config, base_url)) = &self.config else {
            return Ok(CatalogRead {
                verified: None,
                origin: CatalogOrigin::Unconfigured,
                checked_at: None,
                issue: Some(StoreError::Unconfigured.code().into()),
                source_label: None,
            });
        };
        let _guard = self.refresh_lock.lock().await;
        let label = base_url.host_str().map(str::to_owned);
        if !refresh && let Ok(Some((verified, record))) = self.cached(config) {
            return Ok(CatalogRead {
                verified: Some(verified),
                origin: CatalogOrigin::Cache,
                checked_at: Some(record.checked_at),
                issue: None,
                source_label: label,
            });
        }
        match self.fetch_catalog(config, base_url).await {
            Ok((verified, checked_at)) => Ok(CatalogRead {
                verified: Some(verified),
                origin: CatalogOrigin::Network,
                checked_at: Some(checked_at),
                issue: None,
                source_label: label,
            }),
            Err(error) => {
                tracing::warn!(code = error.code(), "store catalog refresh failed");
                if let Some((verified, record)) = self.cached(config)? {
                    return Ok(CatalogRead {
                        verified: Some(verified),
                        origin: CatalogOrigin::Cache,
                        checked_at: Some(record.checked_at),
                        issue: Some(error.code().into()),
                        source_label: label,
                    });
                }
                Err(error)
            }
        }
    }
    async fn fetch_catalog(
        &self,
        config: &SourceConfig,
        base_url: &Url,
    ) -> Result<(Arc<VerifiedCatalog>, u64), StoreError> {
        let previous = self.cache.load(&config.trust.repository_id)?;
        let valid_cached = self.cached(config).ok().flatten();
        let mut request = self
            .client
            .get(
                base_url
                    .join("v1/catalog/current.json")
                    .map_err(|_| StoreError::InvalidSource)?,
            )
            .timeout(Duration::from_secs(20));
        if valid_cached.is_some()
            && let Some(record) = &previous
            && record.base_url == config.base_url
            && let Some(etag) = &record.etag
        {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let response = request.send().await.map_err(|_| StoreError::Network)?;
        if response.status() == StatusCode::NOT_MODIFIED {
            let (verified, record) = valid_cached.ok_or(StoreError::InvalidResponse)?;
            let now = self.cache.save(
                config,
                verified.pointer(),
                &record.bytes,
                record.etag.as_deref(),
            )?;
            return Ok((verified, now));
        }
        if response.status() != StatusCode::OK {
            return Err(StoreError::InvalidResponse);
        }
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let pointer: CatalogPointer = serde_json::from_slice(&bounded_body(response, 8192).await?)
            .map_err(|_| CatalogError::InvalidCatalog)?;
        if previous.as_ref().is_some_and(|old| {
            old.sequence == pointer.sequence && old.digest != pointer.catalog_sha256
        }) {
            return Err(CatalogError::CatalogRollback.into());
        }
        if uuid::Uuid::parse_str(&pointer.publication_id).is_err() {
            return Err(CatalogError::InvalidCatalog.into());
        }
        let response = self
            .client
            .get(
                base_url
                    .join(&format!("v1/catalog/{}.json", pointer.publication_id))
                    .map_err(|_| StoreError::InvalidSource)?,
            )
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .map_err(|_| StoreError::Network)?;
        if response.status() != StatusCode::OK {
            return Err(StoreError::InvalidResponse);
        }
        let bytes = bounded_body(response, MAX_CATALOG_BYTES).await?;
        let verified = Arc::new(VerifiedCatalog::verify(
            pointer,
            &bytes,
            &config.trust,
            previous.as_ref().map_or(0, |r| r.sequence),
        )?);
        let now = self
            .cache
            .save(config, verified.pointer(), &bytes, etag.as_deref())?;
        Ok((verified, now))
    }
    async fn download_blob(
        &self,
        blob: &BlobRef,
        control: DownloadControl,
    ) -> Result<VerifiedDownload, StoreError> {
        if !is_digest(&blob.sha256) || blob.size_bytes > MAX_BLOB_BYTES {
            return Err(StoreError::ObjectNotFound);
        }
        let _guard = tokio::select! {guard=self.download_lock.lock()=>guard,()=control.cancellation.wait()=>return Err(StoreError::Cancelled)};
        // SQLite indexes and partial files are shared by separate Studio processes.
        // An OS lock is released on process exit and never needs a stale lock reset.
        let disk_lock = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(self.cache.root.join("download.lock"))
            .map_err(|_| StoreError::Storage)?;
        loop {
            match disk_lock.try_lock_exclusive() {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::select! {()=tokio::time::sleep(Duration::from_millis(100))=>{},()=control.cancellation.wait()=>return Err(StoreError::Cancelled)}
                }
                Err(_) => return Err(StoreError::Storage),
            }
        }
        if control.cancellation.is_cancelled() {
            return Err(StoreError::Cancelled);
        }
        let path = self.cache.root.join("blobs").join(&blob.sha256);
        if tokio::fs::try_exists(&path)
            .await
            .map_err(|_| StoreError::Storage)?
        {
            if matches_blob(&path, blob).await? {
                (control.progress)(blob.size_bytes, blob.size_bytes);
                return Ok(VerifiedDownload::new(path, blob.sha256.clone()));
            }
            tokio::fs::remove_file(&path)
                .await
                .map_err(|_| StoreError::Storage)?;
        }
        let part = self.cache.root.join("partial").join(&blob.sha256);
        let mut offset = match tokio::fs::metadata(&part).await {
            Ok(meta) => meta.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(_) => return Err(StoreError::Storage),
        };
        if offset == blob.size_bytes
            && tokio::fs::try_exists(&part)
                .await
                .map_err(|_| StoreError::Storage)?
            && matches_blob(&part, blob).await?
        {
            tokio::fs::rename(&part, &path)
                .await
                .map_err(|_| StoreError::Storage)?;
            self.cache.save_download(blob, None, offset, "complete")?;
            return Ok(VerifiedDownload::new(path, blob.sha256.clone()));
        }
        let (_, base_url) = self.config.as_ref().ok_or(StoreError::Unconfigured)?;
        let mut request = self
            .client
            .get(
                base_url
                    .join(&format!("v1/blobs/{}", blob.sha256))
                    .map_err(|_| StoreError::InvalidSource)?,
            )
            .timeout(Duration::from_secs(30 * 60));
        let previous_etag = self.cache.download_etag(&blob.sha256)?;
        if offset > blob.size_bytes || previous_etag.is_none() {
            offset = 0;
        }
        if offset > 0 {
            request = request
                .header(header::RANGE, format!("bytes={offset}-"))
                .header(
                    header::IF_RANGE,
                    previous_etag.as_ref().expect("checked ETag"),
                );
        }
        let mut response = tokio::select! {response=request.send()=>response.map_err(|_|StoreError::Network)?,()=control.cancellation.wait()=>return Err(StoreError::Cancelled)};
        if response.status() == StatusCode::OK {
            offset = 0;
        } else if response.status() == StatusCode::PARTIAL_CONTENT {
            let range = response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|r| r.to_str().ok())
                .ok_or(StoreError::InvalidResponse)?;
            if !valid_content_range(range, offset, blob.size_bytes) {
                return Err(StoreError::InvalidResponse);
            }
        } else {
            return Err(StoreError::InvalidResponse);
        }
        if response
            .content_length()
            .is_some_and(|length| length != blob.size_bytes - offset)
        {
            return Err(StoreError::InvalidResponse);
        }
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|e| e.to_str().ok())
            .map(str::to_owned);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(offset == 0)
            .append(offset > 0)
            .open(&part)
            .await
            .map_err(|_| StoreError::Storage)?;
        self.cache
            .save_download(blob, etag.as_deref(), offset, "partial")?;
        let transfer=async{
            loop {
                let chunk=tokio::select!{chunk=response.chunk()=>chunk.map_err(|_|StoreError::Network)?,()=control.cancellation.wait()=>return Err(StoreError::Cancelled)};
                let Some(chunk)=chunk else{break;};
                if offset+chunk.len() as u64>blob.size_bytes{return Err(StoreError::InvalidResponse);}
                file.write_all(&chunk).await.map_err(|_|StoreError::Storage)?;offset+=chunk.len() as u64;(control.progress)(offset,blob.size_bytes);
            }Ok(())
        }.await;
        let flushed = file.flush().await;
        transfer?;
        flushed.map_err(|_| StoreError::Storage)?;
        file.sync_all().await.map_err(|_| StoreError::Storage)?;
        drop(file);
        if control.cancellation.is_cancelled() {
            return Err(StoreError::Cancelled);
        }
        if offset != blob.size_bytes {
            return Err(StoreError::InvalidResponse);
        }
        if !matches_blob(&part, blob).await? {
            tokio::fs::remove_file(&part)
                .await
                .map_err(|_| StoreError::Storage)?;
            return Err(StoreError::ChecksumMismatch);
        }
        if control.cancellation.is_cancelled() {
            return Err(StoreError::Cancelled);
        }
        tokio::fs::rename(&part, &path)
            .await
            .map_err(|_| StoreError::Storage)?;
        self.cache
            .save_download(blob, etag.as_deref(), offset, "complete")?;
        Ok(VerifiedDownload::new(path, blob.sha256.clone()))
    }
}
impl CatalogRepository for StaticCatalogRepository {
    fn read(&self, refresh: bool) -> StoreFuture<'_, CatalogRead> {
        Box::pin(self.read_catalog(refresh))
    }
    fn media<'a>(&'a self, sha256: &'a str) -> StoreFuture<'a, PathBuf> {
        Box::pin(async move {
            let read = self.read_catalog(false).await?;
            let verified = read.verified.ok_or(StoreError::Unconfigured)?;
            let media = verified
                .catalog()
                .apps
                .iter()
                .flat_map(|a| &a.media)
                .find(|m| m.blob.sha256 == sha256)
                .ok_or(StoreError::ObjectNotFound)?;
            if media.blob.size_bytes > 16 * 1024 * 1024
                || !matches!(
                    media.blob.content_type.as_str(),
                    "image/png" | "image/jpeg" | "image/webp"
                )
            {
                return Err(StoreError::ObjectNotFound);
            }
            Ok(self
                .download_blob(&media.blob, DownloadControl::default())
                .await?
                .path()
                .to_owned())
        })
    }
    fn download<'a>(
        &'a self,
        blob: &'a BlobRef,
        control: DownloadControl,
    ) -> StoreFuture<'a, VerifiedDownload> {
        Box::pin(self.download_blob(blob, control))
    }
}
async fn bounded_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, StoreError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(StoreError::InvalidResponse);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| StoreError::Network)? {
        if bytes.len() + chunk.len() > limit {
            return Err(StoreError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
async fn matches_blob(path: &Path, blob: &BlobRef) -> Result<bool, StoreError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|_| StoreError::Storage)?;
    if file
        .metadata()
        .await
        .map_err(|_| StoreError::Storage)?
        .len()
        != blob.size_bytes
    {
        return Ok(false);
    }
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|_| StoreError::Storage)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()) == blob.sha256)
}
fn valid_content_range(value: &str, start: u64, total: u64) -> bool {
    let Some(value) = value.strip_prefix("bytes ") else {
        return false;
    };
    let Some((range, length)) = value.split_once('/') else {
        return false;
    };
    let Some((first, last)) = range.split_once('-') else {
        return false;
    };
    matches!((first.parse::<u64>(),last.parse::<u64>(),length.parse::<u64>()),(Ok(a),Ok(b),Ok(c))if a==start&&c==total&&b>=a&&b.checked_add(1)==Some(total))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod http_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_resumption_requires_the_exact_remaining_object() {
        assert!(valid_content_range("bytes 4-9/10", 4, 10));
        for value in [
            "bytes 3-9/10",
            "bytes 4-8/10",
            "bytes 4-9/11",
            "bytes */10",
            "bytes 4-18446744073709551615/10",
        ] {
            assert!(!valid_content_range(value, 4, 10));
        }
    }
    #[test]
    fn plaintext_sources_are_restricted_to_explicit_loopback_configuration() {
        let trust = Trust {
            repository_id: uuid::Uuid::new_v4().to_string(),
            keys: std::collections::BTreeMap::from([("key".into(), "public".into())]),
        };
        let mut source = SourceConfig {
            base_url: "http://127.0.0.1:8787/".into(),
            allow_local_http: true,
            trust,
        };
        assert!(source.url().is_ok());
        source.allow_local_http = false;
        assert!(source.url().is_err());
        source.allow_local_http = true;
        source.base_url = "http://example.com/".into();
        assert!(source.url().is_err());
        source.base_url = "https://user:password@example.com/".into();
        assert!(source.url().is_err());
    }
}
