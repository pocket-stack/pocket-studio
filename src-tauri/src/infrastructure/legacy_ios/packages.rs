use super::{LegacyIosProbe, installed::read_registered, probe_normal_with_appsync};
use crate::{
    application::{
        packages::{
            PackageDriver, PackageError, PackageFuture, PackageObserver, PackageProgress,
            PackageTarget, validate_action,
        },
        store::StoreCancellation,
    },
    domain::{
        packages::*,
        store::{StoreVerdict, evaluate_target, sha256_hex},
    },
};
use legacy_ios_services::{
    AppFailure, AppIdentifier, AppInstallMode, AppOperationControl, AppOperationObserver,
    AppOperationTimeouts, AppProgress, IpaPackage, NormalDevice, ServiceError,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::timeout;

pub struct LegacyPackageDriver {
    probe: Arc<LegacyIosProbe>,
}
impl LegacyPackageDriver {
    pub fn new(probe: Arc<LegacyIosProbe>) -> Self {
        Self { probe }
    }
}
struct Target {
    probe: Arc<LegacyIosProbe>,
    device_id: String,
    key: String,
    binding: String,
}
impl PackageDriver for LegacyPackageDriver {
    fn target(&self, device_id: &str) -> Result<Arc<dyn PackageTarget>, PackageError> {
        let key = self
            .probe
            .application_session_key(device_id)
            .map_err(|_| PackageError::DeviceChanged)?;
        Ok(Arc::new(Target {
            probe: self.probe.clone(),
            device_id: device_id.into(),
            binding: sha256_hex(key.as_bytes()),
            key,
        }))
    }
}
impl Target {
    async fn device(&self) -> Result<NormalDevice, PackageError> {
        if self
            .probe
            .application_session_key(&self.device_id)
            .map_err(|_| PackageError::DeviceChanged)?
            != self.key
        {
            return Err(PackageError::DeviceChanged);
        }
        let udid = legacy_ios_core::Udid::new(
            self.key
                .strip_prefix("udid:")
                .expect("validated session key"),
        );
        timeout(Duration::from_secs(5), self.probe.normal.find_device(&udid))
            .await
            .map_err(|_| PackageError::DeviceChanged)?
            .map_err(|_| PackageError::DeviceChanged)
    }
    async fn observe(
        &self,
        device: &NormalDevice,
        ids: &[String],
    ) -> Result<PackageObservation, PackageError> {
        let appsync_session = self
            .probe
            .appsync_sessions
            .lock()
            .expect("AppSync sessions poisoned")
            .get(&self.key)
            .cloned();
        let (record, _) = timeout(
            Duration::from_secs(25),
            probe_normal_with_appsync(device, self.device_id.clone(), appsync_session),
        )
        .await
        .map_err(|_| PackageError::DeviceChanged)?;
        if record.facts.pairing_trusted != Some(true) {
            return Err(PackageError::DeviceChanged);
        }
        let applications = read_registered(device, ids)
            .await
            .map_err(|_| PackageError::DeviceChanged)?;
        let appsync = match record.facts.appsync_installed {
            Some(true) => RequirementState::Satisfied,
            Some(false) => RequirementState::Missing,
            None => RequirementState::Unknown,
        };
        Ok(PackageObservation {
            managed: vec![],
            device: record.summary,
            facts: record.facts,
            appsync,
            applications,
        })
    }
}
impl PackageTarget for Target {
    fn platform(&self) -> crate::domain::device::Platform {
        crate::domain::device::Platform::Ios
    }
    fn binding(&self) -> &str {
        &self.binding
    }
    fn inspect<'a>(&'a self, bundle_ids: &'a [String]) -> PackageFuture<'a, PackageObservation> {
        Box::pin(async move {
            let _access = timeout(Duration::from_secs(3), self.probe.normal_access.read())
                .await
                .map_err(|_| PackageError::Busy)?;
            let device = self.device().await?;
            self.observe(&device, bundle_ids).await
        })
    }
    fn write<'a>(
        &'a self,
        plan: &'a PackagePlan,
        path: Option<&'a Path>,
        cancellation: Arc<StoreCancellation>,
        observer: Arc<dyn PackageObserver>,
    ) -> PackageFuture<'a, ()> {
        Box::pin(async move {
            let package = if let Some(artifact) = &plan.artifact {
                let path = path.ok_or(PackageError::ChecksumMismatch)?;
                let package = IpaPackage::open(path)
                    .await
                    .map_err(|_| PackageError::ChecksumMismatch)?;
                let digest = package
                    .sha256()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                let metadata = package.metadata();
                let identity = artifact.ios_identity().ok_or(PackageError::Incompatible)?;
                if digest != artifact.blob.sha256
                    || package.size_bytes() != artifact.blob.size_bytes
                    || metadata.bundle_id().as_str() != identity.bundle_id
                    || metadata.product_version() != Some(&identity.version)
                    || metadata.build_version() != Some(&identity.build_number)
                {
                    return Err(PackageError::ChecksumMismatch);
                }
                Some(package)
            } else {
                None
            };
            // Drain read-only scans and retain identity through upload, submission
            // and the device response. Preparation shares the application's gate.
            let _access = self.probe.normal_access.clone().write_owned().await;
            if cancellation.is_cancelled() {
                return Err(PackageError::Cancelled);
            }
            let device = self.device().await?;
            let observed = self.observe(&device, &plan.inspection_keys()).await?;
            let actual = observed
                .applications
                .iter()
                .find(|a| Some(a.bundle_id.as_str()) == plan.bundle_id());
            if !same_installation(plan.previous_ios(), actual) {
                return Err(PackageError::StateChanged);
            }
            validate_action(plan.action, actual, plan.artifact.as_ref())?;
            if let (Some(artifact), Some(target)) = (&plan.artifact, &plan.target) {
                if evaluate_target(artifact, target, Some(&observed.device), observed.facts)
                    != StoreVerdict::Compatible
                {
                    return Err(PackageError::Incompatible);
                }
                if target.requires.appsync() && observed.appsync == RequirementState::Missing {
                    return Err(PackageError::NeedsAppSync);
                }
            }
            let control = AppOperationControl::default();
            let observer = Bridge {
                observer,
                error: Mutex::new(None),
            };
            let operation = async {
                if plan.action == PackageAction::Uninstall {
                    device
                        .uninstall_user_app(
                            &AppIdentifier::parse(
                                plan.bundle_id().ok_or(PackageError::InvalidAction)?,
                            )
                            .map_err(|_| PackageError::InvalidAction)?,
                            &control,
                            &observer,
                            AppOperationTimeouts::default(),
                        )
                        .await
                        .map_err(native_error)?;
                } else {
                    let mode = if plan.action == PackageAction::Install {
                        AppInstallMode::Install
                    } else {
                        AppInstallMode::Upgrade
                    };
                    device
                        .install_user_app(
                            package.ok_or(PackageError::ChecksumMismatch)?,
                            mode,
                            &control,
                            &observer,
                            AppOperationTimeouts::default(),
                        )
                        .await
                        .map_err(native_error)?;
                }
                Ok(())
            };
            tokio::pin!(operation);
            let result = tokio::select! {result=&mut operation=>result,_=cancellation.wait()=>{let _=control.cancel();operation.await}};
            if let Some(error) = observer
                .error
                .lock()
                .map_err(|_| PackageError::Storage)?
                .take()
            {
                return Err(error);
            }
            result
        })
    }
}
struct Bridge {
    observer: Arc<dyn PackageObserver>,
    error: Mutex<Option<PackageError>>,
}
impl AppOperationObserver for Bridge {
    fn before_commit(&self) -> Result<(), ServiceError> {
        self.observer.before_submit().map_err(|error| {
            if let Ok(mut slot) = self.error.lock() {
                *slot = Some(error);
            }
            AppFailure::ObserverRejected.into()
        })
    }
    fn progress(&self, event: AppProgress) {
        match event {
            AppProgress::Transfer { bytes, total } => self
                .observer
                .progress(PackageProgress::Transfer { bytes, total }),
            AppProgress::Committing => self.observer.progress(PackageProgress::Submitted),
            AppProgress::Device { percent } => {
                self.observer.progress(PackageProgress::Device { percent })
            }
            AppProgress::Cleanup { complete } => self
                .observer
                .progress(PackageProgress::Cleanup { complete }),
            AppProgress::Staging { path } => {
                self.observer.progress(PackageProgress::Staging { path })
            }
        }
    }
}
fn native_error(error: ServiceError) -> PackageError {
    match error {
        ServiceError::Application(AppFailure::Cancelled) => PackageError::Cancelled,
        ServiceError::Application(AppFailure::DigestMismatch | AppFailure::InvalidPackage) => {
            PackageError::ChecksumMismatch
        }
        ServiceError::Application(AppFailure::Rejected(
            legacy_ios_services::AppRejection::Signature,
        )) => PackageError::SignatureRejected,
        ServiceError::Application(AppFailure::Rejected(
            legacy_ios_services::AppRejection::Storage,
        )) => PackageError::DeviceStorageFull,
        ServiceError::Application(AppFailure::Rejected(_)) => PackageError::WriteRejected,
        ServiceError::Application(AppFailure::SystemApplication) => PackageError::SystemApplication,
        ServiceError::Application(AppFailure::UnknownApplicationType) => {
            PackageError::ApplicationTypeUnknown
        }
        ServiceError::Application(AppFailure::NotInstalled | AppFailure::AlreadyInstalled) => {
            PackageError::StateChanged
        }
        _ => PackageError::DeviceChanged,
    }
}
