use super::*;
use crate::application::store::StoreCancellation;
use crate::domain::store::{CATALOG_LIFETIME_MS, Catalog, sha256_hex};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

struct Reply {
    status: u16,
    body: Vec<u8>,
    headers: Vec<(&'static str, String)>,
}
impl Reply {
    fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            body: body.into(),
            headers: vec![],
        }
    }
    fn header(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((key, value.into()));
        self
    }
}
struct Server {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    requests: Arc<Mutex<Vec<String>>>,
}
impl Server {
    fn new(handler: impl Fn(&str) -> Reply + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(vec![]));
        let stopping = stop.clone();
        let captured = requests.clone();
        let thread = thread::spawn(move || {
            for connection in listener.incoming() {
                if stopping.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(mut stream) = connection else {
                    break;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = vec![];
                let mut byte = [0u8; 1];
                while bytes.len() < 16 * 1024 && !bytes.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut byte) {
                        Ok(1) => bytes.push(byte[0]),
                        _ => break,
                    }
                }
                let request = String::from_utf8(bytes).unwrap();
                if request.is_empty() {
                    continue;
                }
                captured.lock().unwrap().push(request.clone());
                let reply = handler(&request);
                let mut headers = format!(
                    "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n",
                    reply.status,
                    reply.body.len()
                );
                for (key, value) in reply.headers {
                    headers.push_str(&format!("{key}: {value}\r\n"));
                }
                headers.push_str("\r\n");
                if stream.write_all(headers.as_bytes()).is_ok() {
                    let _ = stream.write_all(&reply.body);
                }
            }
        });
        Self {
            address,
            stop,
            thread: Some(thread),
            requests,
        }
    }
    fn url(&self) -> String {
        format!("http://{}/", self.address)
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}
#[derive(Deserialize)]
struct Fixture {
    catalog_utf8: String,
    pointer: CatalogPointer,
    trust: Trust,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../../../contracts/store-v1/fixture-v1.json")).unwrap()
}
#[derive(Clone)]
struct Wire {
    pointer: CatalogPointer,
    bytes: Vec<u8>,
}
fn wire(sequence: u64, name: &str) -> Wire {
    let f = fixture();
    let mut catalog: Catalog = serde_json::from_str(&f.catalog_utf8).unwrap();
    catalog.sequence = sequence;
    catalog.publication_id = uuid::Uuid::new_v4().to_string();
    catalog.created_at = now_millis().saturating_sub(1000);
    catalog.expires_at = catalog.created_at + CATALOG_LIFETIME_MS;
    catalog.apps[0].locales.get_mut("en").unwrap().name = name.into();
    let bytes = serde_json::to_vec(&catalog).unwrap();
    let key = SigningKey::from_bytes(&[1; 32]);
    let mut pointer = f.pointer;
    pointer.sequence = sequence;
    pointer.publication_id = catalog.publication_id;
    pointer.catalog_sha256 = sha256_hex(&bytes);
    pointer.signature = STANDARD.encode(key.sign(&bytes).to_bytes());
    Wire { pointer, bytes }
}
fn source(url: String) -> SourceConfig {
    SourceConfig {
        base_url: url,
        allow_local_http: true,
        trust: fixture().trust,
    }
}
fn repository(root: &Path, url: String) -> StaticCatalogRepository {
    StaticCatalogRepository::new(
        Arc::new(StoreCache::open(root.to_owned()).unwrap()),
        Some(source(url)),
    )
    .unwrap()
}
fn blob() -> BlobRef {
    BlobRef {
        sha256: sha256_hex(b"test IPA"),
        size_bytes: 8,
        content_type: "application/octet-stream".into(),
    }
}

