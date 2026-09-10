//! Explicit preparation plans for devices whose initial manager is installed
//! outside Studio. This service never jailbreaks or modifies firmware.
use super::{
    OperationLog,
    store::{CatalogRepository, DownloadControl},
};
use crate::domain::{
    catalog::PackageCategory,
    now_millis,
    store::{Artifact, Listing, NativeIdentity, ReleaseStatus, RuntimeCapability},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
};
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("3DS setup storage is unavailable")]
    Storage,
    #[error("select a 3DS SD card root")]
    InvalidCard,
    #[error("ftpd cannot access the 3DS SD card root")]
    FtpRoot,
    #[error("ftpd connection failed or was interrupted")]
    FtpConnection,
    #[error("ftpd rejected authentication")]
    FtpAuthentication,
    #[error("ftpd returned an unexpected response")]
    FtpProtocol,
    #[error("pairing key is invalid")]
    InvalidKey,
    #[error("setup destination changed; create a new plan")]
    Changed,
    #[error("3DS connection or identity could not be verified")]
    Device,
    #[error("3DS setup plan expired or is unavailable")]
    Plan,
    #[error("device access is busy")]
    Busy,
    #[error("a compatible launcher is not available in the signed catalog")]
    Launcher,
    #[error(transparent)]
    Store(#[from] crate::application::store::StoreError),
}
impl SetupError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage => "setupStorageFailed",
            Self::InvalidCard => "invalid3dsCard",
            Self::FtpRoot => "ftpRootUnavailable",
            Self::FtpConnection => "ftpConnectionFailed",
            Self::FtpAuthentication => "ftpAuthenticationFailed",
            Self::FtpProtocol => "invalidFtpResponse",
            Self::InvalidKey => "invalidPairingKey",
            Self::Changed => "deviceChanged",
            Self::Device => "pairingUnavailable",
            Self::Plan => "planExpired",
            Self::Busy => "operationBusy",
            Self::Launcher => "runtimeRequired",
            Self::Store(e) => e.code(),
        }
    }
}
pub type SetupFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, SetupError>> + Send + 'a>>;
#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SetupDestination {
    Sd {
        path: String,
    },
    Ftp {
        address: String,
        port: u16,
        username: String,
        password: String,
    },
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupRequest {
    pub runtime_requirement: Option<crate::domain::store::RuntimeRequirement>,
    pub host_abi: Option<u64>,
    pub destination: SetupDestination,
    pub address: Option<String>,
    pub format: Option<String>,
}
#[derive(Clone)]
pub struct SetupObservation {
    pub label: String,
    pub binding: String,
    pub token: Option<[u8; 32]>,
    pub previous_digest: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPlan {
    pub bootstrap_app_id: String,
    pub id: String,
    pub destination: String,
    pub existing_pairing: bool,
    pub artifact: Option<Artifact>,
    pub version: Option<String>,
    pub files: Vec<String>,
    pub expires_at: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    pub pairing_id: String,
    pub files_verified: bool,
    pub restart_required: bool,
}
pub trait SetupPort: Send + Sync {
    fn inspect<'a>(
        &'a self,
        request: &'a SetupRequest,
        file: Option<&'a str>,
    ) -> SetupFuture<'a, SetupObservation>;
    fn write<'a>(
        &'a self,
        request: &'a SetupRequest,
        observed: &'a SetupObservation,
        file: Option<(&'a str, &'a Path, &'a str)>,
    ) -> SetupFuture<'a, SetupResult>;
    fn connect<'a>(&'a self, pairing_id: &'a str, address: Option<&'a str>) -> SetupFuture<'a, ()>;
}
struct Pending {
    request: SetupRequest,
    observed: SetupObservation,
    plan: SetupPlan,
    relative: Option<String>,
}
pub struct SetupService {
    port: Arc<dyn SetupPort>,
    catalog: Arc<dyn CatalogRepository>,
    gate: Arc<tokio::sync::Semaphore>,
    pending: Mutex<HashMap<String, Pending>>,
    log: Arc<OperationLog>,
}
impl SetupService {
    pub fn new(
        port: Arc<dyn SetupPort>,
        catalog: Arc<dyn CatalogRepository>,
        gate: Arc<tokio::sync::Semaphore>,
        log: Arc<OperationLog>,
    ) -> Self {
        Self {
            port,
            catalog,
            gate,
            pending: Mutex::default(),
            log,
        }
    }
    pub async fn plan(&self, request: SetupRequest) -> Result<SetupPlan, SetupError> {
        let bootstrap_app_id = request
            .runtime_requirement
            .as_ref()
            .and_then(|r| r.bootstrap_app_id.clone())
            .unwrap_or_else(|| "dev.pocket-stack.launcher".into());
        let mut artifact = None;
        let mut version = None;
        let mut relative = None;
        if let Some(format) = &request.format {
            if !matches!(format.as_str(), "cia" | "3dsx") {
                return Err(SetupError::Launcher);
            }
            let read = self.catalog.read(true).await?;
            let catalog = read.verified.ok_or(SetupError::Launcher)?;
            if catalog.expired_at(now_millis()) {
                return Err(SetupError::Launcher);
            }
            let app = catalog
                .catalog()
                .apps
                .iter()
                .find(|a| {
                    a.id == bootstrap_app_id
                        && a.category == PackageCategory::Runtime
                        && a.listing == Listing::Listed
                })
                .ok_or(SetupError::Launcher)?;
            let mut releases: Vec<_> = catalog
                .catalog()
                .releases
                .iter()
                .filter(|r| r.app_id == app.id && r.status == ReleaseStatus::Published)
                .collect();
            releases.sort_by(|a, b| {
                semver::Version::parse(&b.version)
                    .unwrap()
                    .cmp(&semver::Version::parse(&a.version).unwrap())
                    .then(b.revision.cmp(&a.revision))
            });
            let (release, selected) = releases
                .iter()
                .find_map(|r| {
                    r.artifacts
                        .iter()
                        .find(|a| {
                            a.format == *format
                                && a.targets.iter().any(|t| {
                                    t.platform == "3ds"
                                        && request
                                            .host_abi
                                            .is_none_or(|abi| t.host_abi == Some(abi))
                                        // The console is not known yet; any New-family
                                        // target qualifies for preparation.
                                        && t.models
                                            .iter()
                                            .any(|m| crate::domain::three_ds::supported_model(m))
                                        && t.runtime_provides.as_ref().is_some_and(|p| {
                                            p.capabilities.contains(&RuntimeCapability::AppLibrary)
                                                && request.runtime_requirement.as_ref().is_none_or(
                                                    |r| {
                                                        r.id == p.id
                                                            && semver::Version::parse(
                                                                &r.min_version,
                                                            )
                                                            .ok()
                                                            .zip(
                                                                semver::Version::parse(&p.version)
                                                                    .ok(),
                                                            )
                                                            .is_some_and(|(required, actual)| {
                                                                actual >= required
                                                            })
                                                    },
                                                )
                                        })
                                })
                        })
                        .map(|a| (*r, a))
                })
                .ok_or(SetupError::Launcher)?;
            relative = Some(match &selected.native_identity {
                Some(NativeIdentity::HomebrewFile { entrypoint }) => entrypoint.clone(),
                Some(NativeIdentity::ThreeDsTitle { .. }) => {
                    format!("cias/pocket-launcher-{}.cia", selected.blob.sha256)
                }
                _ => return Err(SetupError::Launcher),
            });
            artifact = Some(selected.clone());
            version = Some(release.version.clone());
        }
        let observed = self.port.inspect(&request, relative.as_deref()).await?;
        let mut files = vec!["pocketjs/runtime/dev.key".into()];
        files.extend(relative.clone());
        let plan = SetupPlan {
            bootstrap_app_id,
            id: uuid::Uuid::new_v4().to_string(),
            destination: observed.label.clone(),
            existing_pairing: observed.token.is_some(),
            artifact,
            version,
            files,
            expires_at: now_millis() + 15 * 60 * 1000,
        };
        let mut pending = self.pending.lock().map_err(|_| SetupError::Storage)?;
        pending.retain(|_, p| p.plan.expires_at > now_millis());
        if pending.len() >= 16 {
            return Err(SetupError::Busy);
        }
        pending.insert(
            plan.id.clone(),
            Pending {
                request,
                observed,
                relative,
                plan: plan.clone(),
            },
        );
        Ok(plan)
    }
    pub async fn execute(&self, id: &str) -> Result<SetupResult, SetupError> {
        let _permit = self.gate.try_acquire().map_err(|_| SetupError::Busy)?;
        let pending = self
            .pending
            .lock()
            .map_err(|_| SetupError::Storage)?
            .remove(id)
            .ok_or(SetupError::Plan)?;
        if now_millis() >= pending.plan.expires_at {
            return Err(SetupError::Plan);
        }
        let download =
            if let Some(artifact) = &pending.plan.artifact {
                let read = self.catalog.read(true).await?;
                let catalog = read.verified.ok_or(SetupError::Launcher)?;
                if catalog.expired_at(now_millis())
                    || !catalog.catalog().apps.iter().any(|a| {
                        a.id == pending.plan.bootstrap_app_id && a.listing == Listing::Listed
                    })
                    || !catalog.catalog().releases.iter().any(|r| {
                        r.app_id == pending.plan.bootstrap_app_id
                            && r.status == ReleaseStatus::Published
                            && r.artifacts.contains(artifact)
                    })
                {
                    return Err(SetupError::Changed);
                }
                Some(
                    self.catalog
                        .download(&artifact.blob, DownloadControl::default())
                        .await?,
                )
            } else {
                None
            };
        if now_millis() >= pending.plan.expires_at {
            return Err(SetupError::Plan);
        }
        self.log.record(
            crate::domain::log::LogLevel::Info,
            crate::domain::log::LogSource::Device,
            "log.device.setupStarted",
            "User confirmed 3DS pairing and file preparation",
            None,
            None,
        );
        self.port
            .write(
                &pending.request,
                &pending.observed,
                pending
                    .relative
                    .as_deref()
                    .zip(download.as_ref())
                    .map(|(relative, download)| (relative, download.path(), download.sha256())),
            )
            .await
    }
    pub async fn connect(&self, id: &str, address: Option<&str>) -> Result<(), SetupError> {
        self.port.connect(id, address).await
    }
}
