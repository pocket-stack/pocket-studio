use super::store::CatalogRepository;
use crate::domain::installed::{
    InstallationObservation, InstalledReadState, InstalledSnapshot, map_installed,
};
use std::{future::Future, pin::Pin, sync::Arc};

#[derive(Debug, thiserror::Error)]
pub enum InstalledError {
    #[error("device is unavailable for application observation")]
    DeviceUnavailable,
    #[error("the device's installed applications could not be read")]
    ReadFailed,
    #[error("installed application observations could not be persisted")]
    Storage,
    #[error("a verified application catalog is required")]
    CatalogUnavailable,
}
impl InstalledError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceUnavailable => "deviceNotFound",
            Self::ReadFailed => "installedReadFailed",
            Self::Storage => "storeStorageFailed",
            Self::CatalogUnavailable => "storeUnconfigured",
        }
    }
}
pub type InstalledFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, InstalledError>> + Send + 'a>>;
pub trait InstalledReader: Send + Sync {
    fn platform(&self, device_id: &str) -> Result<crate::domain::device::Platform, InstalledError>;
    /// Stable opaque cache key; native identifiers remain inside infrastructure.
    fn binding(&self, device_id: &str) -> Result<String, InstalledError>;
    fn read<'a>(
        &'a self,
        device_id: &'a str,
        bundle_ids: &'a [String],
    ) -> InstalledFuture<'a, InstallationObservation>;
}
pub trait InstalledPersistence: Send + Sync {
    fn load_observation(
        &self,
        binding: &str,
        repository: &str,
    ) -> Result<Option<InstallationObservation>, InstalledError>;
    fn save_observation(
        &self,
        binding: &str,
        repository: &str,
        observation: &InstallationObservation,
    ) -> Result<(), InstalledError>;
}
pub struct InstalledService {
    pub reader: Arc<dyn InstalledReader>,
    pub persistence: Arc<dyn InstalledPersistence>,
    catalog: Arc<dyn CatalogRepository>,
}
impl InstalledService {
    pub fn new(
        reader: Arc<dyn InstalledReader>,
        persistence: Arc<dyn InstalledPersistence>,
        catalog: Arc<dyn CatalogRepository>,
    ) -> Self {
        Self {
            reader,
            persistence,
            catalog,
        }
    }
    pub async fn snapshot(&self, device_id: &str) -> Result<InstalledSnapshot, InstalledError> {
        let catalog = self
            .catalog
            .read(false)
            .await
            .map_err(|_| InstalledError::CatalogUnavailable)?
            .verified
            .ok_or(InstalledError::CatalogUnavailable)?;
        let binding = self.reader.binding(device_id)?;
        let repository = &catalog.catalog().repository_id;
        let mut ids: Vec<_> = catalog
            .catalog()
            .releases
            .iter()
            .flat_map(|r| {
                r.artifacts
                    .iter()
                    .filter_map(|a| a.ios_identity().map(|id| id.bundle_id.clone()))
            })
            .collect();
        if self.reader.platform(device_id)? == crate::domain::device::Platform::ThreeDs {
            ids = catalog
                .catalog()
                .apps
                .iter()
                .map(|app| app.id.clone())
                .collect();
        }
        ids.sort();
        ids.dedup();
        let (observation, state, issue) = match self.reader.read(device_id, &ids).await {
            Ok(observation) => {
                let issue = self
                    .persistence
                    .save_observation(&binding, repository, &observation)
                    .err()
                    .map(|e| e.code().to_owned());
                (Some(observation), InstalledReadState::Fresh, issue)
            }
            Err(error) => {
                let cached = self.persistence.load_observation(&binding, repository)?;
                let state = if cached.is_some() {
                    InstalledReadState::Stale
                } else {
                    InstalledReadState::Unavailable
                };
                (cached, state, Some(error.code().to_owned()))
            }
        };
        Ok(InstalledSnapshot {
            device_id: device_id.into(),
            entries: observation
                .as_ref()
                .map(|o| map_installed(catalog.catalog(), o))
                .unwrap_or_default(),
            state,
            observed_at: observation.as_ref().map(|o| o.observed_at),
            issue,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::store::{
            CatalogOrigin, CatalogRead, DownloadControl, StoreError, StoreFuture, VerifiedDownload,
        },
        domain::{
            installed::NativeApplication,
            store::{BlobRef, CatalogPointer, Trust, VerifiedCatalog},
        },
        infrastructure::store::StoreCache,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Source(Arc<VerifiedCatalog>);
    impl CatalogRepository for Source {
        fn read(&self, _: bool) -> StoreFuture<'_, CatalogRead> {
            Box::pin(async {
                Ok(CatalogRead {
                    verified: Some(self.0.clone()),
                    origin: CatalogOrigin::Cache,
                    checked_at: None,
                    issue: None,
                    source_label: None,
                })
            })
        }
        fn media<'a>(&'a self, _: &'a str) -> StoreFuture<'a, PathBuf> {
            Box::pin(async { Err(StoreError::ObjectNotFound) })
        }
        fn download<'a>(
            &'a self,
            _: &'a BlobRef,
            _: DownloadControl,
        ) -> StoreFuture<'a, VerifiedDownload> {
            Box::pin(async { Err(StoreError::ObjectNotFound) })
        }
    }
    struct Reader {
        fail: AtomicBool,
        empty: AtomicBool,
    }
    impl InstalledReader for Reader {
        fn platform(&self, _: &str) -> Result<crate::domain::device::Platform, InstalledError> {
            Ok(crate::domain::device::Platform::Ios)
        }
        fn binding(&self, id: &str) -> Result<String, InstalledError> {
            Ok(id.to_owned())
        }
        fn read<'a>(
            &'a self,
            _: &'a str,
            ids: &'a [String],
        ) -> InstalledFuture<'a, InstallationObservation> {
            Box::pin(async move {
                assert_eq!(ids, ["dev.example.notes.ios"]);
                if self.fail.load(Ordering::SeqCst) {
                    return Err(InstalledError::ReadFailed);
                }
                Ok(InstallationObservation {
                    managed: vec![],
                    observed_at: 123,
                    applications: if self.empty.load(Ordering::SeqCst) {
                        vec![]
                    } else {
                        vec![NativeApplication {
                            bundle_id: ids[0].clone(),
                            product_version: Some("1.0.0".into()),
                            build_number: Some("1".into()),
                            application_type: Some("User".into()),
                            receipt_build_id: None,
                        }]
                    },
                })
            })
        }
    }
    #[tokio::test]
    async fn failed_observation_retains_cache_but_confirmed_empty_replaces_it() {
        #[derive(serde::Deserialize)]
        struct Fixture {
            catalog_utf8: String,
            pointer: CatalogPointer,
            trust: Trust,
        }
        let f: Fixture =
            serde_json::from_str(include_str!("../../../contracts/store-v1/fixture-v1.json"))
                .unwrap();
        let source = Arc::new(Source(Arc::new(
            VerifiedCatalog::verify(f.pointer, f.catalog_utf8.as_bytes(), &f.trust, 0).unwrap(),
        )));
        let directory = tempfile::tempdir().unwrap();
        let reader = Arc::new(Reader {
            fail: AtomicBool::new(false),
            empty: AtomicBool::new(false),
        });
        let service = InstalledService::new(
            reader.clone(),
            Arc::new(StoreCache::open(directory.path().into()).unwrap()),
            source.clone(),
        );
        assert_eq!(
            service.snapshot("device-one").await.unwrap().entries.len(),
            1
        );
        drop(service);
        let service = InstalledService::new(
            reader.clone(),
            Arc::new(StoreCache::open(directory.path().into()).unwrap()),
            source,
        );
        reader.fail.store(true, Ordering::SeqCst);
        let failed = service.snapshot("device-one").await.unwrap();
        assert_eq!(failed.entries.len(), 1);
        assert_eq!(failed.state, InstalledReadState::Stale);
        assert_eq!(failed.observed_at, Some(123));
        let other = service.snapshot("device-two").await.unwrap();
        assert_eq!(other.state, InstalledReadState::Unavailable);
        assert!(other.entries.is_empty());
        reader.fail.store(false, Ordering::SeqCst);
        reader.empty.store(true, Ordering::SeqCst);
        let empty = service.snapshot("device-one").await.unwrap();
        assert_eq!(empty.state, InstalledReadState::Fresh);
        assert!(empty.entries.is_empty());
    }
}
