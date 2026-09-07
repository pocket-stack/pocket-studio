use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::Serialize;
use tokio::sync::Notify;

use super::discovery::DeviceDiscovery;
use crate::domain::catalog::CatalogEntry;
use crate::domain::device::DeviceFacts;
use crate::domain::store::{BlobRef, CatalogError, VerifiedCatalog};

pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, StoreError>> + Send + 'a>>;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error("a trusted store source has not been configured")]
    Unconfigured,
    #[error("invalid store source configuration")]
    InvalidSource,
    #[error("the store could not be reached")]
    Network,
    #[error("invalid response from the store")]
    InvalidResponse,
    #[error("store cache could not be read or written")]
    Storage,
    #[error("the requested object is not in the verified catalog")]
    ObjectNotFound,
    #[error("download checksum does not match")]
    ChecksumMismatch,
    #[error("download was cancelled")]
    Cancelled,
}
impl StoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Catalog(error) => error.code(),
            Self::Unconfigured => "storeUnconfigured",
            Self::InvalidSource => "invalidStoreSource",
            Self::Network => "storeOffline",
            Self::InvalidResponse => "invalidStoreResponse",
            Self::Storage => "storeStorageFailed",
            Self::ObjectNotFound => "storeObjectNotFound",
            Self::ChecksumMismatch => "checksumMismatch",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogOrigin {
    Network,
    Cache,
    Unconfigured,
}
pub struct CatalogRead {
    pub verified: Option<Arc<VerifiedCatalog>>,
    pub origin: CatalogOrigin,
    pub checked_at: Option<u64>,
    pub issue: Option<String>,
    pub source_label: Option<String>,
}
#[derive(Default)]
pub struct StoreCancellation {
    cancelled: AtomicBool,
    changed: Notify,
}
impl StoreCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.changed.notify_one();
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    pub async fn wait(&self) {
        if !self.is_cancelled() {
            self.changed.notified().await;
        }
    }
}
pub struct DownloadControl {
    pub cancellation: Arc<StoreCancellation>,
    pub progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
}
impl Default for DownloadControl {
    fn default() -> Self {
        Self {
            cancellation: Arc::default(),
            progress: Arc::new(|_, _| {}),
        }
    }
}
pub struct VerifiedDownload {
    path: PathBuf,
    sha256: String,
}
impl VerifiedDownload {
    pub(crate) fn new(path: PathBuf, sha256: String) -> Self {
        Self { path, sha256 }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}
pub trait CatalogRepository: Send + Sync {
    fn read(&self, refresh: bool) -> StoreFuture<'_, CatalogRead>;
    fn media<'a>(&'a self, sha256: &'a str) -> StoreFuture<'a, PathBuf>;
    fn download<'a>(
        &'a self,
        blob: &'a BlobRef,
        control: DownloadControl,
    ) -> StoreFuture<'a, VerifiedDownload>;
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub entries: Vec<CatalogEntry>,
    pub source: CatalogOrigin,
    pub source_label: Option<String>,
    pub sequence: Option<u64>,
    pub expires_at: Option<u64>,
    pub checked_at: Option<u64>,
    pub expired: bool,
    pub verified: bool,
    pub issue: Option<String>,
}
pub struct StoreService {
    pub repository: Arc<dyn CatalogRepository>,
    discovery: Arc<DeviceDiscovery>,
}
impl StoreService {
    pub fn new(repository: Arc<dyn CatalogRepository>, discovery: Arc<DeviceDiscovery>) -> Self {
        Self {
            repository,
            discovery,
        }
    }
    pub async fn catalog(
        &self,
        device_id: Option<&str>,
        refresh: bool,
    ) -> Result<CatalogSnapshot, StoreError> {
        let read = self.repository.read(refresh).await?;
        let record = device_id.and_then(|id| self.discovery.find(id));
        let now = crate::domain::now_millis();
        let entries = read
            .verified
            .as_ref()
            .map(|v| {
                v.listings(
                    record.as_ref().map(|r| &r.summary),
                    record
                        .as_ref()
                        .map_or_else(DeviceFacts::default, |r| r.facts),
                    now,
                )
            })
            .unwrap_or_default();
        if refresh && let Some(verified) = &read.verified {
            tracing::info!(
                sequence = verified.catalog().sequence,
                applications = entries.len(),
                "verified store catalog loaded"
            );
        }
        Ok(CatalogSnapshot {
            entries,
            source: read.origin,
            source_label: read.source_label,
            sequence: read.verified.as_ref().map(|v| v.catalog().sequence),
            expires_at: read.verified.as_ref().map(|v| v.catalog().expires_at),
            checked_at: read.checked_at,
            expired: read.verified.as_ref().is_some_and(|v| v.expired_at(now)),
            verified: read.verified.is_some(),
            issue: read.issue,
        })
    }
}
