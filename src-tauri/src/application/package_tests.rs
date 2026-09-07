use super::*;
use crate::application::store::{
    CatalogOrigin, CatalogRead, StoreError, StoreFuture, VerifiedDownload,
};
use crate::{
    domain::{
        device::{DeviceEvent, DeviceFacts},
        log::LogEntry,
        store::{BlobRef, CATALOG_LIFETIME_MS, Catalog, CatalogPointer, Trust, sha256_hex},
    },
    infrastructure::store::StoreCache,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Notify;

#[derive(Deserialize)]
struct Fixture {
    catalog_utf8: String,
    pointer: CatalogPointer,
    trust: Trust,
}
fn signed(mut catalog: Catalog) -> Arc<VerifiedCatalog> {
    let f: Fixture =
        serde_json::from_str(include_str!("../../../contracts/store-v1/fixture-v1.json")).unwrap();
    catalog.created_at = now_millis();
    catalog.expires_at = catalog.created_at + CATALOG_LIFETIME_MS;
    let bytes = serde_json::to_vec(&catalog).unwrap();
    let mut pointer = f.pointer;
    pointer.catalog_sha256 = sha256_hex(&bytes);
    pointer.sequence = catalog.sequence;
    pointer.publication_id = catalog.publication_id.clone();
    pointer.signature = STANDARD.encode(SigningKey::from_bytes(&[1; 32]).sign(&bytes).to_bytes());
    Arc::new(VerifiedCatalog::verify(pointer, &bytes, &f.trust, 0).unwrap())
}
fn fixture() -> Arc<VerifiedCatalog> {
    let f: Fixture =
        serde_json::from_str(include_str!("../../../contracts/store-v1/fixture-v1.json")).unwrap();
    let mut catalog: Catalog = serde_json::from_str(&f.catalog_utf8).unwrap();
    let mut app = catalog.apps[0].clone();
    app.id = "dev.example.clock".into();
    app.slug = "clock".into();
    catalog.apps.push(app);
    let mut release = catalog.releases[0].clone();
    release.id = uuid::Uuid::new_v4().to_string();
    release.app_id = "dev.example.clock".into();
    release.artifacts[0].id = uuid::Uuid::new_v4().to_string();
    release.artifacts[0].native_identity.bundle_id = "dev.example.clock.ios".into();
    catalog.releases.push(release);
    signed(catalog)
}
struct Source {
    catalog: Mutex<Arc<VerifiedCatalog>>,
    withdraw_on_download: AtomicBool,
    path: PathBuf,
}
impl CatalogRepository for Source {
    fn read(&self, _: bool) -> StoreFuture<'_, CatalogRead> {
        Box::pin(async {
            Ok(CatalogRead {
                verified: Some(self.catalog.lock().unwrap().clone()),
                origin: CatalogOrigin::Cache,
                checked_at: Some(now_millis()),
                issue: Some("storeOffline".into()),
                source_label: Some("test".into()),
            })
        })
    }
    fn media<'a>(&'a self, _: &'a str) -> StoreFuture<'a, PathBuf> {
        Box::pin(async { Err(StoreError::ObjectNotFound) })
    }
    fn download<'a>(
        &'a self,
        blob: &'a BlobRef,
        control: DownloadControl,
    ) -> StoreFuture<'a, VerifiedDownload> {
        Box::pin(async move {
            if control.cancellation.is_cancelled() {
                return Err(StoreError::Cancelled);
            }
            if self.withdraw_on_download.load(Ordering::SeqCst) {
                let mut catalog = self.catalog.lock().unwrap();
                let mut next = catalog.catalog().clone();
                next.apps[0].listing = Listing::Unlisted;
                *catalog = signed(next);
            }
            Ok(VerifiedDownload::new(
                self.path.clone(),
                blob.sha256.clone(),
            ))
        })
    }
}
#[derive(Default)]
struct Sink(Mutex<Vec<OperationEvent>>);
impl EventSink for Sink {
    fn device(&self, _: DeviceEvent) {}
    fn log(&self, _: LogEntry) {}
    fn operation(&self, event: OperationEvent) {
        self.0.lock().unwrap().push(event);
    }
}
#[derive(Default)]
struct Device {
    apps: Mutex<Vec<NativeApplication>>,
    writes: AtomicUsize,
    block_transfer: AtomicBool,
    block_submitted: AtomicBool,
    transfer: Notify,
    submit: Notify,
    resume: Notify,
    unknown_receipt: AtomicBool,
    uncertain_response: AtomicBool,
    reject_signature: AtomicBool,
}
struct Driver(Arc<Device>);
struct Target {
    state: Arc<Device>,
    binding: String,
}
impl PackageDriver for Driver {
    fn target(&self, id: &str) -> Result<Arc<dyn PackageTarget>, PackageError> {
        Ok(Arc::new(Target {
            state: self.0.clone(),
            binding: id.into(),
        }))
    }
}
impl PackageTarget for Target {
    fn binding(&self) -> &str {
        &self.binding
    }
    fn inspect<'a>(&'a self, ids: &'a [String]) -> PackageFuture<'a, PackageObservation> {
        Box::pin(async move {
            let device=serde_json::from_value(serde_json::json!({"id":self.binding,"platform":"ios","marketingName":"Test device","modelIdentifier":"iPod4,1","osVersion":"6.1.6","buildNumber":"10B500","mode":"normal","transport":"usb"})).unwrap();
            Ok(PackageObservation {
                device,
                facts: DeviceFacts {
                    appsync_installed: Some(true),
                    pairing_trusted: Some(true),
                    jailbroken: None,
                    ssh_available: None,
                },
                appsync: RequirementState::Unknown,
                applications: self
                    .state
                    .apps
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|a| ids.contains(&a.bundle_id))
                    .cloned()
                    .collect(),
            })
        })
    }
    fn write<'a>(
        &'a self,
        plan: &'a PackagePlan,
        _: Option<&'a Path>,
        cancellation: Arc<StoreCancellation>,
        observer: Arc<dyn PackageObserver>,
    ) -> PackageFuture<'a, ()> {
        Box::pin(async move {
            self.state.transfer.notify_one();
            if self.state.block_transfer.load(Ordering::SeqCst) {
                tokio::select! {_=self.state.resume.notified()=>{},_=cancellation.wait()=>return Err(PackageError::Cancelled)}
            }
            observer.before_submit()?;
            observer.progress(PackageProgress::Submitted);
            self.state.writes.fetch_add(1, Ordering::SeqCst);
            self.state.submit.notify_one();
            if self.state.block_submitted.load(Ordering::SeqCst) {
                self.state.resume.notified().await;
            }
            if self.state.reject_signature.load(Ordering::SeqCst) {
                return Err(PackageError::SignatureRejected);
            }
            let mut apps = self.state.apps.lock().unwrap();
            apps.retain(|a| a.bundle_id != plan.bundle_id);
            if let Some(artifact) = &plan.artifact {
                apps.push(NativeApplication {
                    bundle_id: plan.bundle_id.clone(),
                    product_version: Some(artifact.native_identity.version.clone()),
                    build_number: Some(artifact.native_identity.build_number.clone()),
                    application_type: Some("User".into()),
                    receipt_build_id: (!self.state.unknown_receipt.load(Ordering::SeqCst))
                        .then(|| artifact.build_id.clone()),
                });
            }
            observer.progress(PackageProgress::Cleanup { complete: true });
            if self.state.uncertain_response.load(Ordering::SeqCst) {
                Err(PackageError::DeviceChanged)
            } else {
                Ok(())
            }
        })
    }
}
struct Persistence {
    cache: Arc<StoreCache>,
    reject_submission: AtomicBool,
}
impl PackagePersistence for Persistence {
    fn jobs(&self) -> Result<Vec<StoredPackageJob>, PackageError> {
        PackagePersistence::jobs(self.cache.as_ref())
    }
    fn save_job(&self, job: &StoredPackageJob) -> Result<(), PackageError> {
        if job.job.submitted && self.reject_submission.load(Ordering::SeqCst) {
            return Err(PackageError::Storage);
        }
        self.cache.save_job(job)
    }
}
struct Setup {
    service: Arc<PackageService>,
    device: Arc<Device>,
    source: Arc<Source>,
    persistence: Arc<Persistence>,
    gate: Arc<Semaphore>,
    _directory: tempfile::TempDir,
}
fn setup() -> Setup {
    let directory = tempfile::tempdir().unwrap();
    let cache = Arc::new(StoreCache::open(directory.path().into()).unwrap());
    let persistence = Arc::new(Persistence {
        cache,
        reject_submission: AtomicBool::new(false),
    });
    let source = Arc::new(Source {
        catalog: Mutex::new(fixture()),
        withdraw_on_download: AtomicBool::new(false),
        path: directory.path().join("package"),
    });
    let device = Arc::new(Device::default());
    let sink = Arc::new(Sink::default());
    let gate = Arc::new(Semaphore::new(1));
    let service = Arc::new(
        PackageService::new(
            Arc::new(Driver(device.clone())),
            source.clone(),
            persistence.clone(),
            sink.clone(),
            Arc::new(OperationLog::new(sink)),
            gate.clone(),
        )
        .unwrap(),
    );
    Setup {
        service,
        device,
        source,
        persistence,
        gate,
        _directory: directory,
    }
}
async fn make_plan(service: &PackageService, app: &str, action: PackageAction) -> PackagePlan {
    service
        .plan(PackageRequest {
            device_id: "device-one".into(),
            app_id: format!("dev.example.{app}"),
            action,
            bundle_id: None,
        })
        .await
        .unwrap()
}
fn start(service: &Arc<PackageService>, plan: &PackagePlan) -> OperationHandle {
    service
        .start(PackageConsent {
            plan_id: plan.id.clone(),
            delete_data: plan.action == PackageAction::Uninstall,
        })
        .unwrap()
}
async fn terminal(service: &PackageService, id: &str) -> PackageJob {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let job = service
                .jobs()
                .into_iter()
                .find(|job| job.handle.operation_id == id)
                .unwrap();
            if !job.phase.active() {
                return job;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn queue_respects_shared_device_gate_and_cancels_without_writing() {
    let s = setup();
    let permit = s.gate.clone().acquire_owned().await.unwrap();
    let first = make_plan(&s.service, "notes", PackageAction::Install).await;
    let second = make_plan(&s.service, "clock", PackageAction::Install).await;
    let one = start(&s.service, &first);
    let two = start(&s.service, &second);
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
    s.service.cancel(&two.operation_id).unwrap();
    assert_eq!(
        terminal(&s.service, &two.operation_id).await.phase,
        PackagePhase::Cancelled
    );
    drop(permit);
    assert_eq!(
        terminal(&s.service, &one.operation_id).await.phase,
        PackagePhase::Verified
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn submission_refuses_cancellation_and_never_replays_an_uncertain_response() {
    let s = setup();
    s.device.block_submitted.store(true, Ordering::SeqCst);
    s.device.uncertain_response.store(true, Ordering::SeqCst);
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &plan);
    s.device.submit.notified().await;
    assert_eq!(
        s.service.cancel(&handle.operation_id),
        Err(PackageError::NotCancellable)
    );
    s.device.resume.notify_one();
    assert_eq!(
        terminal(&s.service, &handle.operation_id).await.phase,
        PackagePhase::Verified
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn upload_cancellation_and_journal_failure_both_veto_submission() {
    let s = setup();
    s.device.block_transfer.store(true, Ordering::SeqCst);
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &plan);
    s.device.transfer.notified().await;
    s.service.cancel(&handle.operation_id).unwrap();
    assert_eq!(
        terminal(&s.service, &handle.operation_id).await.phase,
        PackagePhase::Cancelled
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
    s.device.block_transfer.store(false, Ordering::SeqCst);
    s.persistence
        .reject_submission
        .store(true, Ordering::SeqCst);
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &plan);
    assert_eq!(
        terminal(&s.service, &handle.operation_id).await.phase,
        PackagePhase::Failed
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn withdrawal_after_download_stops_the_pinned_artifact() {
    let s = setup();
    s.source.withdraw_on_download.store(true, Ordering::SeqCst);
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &plan);
    let job = terminal(&s.service, &handle.operation_id).await;
    assert_eq!(job.phase, PackagePhase::Failed);
    assert_eq!(
        job.error.unwrap().diagnostic.unwrap().reason,
        "packageWithdrawn"
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn unknown_receipt_requires_read_only_verification_and_uninstall_requires_data_consent() {
    let s = setup();
    s.device.unknown_receipt.store(true, Ordering::SeqCst);
    let install = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &install);
    assert_eq!(
        terminal(&s.service, &handle.operation_id).await.phase,
        PackagePhase::Unverified
    );
    assert_eq!(
        s.service
            .verify(&handle.operation_id, "device-two")
            .unwrap_err(),
        PackageError::DeviceChanged
    );
    s.device.apps.lock().unwrap()[0].receipt_build_id =
        install.artifact.as_ref().map(|a| a.build_id.clone());
    s.service
        .verify(&handle.operation_id, "device-one")
        .unwrap();
    assert_eq!(
        terminal(&s.service, &handle.operation_id).await.phase,
        PackagePhase::Verified
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 1);
    let uninstall = make_plan(&s.service, "notes", PackageAction::Uninstall).await;
    assert_eq!(
        s.service
            .start(PackageConsent {
                plan_id: uninstall.id.clone(),
                delete_data: false
            })
            .unwrap_err(),
        PackageError::InvalidConsent
    );
    let handle = start(&s.service, &uninstall);
    assert_eq!(
        terminal(&s.service, &handle.operation_id).await.phase,
        PackagePhase::Verified
    );
    assert!(s.device.apps.lock().unwrap().is_empty());
}
#[tokio::test]
async fn persisted_submissions_restart_as_observations_without_replaying_writes() {
    let s = setup();
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let job = PackageJob {
        queue_order: 1,
        handle: OperationHandle {
            operation_id: "interrupted".into(),
            kind: OperationKind::Install,
            steps: plan.steps.clone(),
            subject: Some(plan.app_id.clone()),
        },
        plan,
        phase: PackagePhase::Running,
        step: StepId::Install,
        completed_steps: vec![],
        percent: 0,
        submitted: true,
        cleanup_complete: None,
        staging_path: None,
        error: None,
        updated_at: 123,
    };
    s.persistence
        .save_job(&StoredPackageJob {
            binding: "device-one".into(),
            repository_id: s
                .source
                .catalog
                .lock()
                .unwrap()
                .catalog()
                .repository_id
                .clone(),
            job,
        })
        .unwrap();
    let sink = Arc::new(Sink::default());
    let restarted = PackageService::new(
        Arc::new(Driver(s.device.clone())),
        s.source.clone(),
        s.persistence.clone(),
        sink.clone(),
        Arc::new(OperationLog::new(sink)),
        s.gate.clone(),
    )
    .unwrap();
    assert_eq!(restarted.jobs()[0].phase, PackagePhase::Interrupted);
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn native_version_and_unknown_revision_never_trigger_automatic_downgrades() {
    let s = setup();
    s.device.apps.lock().unwrap().push(NativeApplication {
        bundle_id: "dev.example.notes.ios".into(),
        product_version: Some("2.0.0".into()),
        build_number: Some("10".into()),
        application_type: Some("User".into()),
        receipt_build_id: None,
    });
    let request = PackageRequest {
        device_id: "device-one".into(),
        app_id: "dev.example.notes".into(),
        action: PackageAction::Reinstall,
        bundle_id: None,
    };
    assert_eq!(
        s.service.plan(request.clone()).await.unwrap_err(),
        PackageError::Downgrade
    );
    {
        let mut apps = s.device.apps.lock().unwrap();
        apps[0].product_version = Some("1.0.0".into());
        apps[0].build_number = Some("1".into());
    }
    assert!(s.service.plan(request.clone()).await.is_ok());
    assert_eq!(
        s.service
            .plan(PackageRequest {
                action: PackageAction::Update,
                ..request
            })
            .await
            .unwrap_err(),
        PackageError::InvalidAction
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn system_signature_rejection_keeps_its_diagnosis() {
    let s = setup();
    s.device.reject_signature.store(true, Ordering::SeqCst);
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &plan);
    let job = terminal(&s.service, &handle.operation_id).await;
    assert_eq!(job.phase, PackagePhase::Failed);
    assert_eq!(
        job.error.unwrap().diagnostic.unwrap().reason,
        "signatureRejected"
    );
}
#[tokio::test]
async fn a_changed_installation_is_not_silently_converted_to_an_update() {
    let s = setup();
    let permit = s.gate.clone().acquire_owned().await.unwrap();
    let plan = make_plan(&s.service, "notes", PackageAction::Install).await;
    let handle = start(&s.service, &plan);
    s.device.apps.lock().unwrap().push(NativeApplication {
        bundle_id: plan.bundle_id,
        product_version: Some("1.0.0".into()),
        build_number: Some("1".into()),
        application_type: Some("User".into()),
        receipt_build_id: None,
    });
    drop(permit);
    let job = terminal(&s.service, &handle.operation_id).await;
    assert_eq!(
        job.error.unwrap().diagnostic.unwrap().reason,
        "installedStateChanged"
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn system_and_unknown_application_types_have_actionable_errors() {
    let s = setup();
    s.device.apps.lock().unwrap().push(NativeApplication {
        bundle_id: "dev.example.notes.ios".into(),
        product_version: Some("1.0.0".into()),
        build_number: Some("1".into()),
        application_type: Some("System".into()),
        receipt_build_id: None,
    });
    let request = PackageRequest {
        device_id: "device-one".into(),
        app_id: "dev.example.notes".into(),
        action: PackageAction::Uninstall,
        bundle_id: None,
    };
    assert_eq!(
        s.service.plan(request.clone()).await.unwrap_err(),
        PackageError::SystemApplication
    );
    s.device.apps.lock().unwrap()[0].application_type = None;
    assert_eq!(
        s.service.plan(request).await.unwrap_err(),
        PackageError::ApplicationTypeUnknown
    );
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
}
