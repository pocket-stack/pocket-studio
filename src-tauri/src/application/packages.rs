use super::{
    EventSink, OperationLog,
    store::{CatalogRepository, DownloadControl, StoreCancellation},
};
use crate::domain::{
    installed::NativeApplication,
    log::{LogLevel, LogSource},
    now_millis,
    operation::*,
    packages::*,
    store::{Listing, ReleaseStatus, StoreVerdict, VerifiedCatalog, evaluate_target},
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum PackageError {
    #[error("a valid trusted catalog is required")]
    CatalogUnavailable,
    #[error("the catalog has expired; refresh it before installing")]
    CatalogExpired,
    #[error("this application or release has been withdrawn")]
    Withdrawn,
    #[error("this artifact is incompatible with the device")]
    Incompatible,
    #[error("the bound device is unavailable or has changed")]
    DeviceChanged,
    #[error("the observed installation changed; review a new plan")]
    StateChanged,
    #[error("the selected artifact would downgrade the installed app")]
    Downgrade,
    #[error("this action does not match the installed state")]
    InvalidAction,
    #[error("AppSync is missing; install it on the device and refresh")]
    NeedsAppSync,
    #[error("an operation for this app is already queued or running")]
    Busy,
    #[error("the operation plan expired or was already used")]
    PlanExpired,
    #[error("the operation consent is incomplete")]
    InvalidConsent,
    #[error("the operation cannot be cancelled after submission")]
    NotCancellable,
    #[error("the operation is unknown or has finished")]
    UnknownOperation,
    #[error("operation state could not be persisted")]
    Storage,
    #[error("the device rejected the application operation")]
    WriteRejected,
    #[error("the device rejected the IPA signature; check AppSync and signing")]
    SignatureRejected,
    #[error("the device reported insufficient storage")]
    DeviceStorageFull,
    #[error("the application could not be verified; retry read-only verification")]
    VerificationUnavailable,
    #[error("the device's registered application does not match the selected artifact")]
    VerificationFailed,
    #[error("the IPA does not match its signed artifact")]
    ChecksumMismatch,
    #[error("the package could not be downloaded")]
    DownloadFailed,
    #[error("operation cancelled before device submission")]
    Cancelled,
}
impl PackageError {
    pub fn code(self) -> &'static str {
        match self {
            Self::CatalogUnavailable => "storeUnconfigured",
            Self::CatalogExpired => "catalogExpired",
            Self::Withdrawn => "packageWithdrawn",
            Self::Incompatible => "incompatiblePackage",
            Self::DeviceChanged => "deviceChanged",
            Self::StateChanged => "installedStateChanged",
            Self::Downgrade => "packageDowngrade",
            Self::InvalidAction => "invalidPackageAction",
            Self::NeedsAppSync => "appsyncMissing",
            Self::Busy => "operationBusy",
            Self::PlanExpired => "planExpired",
            Self::InvalidConsent => "invalidConsent",
            Self::NotCancellable => "notCancellable",
            Self::UnknownOperation => "unknownOperation",
            Self::Storage => "storeStorageFailed",
            Self::WriteRejected => "installRejected",
            Self::SignatureRejected => "signatureRejected",
            Self::DeviceStorageFull => "deviceStorageFull",
            Self::VerificationUnavailable => "verificationUnavailable",
            Self::VerificationFailed => "verificationFailed",
            Self::ChecksumMismatch => "checksumMismatch",
            Self::DownloadFailed => "downloadFailed",
            Self::Cancelled => "cancelled",
        }
    }
    fn operation_error(self) -> OperationError {
        OperationError {
            code: match self {
                Self::DeviceChanged => OperationErrorCode::DeviceChanged,
                Self::ChecksumMismatch => OperationErrorCode::ChecksumMismatch,
                Self::DownloadFailed => OperationErrorCode::DownloadFailed,
                Self::VerificationUnavailable => OperationErrorCode::VerificationUnavailable,
                Self::VerificationFailed => OperationErrorCode::VerificationFailed,
                Self::Cancelled => OperationErrorCode::Cancelled,
                _ => OperationErrorCode::InstallRejected,
            },
            recoverable: matches!(self, Self::VerificationUnavailable),
            retry_from_step_id: None,
            diagnostic: Some(OperationDiagnostic {
                stage: "package".into(),
                reason: self.code().into(),
            }),
        }
    }
}
pub type PackageFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, PackageError>> + Send + 'a>>;
pub trait PackageTarget: Send + Sync {
    fn binding(&self) -> &str;
    fn inspect<'a>(&'a self, bundle_ids: &'a [String]) -> PackageFuture<'a, PackageObservation>;
    fn write<'a>(
        &'a self,
        plan: &'a PackagePlan,
        path: Option<&'a Path>,
        cancellation: Arc<StoreCancellation>,
        observer: Arc<dyn PackageObserver>,
    ) -> PackageFuture<'a, ()>;
}
pub trait PackageDriver: Send + Sync {
    fn target(&self, device_id: &str) -> Result<Arc<dyn PackageTarget>, PackageError>;
}
pub enum PackageProgress {
    Staging { path: String },
    Transfer { bytes: u64, total: u64 },
    Submitted,
    Device { percent: Option<u8> },
    Cleanup { complete: bool },
}
pub trait PackageObserver: Send + Sync {
    fn before_submit(&self) -> Result<(), PackageError>;
    fn progress(&self, event: PackageProgress);
}
pub trait PackagePersistence: Send + Sync {
    fn jobs(&self) -> Result<Vec<StoredPackageJob>, PackageError>;
    fn save_job(&self, job: &StoredPackageJob) -> Result<(), PackageError>;
}
struct PendingPlan {
    plan: PackagePlan,
    repository_id: String,
    target: Arc<dyn PackageTarget>,
}
struct QueuedRun {
    id: String,
    pending: PendingPlan,
    cancellation: Arc<StoreCancellation>,
}
#[derive(Default)]
struct State {
    queue: VecDeque<QueuedRun>,
    worker_running: bool,
    next_order: u64,
    plans: HashMap<String, PendingPlan>,
    jobs: HashMap<String, StoredPackageJob>,
    cancellations: HashMap<String, Arc<StoreCancellation>>,
}
pub struct PackageService {
    driver: Arc<dyn PackageDriver>,
    catalog: Arc<dyn CatalogRepository>,
    persistence: Arc<dyn PackagePersistence>,
    sink: Arc<dyn EventSink>,
    log: Arc<OperationLog>,
    gate: Arc<Semaphore>,
    state: Mutex<State>,
}
impl PackageService {
    pub fn new(
        driver: Arc<dyn PackageDriver>,
        catalog: Arc<dyn CatalogRepository>,
        persistence: Arc<dyn PackagePersistence>,
        sink: Arc<dyn EventSink>,
        log: Arc<OperationLog>,
        gate: Arc<Semaphore>,
    ) -> Result<Self, PackageError> {
        let mut state = State::default();
        for mut stored in persistence.jobs()? {
            if stored.job.phase.active() {
                stored.job.phase = PackagePhase::Interrupted;
                stored.job.error = Some(PackageError::VerificationUnavailable.operation_error());
                stored.job.updated_at = now_millis();
                persistence.save_job(&stored)?;
            }
            state.next_order = state.next_order.max(stored.job.queue_order);
            state
                .jobs
                .insert(stored.job.handle.operation_id.clone(), stored);
        }
        Ok(Self {
            driver,
            catalog,
            persistence,
            sink,
            log,
            gate,
            state: Mutex::new(state),
        })
    }
    async fn catalog(&self, refresh: bool) -> Result<Arc<VerifiedCatalog>, PackageError> {
        self.catalog
            .read(refresh)
            .await
            .map_err(|_| PackageError::CatalogUnavailable)?
            .verified
            .ok_or(PackageError::CatalogUnavailable)
    }
    pub async fn plan(&self, request: PackageRequest) -> Result<PackagePlan, PackageError> {
        let catalog = self.catalog(true).await?;
        let app = catalog
            .catalog()
            .apps
            .iter()
            .find(|app| app.id == request.app_id)
            .ok_or(PackageError::InvalidAction)?;
        let target = self.driver.target(&request.device_id)?;
        let mut bundle_ids: Vec<_> = catalog
            .catalog()
            .releases
            .iter()
            .filter(|r| r.app_id == app.id)
            .flat_map(|r| {
                r.artifacts
                    .iter()
                    .map(|a| a.native_identity.bundle_id.clone())
            })
            .collect();
        bundle_ids.sort();
        bundle_ids.dedup();
        let observation = target.inspect(&bundle_ids).await?;
        let selection = if request.action == PackageAction::Uninstall {
            None
        } else {
            if catalog.expired_at(now_millis()) {
                return Err(PackageError::CatalogExpired);
            }
            let selected = catalog
                .select(
                    &app.id,
                    Some(&observation.device),
                    observation.facts,
                    now_millis(),
                )
                .ok_or(PackageError::InvalidAction)?;
            if selected.verdict != StoreVerdict::Compatible {
                return Err(if selected.verdict == StoreVerdict::Withdrawn {
                    PackageError::Withdrawn
                } else {
                    PackageError::Incompatible
                });
            }
            if selected.target.requires.appsync && observation.appsync == RequirementState::Missing
            {
                return Err(PackageError::NeedsAppSync);
            }
            Some(selected)
        };
        let bundle = if let Some(selected) = selection {
            selected.artifact.native_identity.bundle_id.clone()
        } else if let Some(bundle) = request.bundle_id {
            if !bundle_ids.contains(&bundle) {
                return Err(PackageError::InvalidAction);
            }
            bundle
        } else {
            let installed: Vec<_> = observation
                .applications
                .iter()
                .filter(|a| bundle_ids.contains(&a.bundle_id))
                .collect();
            if installed.len() != 1 {
                return Err(PackageError::InvalidAction);
            }
            installed[0].bundle_id.clone()
        };
        let previous = observation
            .applications
            .iter()
            .find(|a| a.bundle_id == bundle)
            .cloned();
        validate_action(
            request.action,
            previous.as_ref(),
            selection.map(|s| s.artifact),
        )?;
        if request.action == PackageAction::Update {
            let previous = previous.as_ref().ok_or(PackageError::InvalidAction)?;
            let known = catalog
                .catalog()
                .releases
                .iter()
                .filter(|r| r.app_id == app.id)
                .flat_map(|r| &r.artifacts)
                .any(|a| {
                    a.native_identity.bundle_id == previous.bundle_id
                        && previous.product_version.as_deref() == Some(&a.native_identity.version)
                        && previous.build_number.as_deref() == Some(&a.native_identity.build_number)
                        && previous.receipt_build_id.as_deref() == Some(&a.build_id)
                });
            if !known {
                return Err(PackageError::InvalidAction);
            }
        }

        let plan = PackagePlan {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: request.device_id,
            device_name: observation.device.marketing_name.clone(),
            app_id: app.id.clone(),
            names: app
                .locales
                .iter()
                .map(|(language, text)| (language.clone(), text.name.clone()))
                .collect::<BTreeMap<_, _>>(),
            action: request.action,
            bundle_id: bundle,
            previous,
            release_id: selection.map(|s| s.release.id.clone()),
            artifact: selection.map(|s| s.artifact.clone()),
            target: selection.map(|s| s.target.clone()),
            version: selection.map(|s| s.release.version.clone()),
            revision: selection.map(|s| s.release.revision),
            publication_id: catalog.catalog().publication_id.clone(),
            sequence: catalog.catalog().sequence,
            catalog_expires_at: catalog.catalog().expires_at,
            expires_at: now_millis() + 15 * 60 * 1000,
            appsync: observation.appsync,
            jailbreak: match observation.facts.jailbroken {
                Some(true) => RequirementState::Satisfied,
                Some(false) => RequirementState::Missing,
                None => RequirementState::Unknown,
            },
            steps: package_steps(request.action),
        };
        let mut state = self.state.lock().map_err(|_| PackageError::Storage)?;
        state
            .plans
            .retain(|_, pending| pending.plan.expires_at > now_millis());
        if state.plans.len() >= 64 {
            return Err(PackageError::Busy);
        }
        state.plans.insert(
            plan.id.clone(),
            PendingPlan {
                plan: plan.clone(),
                repository_id: catalog.catalog().repository_id.clone(),
                target,
            },
        );
        Ok(plan)
    }
    pub fn start(
        self: &Arc<Self>,
        consent: PackageConsent,
    ) -> Result<OperationHandle, PackageError> {
        let (handle, start_worker) = {
            let mut state = self.state.lock().map_err(|_| PackageError::Storage)?;
            let pending = state
                .plans
                .get(&consent.plan_id)
                .ok_or(PackageError::PlanExpired)?;
            if pending.plan.expires_at <= now_millis() {
                return Err(PackageError::PlanExpired);
            }
            if consent.delete_data != (pending.plan.action == PackageAction::Uninstall) {
                return Err(PackageError::InvalidConsent);
            }
            if state.jobs.values().any(|stored| {
                stored.binding == pending.target.binding()
                    && stored.job.plan.bundle_id == pending.plan.bundle_id
                    && stored.job.phase.active()
            }) {
                return Err(PackageError::Busy);
            }
            let handle = OperationHandle {
                operation_id: uuid::Uuid::new_v4().to_string(),
                kind: if pending.plan.action == PackageAction::Uninstall {
                    OperationKind::Uninstall
                } else {
                    OperationKind::Install
                },
                steps: pending.plan.steps.clone(),
                subject: Some(pending.plan.app_id.clone()),
            };
            let stored = StoredPackageJob {
                binding: pending.target.binding().into(),
                repository_id: pending.repository_id.clone(),
                job: PackageJob {
                    queue_order: state
                        .next_order
                        .checked_add(1)
                        .ok_or(PackageError::Storage)?,
                    handle: handle.clone(),
                    plan: pending.plan.clone(),
                    phase: PackagePhase::Queued,
                    step: StepId::Resolve,
                    completed_steps: vec![],
                    percent: 0,
                    submitted: false,
                    cleanup_complete: None,
                    staging_path: None,
                    error: None,
                    updated_at: now_millis(),
                },
            };
            self.persistence.save_job(&stored)?;
            state.next_order = stored.job.queue_order;
            state.jobs.insert(handle.operation_id.clone(), stored);
            let cancellation = Arc::new(StoreCancellation::default());
            state
                .cancellations
                .insert(handle.operation_id.clone(), cancellation.clone());
            let pending = state
                .plans
                .remove(&consent.plan_id)
                .expect("validated plan");
            state.queue.push_back(QueuedRun {
                id: handle.operation_id.clone(),
                pending,
                cancellation,
            });
            let start_worker = !state.worker_running;
            state.worker_running = true;
            (handle, start_worker)
        };
        self.log.record(
            LogLevel::Info,
            LogSource::Store,
            "log.store.authorized",
            "User authorized a device-bound application operation",
            None,
            Some(&handle.operation_id),
        );
        if start_worker {
            let service = self.clone();
            tokio::spawn(async move {
                service.drain_queue().await;
            });
        }
        Ok(handle)
    }
    async fn drain_queue(self: Arc<Self>) {
        loop {
            let next = {
                let mut state = self.state.lock().expect("package state poisoned");
                match state.queue.pop_front() {
                    Some(next) => Some(next),
                    None => {
                        state.worker_running = false;
                        None
                    }
                }
            };
            let Some(next) = next else {
                return;
            };
            self.clone()
                .run(next.id, next.pending, next.cancellation)
                .await;
        }
    }
    pub fn jobs(&self) -> Vec<PackageJob> {
        let state = self.state.lock().expect("package state poisoned");
        let mut jobs: Vec<_> = state.jobs.values().map(|s| s.job.clone()).collect();
        jobs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        jobs
    }
    pub fn busy(&self) -> bool {
        self.state
            .lock()
            .expect("package state poisoned")
            .jobs
            .values()
            .any(|s| s.job.phase.active())
    }
    pub fn warn_before_close(&self) -> bool {
        let busy = self.busy();
        if busy {
            self.log.record(
                LogLevel::Warn,
                LogSource::Store,
                "log.store.closeBlocked",
                "Wait for application operations or cancel them before closing",
                None,
                None,
            );
        }
        busy
    }
    pub fn cancel(&self, id: &str) -> Result<(), PackageError> {
        let queued = {
            let mut state = self.state.lock().map_err(|_| PackageError::Storage)?;
            let stored = state
                .jobs
                .get(id)
                .cloned()
                .ok_or(PackageError::UnknownOperation)?;
            if stored.job.submitted && stored.job.phase.active() {
                return Err(PackageError::NotCancellable);
            }
            if !stored.job.phase.active() {
                return Err(PackageError::UnknownOperation);
            }
            let cancellation = state
                .cancellations
                .get(id)
                .cloned()
                .ok_or(PackageError::UnknownOperation)?;
            let queued = stored.job.phase == PackagePhase::Queued;
            if queued {
                let mut stored = stored;
                stored.job.phase = PackagePhase::Cancelled;
                stored.job.error = Some(PackageError::Cancelled.operation_error());
                stored.job.updated_at = now_millis();
                self.persistence.save_job(&stored)?;
                state.jobs.insert(id.into(), stored);
                state.queue.retain(|run| run.id != id);
                state.cancellations.remove(id);
            }
            cancellation.cancel();
            queued
        };
        if queued {
            self.sink.operation(OperationEvent::Cancelled {
                operation_id: id.into(),
                step_id: StepId::Resolve,
            });
        }
        Ok(())
    }
    fn update(&self, id: &str, mutate: impl FnOnce(&mut PackageJob)) -> Result<(), PackageError> {
        let mut state = self.state.lock().map_err(|_| PackageError::Storage)?;
        let stored = state.jobs.get(id).ok_or(PackageError::UnknownOperation)?;
        let mut next = stored.clone();
        mutate(&mut next.job);
        next.job.updated_at = now_millis();
        self.persistence.save_job(&next)?;
        state.jobs.insert(id.into(), next);
        Ok(())
    }
    fn step(&self, id: &str, step: StepId) -> Result<(), PackageError> {
        let previous = {
            let mut state = self.state.lock().map_err(|_| PackageError::Storage)?;
            let mut stored = state
                .jobs
                .get(id)
                .cloned()
                .ok_or(PackageError::UnknownOperation)?;
            let job = &mut stored.job;
            if job.phase == PackagePhase::Cancelled {
                return Err(PackageError::Cancelled);
            }
            let previous = job.step;
            if previous != step && !job.completed_steps.contains(&previous) {
                job.completed_steps.push(previous);
            }
            job.step = step;
            job.percent = 0;
            job.phase = if step == StepId::VerifyInstall {
                PackagePhase::Verifying
            } else {
                PackagePhase::Running
            };
            job.updated_at = now_millis();
            self.persistence.save_job(&stored)?;
            state.jobs.insert(id.into(), stored);
            previous
        };
        if previous != step {
            self.sink.operation(OperationEvent::StepChanged {
                operation_id: id.into(),
                step_id: previous,
                status: StepStatus::Done,
            });
        }
        self.sink.operation(OperationEvent::StepChanged {
            operation_id: id.into(),
            step_id: step,
            status: StepStatus::Running,
        });
        Ok(())
    }
    fn progress(&self, id: &str, step: StepId, percent: u8) {
        if let Ok(mut state) = self.state.lock()
            && let Some(stored) = state.jobs.get_mut(id)
        {
            stored.job.percent = percent;
        }
        self.sink.operation(OperationEvent::Progress {
            operation_id: id.into(),
            step_id: step,
            percent,
        });
    }
    async fn run(
        self: Arc<Self>,
        id: String,
        pending: PendingPlan,
        cancellation: Arc<StoreCancellation>,
    ) {
        let result = self.execute(&id, &pending, cancellation).await;
        self.finish(&id, result);
    }
    async fn execute(
        self: &Arc<Self>,
        id: &str,
        pending: &PendingPlan,
        cancellation: Arc<StoreCancellation>,
    ) -> Result<(), PackageError> {
        let _permit = tokio::select! {permit=self.gate.clone().acquire_owned()=>permit.map_err(|_|PackageError::Busy)?,_=cancellation.wait()=>return Err(PackageError::Cancelled)};
        if cancellation.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        self.sink.operation(OperationEvent::Started {
            operation_id: id.to_owned(),
            kind: if pending.plan.action == PackageAction::Uninstall {
                OperationKind::Uninstall
            } else {
                OperationKind::Install
            },
        });
        self.step(id, StepId::Resolve)?;
        let mut valid_until = self.validate_pending(pending).await?;
        let path: Option<PathBuf> = if let Some(artifact) = &pending.plan.artifact {
            self.step(id, StepId::Download)?;
            let service = self.clone();
            let operation_id = id.to_owned();
            let control = DownloadControl {
                cancellation: cancellation.clone(),
                progress: Arc::new(move |bytes, total| {
                    service.progress(&operation_id, StepId::Download, percentage(bytes, total))
                }),
            };
            let file = self
                .catalog
                .download(&artifact.blob, control)
                .await
                .map_err(|error| match error {
                    super::store::StoreError::Cancelled => PackageError::Cancelled,
                    super::store::StoreError::ChecksumMismatch => PackageError::ChecksumMismatch,
                    _ => PackageError::DownloadFailed,
                })?;
            self.step(id, StepId::Verify)?;
            if file.sha256() != artifact.blob.sha256 {
                return Err(PackageError::ChecksumMismatch);
            }
            // Refresh again after downloading. A pinned artifact cannot be
            // silently replaced, revived after withdrawal, or installed stale.
            valid_until = self.validate_pending(pending).await?;
            Some(file.path().to_owned())
        } else {
            None
        };
        if cancellation.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        if pending.plan.action != PackageAction::Uninstall {
            self.step(id, StepId::Transfer)?;
        }
        let observer = Arc::new(Observer {
            service: self.clone(),
            id: id.to_owned(),
            valid_until,
            requires_catalog: pending.plan.action != PackageAction::Uninstall,
            cancellation: cancellation.clone(),
        });
        let result = pending
            .target
            .write(&pending.plan, path.as_deref(), cancellation, observer)
            .await;
        let submitted = self
            .state
            .lock()
            .map_err(|_| PackageError::Storage)?
            .jobs
            .get(id)
            .ok_or(PackageError::UnknownOperation)?
            .job
            .submitted;
        match result {
            Ok(()) => {}
            Err(error)
                if !submitted
                    || matches!(
                        error,
                        PackageError::WriteRejected
                            | PackageError::SignatureRejected
                            | PackageError::DeviceStorageFull
                    ) =>
            {
                return Err(error);
            }
            // An uncertain submission is observed, never replayed.
            Err(_) => {}
        }
        self.step(id, StepId::VerifyInstall)?;
        verify_target(pending.target.as_ref(), &pending.plan).await
    }
    async fn validate_pending(&self, pending: &PendingPlan) -> Result<u64, PackageError> {
        let observation = pending
            .target
            .inspect(std::slice::from_ref(&pending.plan.bundle_id))
            .await?;
        let actual = observation
            .applications
            .iter()
            .find(|a| a.bundle_id == pending.plan.bundle_id);
        if !same_installation(pending.plan.previous.as_ref(), actual) {
            return Err(PackageError::StateChanged);
        }
        if pending.plan.action == PackageAction::Uninstall {
            return Ok(u64::MAX);
        }
        let catalog = self.catalog(true).await?;
        if catalog.catalog().repository_id != pending.repository_id {
            return Err(PackageError::CatalogUnavailable);
        }
        if catalog.expired_at(now_millis()) {
            return Err(PackageError::CatalogExpired);
        }
        let app = catalog
            .catalog()
            .apps
            .iter()
            .find(|a| a.id == pending.plan.app_id && a.listing == Listing::Listed)
            .ok_or(PackageError::Withdrawn)?;
        let release = catalog
            .catalog()
            .releases
            .iter()
            .find(|r| {
                Some(&r.id) == pending.plan.release_id.as_ref()
                    && r.app_id == app.id
                    && r.status == ReleaseStatus::Published
            })
            .ok_or(PackageError::Withdrawn)?;
        let artifact = pending
            .plan
            .artifact
            .as_ref()
            .ok_or(PackageError::InvalidAction)?;
        let current = release
            .artifacts
            .iter()
            .find(|a| a.id == artifact.id && *a == artifact)
            .ok_or(PackageError::Withdrawn)?;
        let target = pending
            .plan
            .target
            .as_ref()
            .ok_or(PackageError::InvalidAction)?;
        if evaluate_target(
            current,
            target,
            Some(&observation.device),
            observation.facts,
        ) != StoreVerdict::Compatible
        {
            return Err(PackageError::Incompatible);
        }
        if target.requires.appsync && observation.appsync == RequirementState::Missing {
            return Err(PackageError::NeedsAppSync);
        }
        validate_action(pending.plan.action, actual, Some(current))?;
        Ok(catalog.catalog().expires_at)
    }
    fn finish(&self, id: &str, result: Result<(), PackageError>) {
        if result == Err(PackageError::Cancelled)
            && self
                .state
                .lock()
                .expect("package state poisoned")
                .jobs
                .get(id)
                .is_some_and(|stored| stored.job.phase == PackagePhase::Cancelled)
        {
            return;
        }
        let phase = match result {
            Ok(()) => PackagePhase::Verified,
            Err(PackageError::Cancelled) => PackagePhase::Cancelled,
            Err(PackageError::VerificationUnavailable) => PackagePhase::Unverified,
            Err(_) => PackagePhase::Failed,
        };
        let saved = self.update(id, |job| {
            job.phase = phase;
            job.error = result.err().map(PackageError::operation_error);
            if result.is_ok() && !job.completed_steps.contains(&job.step) {
                job.completed_steps.push(job.step);
            }
        });
        // Persistence failure must still release the live queue and close guard.
        // The persisted submitting record is reconciled after the next startup.
        let step = {
            let mut state = self.state.lock().expect("package state poisoned");
            state.cancellations.remove(id);
            let job = &mut state.jobs.get_mut(id).expect("running job").job;
            if saved.is_err() {
                job.phase = PackagePhase::Unverified;
                job.error = Some(PackageError::Storage.operation_error());
            }
            job.step
        };
        let result = if saved.is_err() {
            Err(PackageError::Storage)
        } else {
            result
        };
        match result {
            Ok(()) => {
                self.sink.operation(OperationEvent::StepChanged {
                    operation_id: id.into(),
                    step_id: step,
                    status: StepStatus::Done,
                });
                self.sink.operation(OperationEvent::Finished {
                    operation_id: id.into(),
                });
            }
            Err(PackageError::Cancelled) => self.sink.operation(OperationEvent::Cancelled {
                operation_id: id.into(),
                step_id: step,
            }),
            Err(error) => self.sink.operation(OperationEvent::Failed {
                operation_id: id.into(),
                step_id: step,
                error: error.operation_error(),
            }),
        }
        self.log.record(
            if result.is_ok() {
                LogLevel::Info
            } else {
                LogLevel::Warn
            },
            LogSource::Store,
            "log.store.completed",
            match result {
                Ok(()) => "Application operation verified".into(),
                Err(error) => error.to_string(),
            },
            None,
            Some(id),
        );
    }
    pub fn verify(
        self: &Arc<Self>,
        id: &str,
        device_id: &str,
    ) -> Result<OperationHandle, PackageError> {
        let permit = self
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| PackageError::Busy)?;
        let stored = self
            .state
            .lock()
            .map_err(|_| PackageError::Storage)?
            .jobs
            .get(id)
            .cloned()
            .ok_or(PackageError::UnknownOperation)?;
        if stored.job.phase.active() || !stored.job.submitted {
            return Err(PackageError::InvalidAction);
        }
        let target = self.driver.target(device_id)?;
        if target.binding() != stored.binding {
            return Err(PackageError::DeviceChanged);
        }
        self.step(id, StepId::VerifyInstall)?;
        let service = self.clone();
        let id = id.to_owned();
        let handle = stored.job.handle.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let result = verify_target(target.as_ref(), &stored.job.plan).await;
            service.finish(&id, result);
        });
        Ok(handle)
    }
}
struct Observer {
    service: Arc<PackageService>,
    id: String,
    valid_until: u64,
    requires_catalog: bool,
    cancellation: Arc<StoreCancellation>,
}
impl PackageObserver for Observer {
    fn before_submit(&self) -> Result<(), PackageError> {
        // Shares the same mutex as cancel(), so acceptance and the durable
        // submission transition cannot both win.
        let mut state = self
            .service
            .state
            .lock()
            .map_err(|_| PackageError::Storage)?;
        if self.cancellation.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        if self.requires_catalog && now_millis() >= self.valid_until {
            return Err(PackageError::CatalogExpired);
        }
        let mut stored = state
            .jobs
            .get(&self.id)
            .cloned()
            .ok_or(PackageError::UnknownOperation)?;
        stored.job.submitted = true;
        stored.job.updated_at = now_millis();
        self.service.persistence.save_job(&stored)?;
        state.jobs.insert(self.id.clone(), stored);
        Ok(())
    }
    fn progress(&self, event: PackageProgress) {
        match event {
            PackageProgress::Staging { path } => {
                if let Ok(mut state) = self.service.state.lock()
                    && let Some(stored) = state.jobs.get_mut(&self.id)
                {
                    stored.job.staging_path = Some(path);
                    let _ = self.service.persistence.save_job(stored);
                }
            }
            PackageProgress::Transfer { bytes, total } => {
                self.service
                    .progress(&self.id, StepId::Transfer, percentage(bytes, total))
            }
            PackageProgress::Submitted => {
                let step = {
                    let state = self.service.state.lock().expect("package state poisoned");
                    if state.jobs[&self.id].job.plan.action == PackageAction::Uninstall {
                        StepId::Uninstall
                    } else {
                        StepId::Install
                    }
                };
                let _ = self.service.step(&self.id, step);
            }
            PackageProgress::Device {
                percent: Some(percent),
            } => {
                let step = self
                    .service
                    .state
                    .lock()
                    .expect("package state poisoned")
                    .jobs[&self.id]
                    .job
                    .step;
                self.service.progress(&self.id, step, percent);
            }
            PackageProgress::Cleanup { complete } => {
                let _ = self
                    .service
                    .update(&self.id, |job| job.cleanup_complete = Some(complete));
            }
            PackageProgress::Device { percent: None } => {}
        }
    }
}
fn percentage(bytes: u64, total: u64) -> u8 {
    if total == 0 {
        0
    } else {
        ((u128::from(bytes) * 100 / u128::from(total)).min(100)) as u8
    }
}
pub fn validate_action(
    action: PackageAction,
    previous: Option<&NativeApplication>,
    artifact: Option<&crate::domain::store::Artifact>,
) -> Result<(), PackageError> {
    if previous.is_some_and(|a| a.application_type.as_deref() != Some("User")) {
        return Err(PackageError::InvalidAction);
    }
    match (action, previous) {
        (PackageAction::Install, Some(_))
        | (PackageAction::Update | PackageAction::Reinstall | PackageAction::Uninstall, None) => {
            return Err(PackageError::InvalidAction);
        }
        _ => {}
    }
    if action == PackageAction::Uninstall {
        return Ok(());
    }
    let artifact = artifact.ok_or(PackageError::InvalidAction)?;
    if let Some(previous) = previous {
        let version = previous
            .product_version
            .as_deref()
            .and_then(|v| compare_native(v, &artifact.native_identity.version));
        let build = previous
            .build_number
            .as_deref()
            .and_then(|v| compare_native(v, &artifact.native_identity.build_number));
        if version.is_some_and(|v| v.is_gt()) || build.is_some_and(|v| v.is_gt()) {
            return Err(PackageError::Downgrade);
        }
        if action == PackageAction::Update
            && (previous.receipt_build_id.is_none()
                || !(version.is_some_and(|v| v.is_lt()) || build.is_some_and(|v| v.is_lt())))
        {
            return Err(PackageError::InvalidAction);
        }
    }
    Ok(())
}
async fn verify_target(target: &dyn PackageTarget, plan: &PackagePlan) -> Result<(), PackageError> {
    let observation = target
        .inspect(std::slice::from_ref(&plan.bundle_id))
        .await
        .map_err(|_| PackageError::VerificationUnavailable)?;
    let registered = observation
        .applications
        .iter()
        .find(|a| a.bundle_id == plan.bundle_id);
    if plan.action == PackageAction::Uninstall {
        return if registered.is_none() {
            Ok(())
        } else {
            Err(PackageError::VerificationFailed)
        };
    }
    let registered = registered.ok_or(PackageError::VerificationFailed)?;
    let artifact = plan.artifact.as_ref().ok_or(PackageError::InvalidAction)?;
    if registered.application_type.is_none()
        || registered.product_version.is_none()
        || registered.build_number.is_none()
    {
        return Err(PackageError::VerificationUnavailable);
    }
    if registered.application_type.as_deref() != Some("User")
        || registered.product_version.as_deref() != Some(&artifact.native_identity.version)
        || registered.build_number.as_deref() != Some(&artifact.native_identity.build_number)
    {
        return Err(PackageError::VerificationFailed);
    }
    let receipt = registered
        .receipt_build_id
        .as_deref()
        .ok_or(PackageError::VerificationUnavailable)?;
    if receipt != artifact.build_id {
        return Err(PackageError::VerificationFailed);
    }
    Ok(())
}

#[cfg(test)]
#[path = "package_tests.rs"]
mod tests;