#[tokio::test]
async fn cache_survives_restart_rejects_rollback_and_never_trusts_corrupt_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let current = Arc::new(Mutex::new(wire(1, "First")));
    let served = current.clone();
    let server = Server::new(move |request| {
        let wire = served.lock().unwrap().clone();
        let etag = format!("\"{}\"", wire.pointer.catalog_sha256);
        if request.starts_with("GET /v1/catalog/current.json ") {
            if request.contains(&format!("if-none-match: {etag}")) {
                Reply::new(304, vec![])
            } else {
                Reply::new(200, serde_json::to_vec(&wire.pointer).unwrap()).header("ETag", etag)
            }
        } else {
            Reply::new(200, wire.bytes)
        }
    });
    let url = server.url();
    let repo = repository(directory.path(), url.clone());
    assert_eq!(
        repo.read(true)
            .await
            .unwrap()
            .verified
            .unwrap()
            .catalog()
            .sequence,
        1
    );
    repo.read(true).await.unwrap();
    assert!(
        server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.contains("if-none-match:"))
    );
    *current.lock().unwrap() = wire(2, "Second");
    repo.read(true).await.unwrap();
    *current.lock().unwrap() = wire(1, "Old");
    let rejected = repo.read(true).await.unwrap();
    assert_eq!(rejected.issue.as_deref(), Some("catalogRollback"));
    assert_eq!(rejected.verified.unwrap().catalog().sequence, 2);
    *current.lock().unwrap() = wire(2, "Fork");
    let fork = repo.read(true).await.unwrap();
    assert_eq!(fork.issue.as_deref(), Some("catalogRollback"));
    assert_eq!(
        fork.verified.unwrap().catalog().apps[0].locales["en"].name,
        "Second"
    );
    drop(repo);
    drop(server);
    let restarted = repository(directory.path(), url);
    let offline = restarted.read(true).await.unwrap();
    assert!(matches!(offline.origin, CatalogOrigin::Cache));
    assert_eq!(offline.issue.as_deref(), Some("storeOffline"));
    assert_eq!(offline.verified.unwrap().catalog().sequence, 2);
    restarted
        .cache
        .db
        .lock()
        .unwrap()
        .execute("UPDATE catalog_cache SET catalog_bytes=x'00'", [])
        .unwrap();
    assert!(restarted.read(false).await.is_err());
}
#[tokio::test]
async fn resume_checks_range_and_hash_and_reuses_the_verified_cache() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::new(|request| {
        assert!(request.contains("range: bytes=3-"));
        assert!(request.contains("if-range: \"blob\""));
        Reply::new(206, b"t IPA".to_vec())
            .header("Content-Range", "bytes 3-7/8")
            .header("ETag", "\"blob\"")
    });
    let repo = repository(directory.path(), server.url());
    let blob = blob();
    std::fs::write(repo.cache.root.join("partial").join(&blob.sha256), b"tes").unwrap();
    repo.cache
        .save_download(&blob, Some("\"blob\""), 3, "partial")
        .unwrap();
    let downloaded = repo
        .download(&blob, DownloadControl::default())
        .await
        .unwrap();
    assert_eq!(std::fs::read(downloaded.path()).unwrap(), b"test IPA");
    assert_eq!(downloaded.sha256(), blob.sha256);
    let requests = server.requests.lock().unwrap().len();
    repo.download(&blob, DownloadControl::default())
        .await
        .unwrap();
    assert_eq!(server.requests.lock().unwrap().len(), requests);
}
#[tokio::test]
async fn changed_range_validator_restarts_from_zero_and_bad_bytes_are_not_promoted() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::new(|_| Reply::new(200, b"test IPA".to_vec()).header("ETag", "\"new\""));
    let repo = repository(directory.path(), server.url());
    let blob = blob();
    std::fs::write(repo.cache.root.join("partial").join(&blob.sha256), b"BAD").unwrap();
    repo.cache
        .save_download(&blob, Some("\"old\""), 3, "partial")
        .unwrap();
    let downloaded = repo
        .download(&blob, DownloadControl::default())
        .await
        .unwrap();
    assert_eq!(std::fs::read(downloaded.path()).unwrap(), b"test IPA");
    drop(repo);
    drop(server);
    let corrupt = Server::new(|_| Reply::new(200, b"corrupt!".to_vec()));
    let other = tempfile::tempdir().unwrap();
    let repo = repository(other.path(), corrupt.url());
    assert!(matches!(
        repo.download(&blob, DownloadControl::default()).await,
        Err(StoreError::ChecksumMismatch)
    ));
    assert!(!repo.cache.root.join("blobs").join(&blob.sha256).exists());
}
#[tokio::test]
async fn cancellation_keeps_partial_data_but_never_returns_a_verified_download() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::new(|_| Reply::new(200, b"test IPA".to_vec()).header("ETag", "\"blob\""));
    let repo = repository(directory.path(), server.url());
    let blob = blob();
    let cancellation = Arc::new(StoreCancellation::default());
    let cancel = cancellation.clone();
    let control = DownloadControl {
        cancellation,
        progress: Arc::new(move |_, _| cancel.cancel()),
    };
    assert!(matches!(
        repo.download(&blob, control).await,
        Err(StoreError::Cancelled)
    ));
    assert!(!repo.cache.root.join("blobs").join(&blob.sha256).exists());
    assert_eq!(
        std::fs::read(repo.cache.root.join("partial").join(&blob.sha256)).unwrap(),
        b"test IPA"
    );
}
