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
    release.artifacts[0].ios_identity_mut().unwrap().bundle_id = "dev.example.clock.ios".into();
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
    fn platform(&self) -> crate::domain::device::Platform {
        crate::domain::device::Platform::Ios
    }
    fn binding(&self) -> &str {
        &self.binding
    }
    fn inspect<'a>(&'a self, ids: &'a [String]) -> PackageFuture<'a, PackageObservation> {
        Box::pin(async move {
            let device=serde_json::from_value(serde_json::json!({"id":self.binding,"platform":"ios","marketingName":"Test device","modelIdentifier":"iPod4,1","osVersion":"6.1.6","buildNumber":"10B500","mode":"normal","transport":"usb"})).unwrap();
            Ok(PackageObservation {
                managed: vec![],
                device,
                facts: DeviceFacts {
                    cfw: None,
                    appsync_installed: Some(true),
                    appsync_last_observation: None,
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
            apps.retain(|a| Some(a.bundle_id.as_str()) != plan.bundle_id());
            if let Some(artifact) = &plan.artifact {
                apps.push(NativeApplication {
                    bundle_id: plan.bundle_id().unwrap().to_owned(),
                    product_version: Some(artifact.ios_identity().unwrap().version.clone()),
                    build_number: Some(artifact.ios_identity().unwrap().build_number.clone()),
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
            installation_id: None,
            delivery: None,
            format: None,
            delete_data: None,
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
        installation_id: None,
        delivery: None,
        format: None,
        delete_data: None,
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
        bundle_id: plan.bundle_id().unwrap().to_owned(),
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
        installation_id: None,
        delivery: None,
        format: None,
        delete_data: None,
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

fn three_ds_catalog() -> Arc<VerifiedCatalog> {
    let original = fixture();
    let mut catalog = original.catalog().clone();
    catalog.apps.truncate(1);
    catalog.releases.truncate(1);
    let first = &mut catalog.releases[0];
    let mut native = first.artifacts[0].clone();
    let target = &mut native.targets[0];
    target.platform = "3ds".into();
    target.arch = "armv6k".into();
    target.models = vec!["RED".into()];
    target.os.min = "11.0.0".into();
    target.os.max = None;
    target.os.builds.clear();
    target.requires = crate::domain::store::Requirements::ThreeDs { cfw: true };
    target.host_abi = Some(8);
    target.installer_id = "3ds-cia".into();
    target.runtime_deliveries = vec![crate::domain::store::RuntimeDelivery::Bundled];
    target.runtime_requirement = None;
    target.runtime_provides = Some(crate::domain::store::RuntimeProvides {
        id: "pocketjs-3ds".into(),
        version: "0.11.0".into(),
        capabilities: vec![crate::domain::store::RuntimeCapability::GuestUpdate],
    });
    native.format = "cia".into();
    native.native_identity = Some(crate::domain::store::NativeIdentity::ThreeDsTitle {
        title_id: "000400000ff00000".into(),
        title_version: 1,
    });
    let mut guest = native.clone();
    guest.id = uuid::Uuid::new_v4().to_string();
    guest.format = "pocket".into();
    guest.native_identity = None;
    guest.blob.sha256 = "9".repeat(64);
    guest.build_id = "9".repeat(64);
    guest.targets[0].runtime_provides = None;
    guest.targets[0].runtime_requirement = Some(crate::domain::store::RuntimeRequirement {
        id: "pocketjs-3ds".into(),
        min_version: "0.11.0".into(),
        bootstrap_app_id: Some("dev.pocket-stack.launcher".into()),
    });
    guest.targets[0].installer_id = "pocket-runtime".into();
    guest.targets[0].runtime_deliveries = vec![
        crate::domain::store::RuntimeDelivery::Shared,
        crate::domain::store::RuntimeDelivery::Bundled,
    ];
    first.artifacts = vec![native, guest];
    signed(catalog)
}
fn three_ds_observation() -> PackageObservation {
    let device=serde_json::from_value(serde_json::json!({"id":"3ds-session","platform":"3ds","marketingName":"New 3DS LL","modelIdentifier":"RED","osVersion":"11.17.0","mode":"normal","transport":"network","threeDs":{"region":"JPN","firmwareRevision":50,"firmware":"11.17.0-50J","runtime":{"id":"pocketjs-3ds","version":"0.11.0","capabilities":["guest-update","app-library","cia-management","file-management"]},"hostAbi":8,"hostAppId":"dev.pocket-stack.launcher","launcher":true,"busy":false,"nativeManagement":true,"hardwareVerified":false}})).unwrap();
    PackageObservation {
        device,
        facts: DeviceFacts {
            cfw: Some(true),
            pairing_trusted: Some(true),
            ..Default::default()
        },
        appsync: RequirementState::Unknown,
        applications: vec![],
        managed: vec![],
    }
}
#[test]
fn three_ds_forms_require_explicit_selection_and_keep_distinct_installations() {
    use crate::domain::store::RuntimeDelivery;
    let catalog = three_ds_catalog();
    let observed = three_ds_observation();
    let mut request = PackageRequest {
        device_id: observed.device.id.clone(),
        app_id: catalog.catalog().apps[0].id.clone(),
        action: PackageAction::Install,
        installation_id: None,
        delivery: None,
        format: None,
        delete_data: None,
    };
    assert!(matches!(
        super::super::three_ds_packages::plan(&catalog, &request, &observed),
        Err(PackageError::InvalidAction)
    ));
    request.delivery = Some(RuntimeDelivery::Shared);
    let shared = super::super::three_ds_packages::plan(&catalog, &request, &observed).unwrap();
    assert_eq!(shared.artifact.as_ref().unwrap().format, "pocket");
    request.delivery = Some(RuntimeDelivery::Bundled);
    let bundled = super::super::three_ds_packages::plan(&catalog, &request, &observed).unwrap();
    assert_eq!(bundled.artifact.as_ref().unwrap().format, "cia");
    assert_ne!(shared.installation_key(), bundled.installation_key());
    request.format = Some("pocket".into());
    assert!(super::super::three_ds_packages::plan(&catalog, &request, &observed).is_err());
}
#[test]
fn three_ds_guest_updates_bind_to_the_existing_host_and_reject_changed_generations() {
    use crate::domain::{store::RuntimeDelivery, three_ds::*};
    let catalog = three_ds_catalog();
    let mut observed = three_ds_observation();
    let release = &catalog.catalog().releases[0];
    let app = &release.app_id;
    let id = installation_id("cia-000400000ff00000", app);
    let mut prior:ThreeDsInstallation=serde_json::from_value(serde_json::json!({"installationId":id,"appId":app,"containerId":"cia-000400000ff00000","generation":4,"format":"cia","delivery":"bundled","installed":true,"health":"accepted","title":"Notes","version":"0.0.1","revision":1,"buildId":"old","guestSha256":"a".repeat(64),"nativeVersion":"0.0.1","nativeBuildId":"old-native","nativeIdentity":release.artifacts[0].native_identity,"runtimeId":"pocketjs-3ds","runtimeVersion":"0.11.0","hostAbi":8})).unwrap();
    observed.managed.push(prior.clone());
    let request = PackageRequest {
        device_id: observed.device.id.clone(),
        app_id: app.clone(),
        action: PackageAction::Update,
        installation_id: Some(id.clone()),
        delivery: Some(RuntimeDelivery::Shared),
        format: None,
        delete_data: None,
    };
    let plan = super::super::three_ds_packages::plan(&catalog, &request, &observed).unwrap();
    assert_eq!(plan.managed().unwrap().delivery, RuntimeDelivery::Bundled);
    assert_eq!(plan.artifact.as_ref().unwrap().format, "pocket");
    assert!(!plan.managed().unwrap().updates_host);
    prior.generation += 1;
    observed.managed[0] = prior;
    assert!(matches!(
        super::super::three_ds_packages::validate_state(&plan, &observed),
        Err(PackageError::StateChanged)
    ));
    let remove = PackageRequest {
        action: PackageAction::Uninstall,
        ..request
    };
    let plan = super::super::three_ds_packages::plan(&catalog, &remove, &observed).unwrap();
    assert!(!plan.delete_data);
    assert_eq!(plan.installation_key(), id);
}

#[test]
fn three_ds_hosts_never_manage_themselves_and_standalone_runtimes_only_update_their_guest() {
    use crate::domain::{store::RuntimeDelivery, three_ds::*};
    let catalog = three_ds_catalog();
    let release = &catalog.catalog().releases[0];
    let app = release.app_id.clone();
    let id = installation_id("cia-000400000ff00000", &app);
    let prior: ThreeDsInstallation = serde_json::from_value(serde_json::json!({"installationId":id,"appId":app,"containerId":"cia-000400000ff00000","generation":4,"format":"cia","delivery":"bundled","installed":true,"health":"accepted","title":"Notes","version":"0.0.1","revision":1,"buildId":"old","guestSha256":"a".repeat(64),"nativeVersion":"0.0.1","nativeBuildId":"old-native","nativeIdentity":release.artifacts[0].native_identity,"runtimeId":"pocketjs-3ds","runtimeVersion":"0.11.0","hostAbi":8})).unwrap();
    let mut launcher = three_ds_observation();
    launcher.managed.push(prior.clone());
    let request = PackageRequest {
        device_id: launcher.device.id.clone(),
        app_id: app.clone(),
        action: PackageAction::Uninstall,
        installation_id: Some(id.clone()),
        delivery: None,
        format: None,
        delete_data: None,
    };
    // The launcher may remove the app, but never itself, whatever it is called.
    let plan = super::super::three_ds_packages::plan(&catalog, &request, &launcher).unwrap();
    let mut hosted_by_itself = launcher.clone();
    hosted_by_itself
        .device
        .three_ds
        .as_mut()
        .unwrap()
        .host_app_id = app.clone();
    assert!(matches!(
        super::super::three_ds_packages::plan(&catalog, &request, &hosted_by_itself),
        Err(PackageError::RuntimeRequired)
    ));
    assert!(matches!(
        super::super::three_ds_packages::validate_state(&plan, &hosted_by_itself),
        Err(PackageError::RuntimeRequired)
    ));
    // A standalone runtime updates its own guest and nothing else.
    let mut standalone = hosted_by_itself.clone();
    standalone.device.three_ds.as_mut().unwrap().launcher = false;
    assert!(matches!(
        super::super::three_ds_packages::plan(&catalog, &request, &standalone),
        Err(PackageError::RuntimeRequired)
    ));
    let update = PackageRequest {
        action: PackageAction::Update,
        format: Some("pocket".into()),
        ..request.clone()
    };
    let guest = super::super::three_ds_packages::plan(&catalog, &update, &standalone).unwrap();
    assert_eq!(guest.artifact.as_ref().unwrap().format, "pocket");
    assert!(!guest.managed().unwrap().updates_host);
    // Asking for the native format replaces the host; another format is refused.
    let native = PackageRequest {
        format: Some("cia".into()),
        ..update.clone()
    };
    assert!(
        super::super::three_ds_packages::plan(&catalog, &native, &launcher)
            .unwrap()
            .managed()
            .unwrap()
            .updates_host
    );
    let switched = PackageRequest {
        format: Some("3dsx".into()),
        ..update.clone()
    };
    assert!(matches!(
        super::super::three_ds_packages::plan(&catalog, &switched, &launcher),
        Err(PackageError::InvalidAction)
    ));
    // Runtimes in the catalog are prepared, never installed as applications.
    let mut runtimes = catalog.catalog().clone();
    runtimes.apps[0].category = crate::domain::catalog::PackageCategory::Runtime;
    let install = PackageRequest {
        action: PackageAction::Install,
        installation_id: None,
        delivery: Some(RuntimeDelivery::Bundled),
        ..request.clone()
    };
    assert!(matches!(
        super::super::three_ds_packages::plan(&signed(runtimes), &install, &three_ds_observation()),
        Err(PackageError::RuntimeRequired)
    ));
    // Removal is verified against readable evidence only.
    let mut tombstone = prior.clone();
    tombstone.installed = false;
    tombstone.unavailable = true;
    let mut damaged = launcher.clone();
    damaged.managed = vec![tombstone];
    assert!(matches!(
        super::super::three_ds_packages::verify(&plan, &damaged),
        Err(PackageError::VerificationUnavailable)
    ));
    assert!(matches!(
        super::super::three_ds_packages::verify(&plan, &launcher),
        Err(PackageError::VerificationFailed)
    ));
    damaged.managed.clear();
    assert!(super::super::three_ds_packages::verify(&plan, &damaged).is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn a_failed_key_write_records_no_pairing() {
    use crate::application::device_setup::{
        SetupDestination, SetupObservation, SetupPort, SetupRequest,
    };
    use crate::infrastructure::three_ds::{
        ThreeDsBridge,
        provisioning::{KEY, NativeSetup, parse_token},
    };
    let root = tempfile::tempdir().unwrap();
    let card = root.path().join("card");
    std::fs::create_dir_all(card.join("Nintendo 3DS")).unwrap();
    std::fs::write(card.join("boot.firm"), b"fixture").unwrap();
    // The key directory exists but cannot be written to.
    use std::os::unix::fs::PermissionsExt;
    let runtime = card.join("pocketjs/runtime");
    std::fs::create_dir_all(&runtime).unwrap();
    std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o555)).unwrap();
    let pairings = root.path().join("pairings.json");
    let bridge = Arc::new(ThreeDsBridge::new(pairings.clone()).unwrap());
    let setup = NativeSetup(bridge.clone());
    let request = SetupRequest {
        runtime_requirement: None,
        host_abi: None,
        destination: SetupDestination::Sd {
            path: card.to_string_lossy().into(),
        },
        address: None,
        format: None,
    };
    let observed: SetupObservation = setup.inspect(&request, None).await.unwrap();
    assert!(observed.token.is_none());
    assert!(setup.write(&request, &observed, None).await.is_err());
    assert!(!pairings.exists());
    assert!(!card.join(KEY).exists());
    // Once the card accepts the key, the very same plan pairs normally.
    std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o755)).unwrap();
    let result = setup.write(&request, &observed, None).await.unwrap();
    assert!(pairings.exists());
    assert!(bridge.contains(&result.pairing_id));
    assert!(parse_token(&std::fs::read(card.join(KEY)).unwrap()).is_ok());
}

#[tokio::test]
async fn pairing_plan_does_not_write_until_confirmed_and_cannot_be_replayed() {
    use crate::application::device_setup::{SetupDestination, SetupRequest, SetupService};
    use crate::infrastructure::three_ds::{
        ThreeDsBridge,
        provisioning::{KEY, NativeSetup, SetupError},
    };
    let root = tempfile::tempdir().unwrap();
    let card = root.path().join("card");
    std::fs::create_dir_all(card.join("Nintendo 3DS")).unwrap();
    std::fs::write(card.join("boot.firm"), b"fixture").unwrap();
    let bridge = Arc::new(ThreeDsBridge::new(root.path().join("pairings.json")).unwrap());
    let source = Arc::new(Source {
        catalog: Mutex::new(fixture()),
        withdraw_on_download: AtomicBool::new(false),
        path: root.path().into(),
    });
    let sink = Arc::new(Sink::default());
    let service = SetupService::new(
        Arc::new(NativeSetup(bridge)),
        source,
        Arc::new(Semaphore::new(1)),
        Arc::new(OperationLog::new(sink)),
    );
    let request = SetupRequest {
        runtime_requirement: None,
        host_abi: None,
        destination: SetupDestination::Sd {
            path: card.to_string_lossy().into(),
        },
        address: None,
        format: None,
    };
    let plan = service.plan(request).await.unwrap();
    assert!(!card.join(KEY).exists());
    let json = serde_json::to_string(&plan).unwrap();
    assert!(!json.contains("token"));
    let result = service.execute(&plan.id).await.unwrap();
    assert!(result.files_verified);
    assert!(card.join(KEY).exists());
    assert!(matches!(
        service.execute(&plan.id).await,
        Err(SetupError::Plan)
    ));
}

#[tokio::test]
async fn bootstrap_plans_follow_the_requested_runtime_identity_abi_and_minimum_version() {
    use crate::application::device_setup::{
        SetupDestination, SetupError, SetupRequest, SetupService,
    };
    use crate::domain::store::{RuntimeCapability, RuntimeRequirement};
    use crate::infrastructure::three_ds::{ThreeDsBridge, provisioning::NativeSetup};
    let original = three_ds_catalog();
    let mut catalog = original.catalog().clone();
    catalog.apps[0].category = crate::domain::catalog::PackageCategory::Runtime;
    catalog.releases[0].artifacts[0].targets[0]
        .runtime_provides
        .as_mut()
        .unwrap()
        .capabilities
        .push(RuntimeCapability::AppLibrary);
    let bootstrap = catalog.apps[0].id.clone();
    let root = tempfile::tempdir().unwrap();
    let card = root.path().join("card");
    std::fs::create_dir_all(card.join("Nintendo 3DS")).unwrap();
    std::fs::write(card.join("boot.firm"), b"fixture").unwrap();
    let source = Arc::new(Source {
        catalog: Mutex::new(signed(catalog)),
        withdraw_on_download: AtomicBool::new(false),
        path: root.path().into(),
    });
    let bridge = Arc::new(ThreeDsBridge::new(root.path().join("pairings.json")).unwrap());
    let sink = Arc::new(Sink::default());
    let service = SetupService::new(
        Arc::new(NativeSetup(bridge)),
        source,
        Arc::new(Semaphore::new(1)),
        Arc::new(OperationLog::new(sink)),
    );
    let mut request = SetupRequest {
        runtime_requirement: Some(RuntimeRequirement {
            id: "pocketjs-3ds".into(),
            min_version: "0.12.0".into(),
            bootstrap_app_id: Some(bootstrap.clone()),
        }),
        host_abi: Some(8),
        destination: SetupDestination::Sd {
            path: card.to_string_lossy().into(),
        },
        address: None,
        format: Some("cia".into()),
    };
    assert!(matches!(
        service.plan(request.clone()).await,
        Err(SetupError::Launcher)
    ));
    request.runtime_requirement.as_mut().unwrap().min_version = "0.11.0".into();
    request.host_abi = Some(9);
    assert!(matches!(
        service.plan(request.clone()).await,
        Err(SetupError::Launcher)
    ));
    request.host_abi = Some(8);
    let plan = service.plan(request).await.unwrap();
    assert_eq!(plan.bootstrap_app_id, bootstrap);
    assert_eq!(plan.artifact.unwrap().format, "cia");
}

#[test]
fn guest_release_mapping_keeps_the_native_host_as_separate_evidence() {
    use crate::domain::{
        installed::{InstallationObservation, map_installed},
        three_ds::ThreeDsInstallation,
    };
    let catalog = three_ds_catalog();
    let release = &catalog.catalog().releases[0];
    let native = &release.artifacts[0];
    let guest = &release.artifacts[1];
    let mut record:ThreeDsInstallation=serde_json::from_value(serde_json::json!({"installationId":"instance","appId":release.app_id,"containerId":"cia-000400000ff00000","generation":4,"format":"cia","delivery":"bundled","installed":true,"health":"accepted","title":"Notes","version":release.version,"revision":release.revision,"buildId":native.build_id,"guestSha256":guest.blob.sha256,"nativeVersion":release.version,"nativeBuildId":native.build_id,"nativeIdentity":native.native_identity,"runtimeId":"pocketjs-3ds","runtimeVersion":"0.11.0","hostAbi":8})).unwrap();
    let mut observation = InstallationObservation {
        applications: vec![],
        managed: vec![record.clone()],
        observed_at: 0,
    };
    assert_eq!(
        map_installed(catalog.catalog(), &observation)[0]
            .artifact_id
            .as_deref(),
        Some(native.id.as_str())
    );
    record.native_identity = Some(crate::domain::store::NativeIdentity::ThreeDsTitle {
        title_id: "000400000ff00000".into(),
        title_version: 2,
    });
    record.native_build_id = Some("upgraded-host".into());
    assert!(record.matches_application_artifact(native));
    assert!(!record.matches_artifact(native));
    record.build_id = Some(guest.build_id.clone());
    observation.managed = vec![record];
    let mapped = map_installed(catalog.catalog(), &observation);
    assert_eq!(mapped[0].artifact_id.as_deref(), Some(guest.id.as_str()));
    assert_eq!(mapped[0].revision, Some(release.revision));
    assert_eq!(
        mapped[0]
            .managed
            .as_ref()
            .unwrap()
            .native_build_id
            .as_deref(),
        Some("upgraded-host")
    );
}

#[tokio::test]
async fn legacy_cache_startup_recovers_submissions_without_executing_the_old_plan() {
    let s = setup();
    let directory = tempfile::tempdir().unwrap();
    let db = rusqlite::Connection::open(directory.path().join("store.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE package_jobs(operation_id TEXT PRIMARY KEY,record_json TEXT NOT NULL,updated_at INTEGER NOT NULL);").unwrap();
    db.execute(
        "INSERT INTO package_jobs VALUES('old-operation',?,123)",
        [include_str!(
            "../infrastructure/store/fixtures/legacy-package-job.json"
        )],
    )
    .unwrap();
    drop(db);
    let cache = Arc::new(StoreCache::open(directory.path().into()).unwrap());
    let sink = Arc::new(Sink::default());
    let service = PackageService::new(
        Arc::new(Driver(s.device.clone())),
        s.source.clone(),
        cache,
        sink.clone(),
        Arc::new(OperationLog::new(sink)),
        s.gate.clone(),
    )
    .unwrap();
    let jobs = service.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].phase, PackagePhase::Interrupted);
    assert!(jobs[0].submitted);
    assert_eq!(jobs[0].plan.id, "old-plan");
    assert_eq!(jobs[0].plan.bundle_id(), Some("dev.example.notes.ios"));
    assert_eq!(s.device.writes.load(Ordering::SeqCst), 0);
    drop(service);
    let reopened = StoreCache::open(directory.path().into()).unwrap();
    assert_eq!(
        reopened.jobs().unwrap()[0].job.phase,
        PackagePhase::Interrupted
    );
}
