//! Simulated device and workflow driver. It emits the exact event stream a
//! real Legacy iOS Kit adapter will emit, on a compressed clock, without
//! touching any hardware. `src/shared/gateway/simulatedGateway.ts` mirrors it
//! for browser-only development.

mod fixtures;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use tokio::sync::Notify;

use crate::application::{
    DeviceInventory, EventSink, OperationLog, PackageCatalog, WorkflowRunner, params,
};
use crate::domain::catalog::{CatalogEntry, InstalledPackage};
use crate::domain::device::{DeviceEvent, DeviceFacts, DeviceMode, DeviceSummary};
use crate::domain::log::{LogLevel, LogSource};
use crate::domain::now_millis;
use crate::domain::operation::{
    CancelError, OperationError, OperationErrorCode, OperationEvent, OperationHandle,
    OperationKind, PlanStep, RequiredAction, StepId, StepStatus,
};
use crate::domain::preparation::PreparationPlan;

/// One "estimated second" of a step takes this long in the demo.
const TIME_SCALE: Duration = Duration::from_millis(120);
const DFU_WAIT_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Default)]
struct DemoState {
    device: Option<DeviceSummary>,
    jailbroken: bool,
    installed: Vec<InstalledPackage>,
    fail_next: Option<StepId>,
}

struct OperationControl {
    cancel_requested: AtomicBool,
    finished: AtomicBool,
    current_step: Mutex<Option<PlanStep>>,
}

pub struct DemoDriver {
    this: Weak<DemoDriver>,
    sink: Arc<dyn EventSink>,
    log: Arc<OperationLog>,
    state: Mutex<DemoState>,
    operations: Mutex<HashMap<String, Arc<OperationControl>>>,
    operation_sequence: AtomicU64,
    mode_changed: Notify,
}

impl DemoDriver {
    pub fn new(sink: Arc<dyn EventSink>, log: Arc<OperationLog>) -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            this: this.clone(),
            sink,
            log,
            state: Mutex::new(DemoState::default()),
            operations: Mutex::new(HashMap::new()),
            operation_sequence: AtomicU64::new(0),
            mode_changed: Notify::new(),
        })
    }

    fn state(&self) -> std::sync::MutexGuard<'_, DemoState> {
        self.state.lock().expect("demo state poisoned")
    }

    // --- demo controls -----------------------------------------------------

    pub fn attach_device(&self) {
        let device = fixtures::ipod_touch_4();
        {
            let mut state = self.state();
            if state.device.is_some() {
                return;
            }
            state.device = Some(device.clone());
        }
        self.log.record(
            LogLevel::Info,
            LogSource::Device,
            "log.device.attached",
            "device attached over USB",
            Some(params([("model", device.model_identifier.clone())])),
            None,
        );
        self.sink.device(DeviceEvent::Attached {
            device: Box::new(device),
        });
    }

    pub fn detach_device(&self) {
        let Some(device) = self.state().device.take() else {
            return;
        };
        self.mode_changed.notify_waiters();
        self.log.record(
            LogLevel::Warn,
            LogSource::Device,
            "log.device.detached",
            "device detached",
            None,
            None,
        );
        self.sink.device(DeviceEvent::Detached {
            device_id: device.id,
        });
    }

    pub fn set_device_mode(&self, mode: DeviceMode) {
        let device_id = {
            let mut state = self.state();
            let Some(device) = state.device.as_mut() else {
                return;
            };
            if device.mode == mode {
                return;
            }
            device.mode = mode;
            device.id.clone()
        };
        self.mode_changed.notify_waiters();
        self.log.record(
            LogLevel::Info,
            LogSource::Device,
            "log.device.modeChanged",
            format!("device mode is now {mode:?}"),
            Some(params([("mode", mode_name(mode).to_owned())])),
            None,
        );
        self.sink
            .device(DeviceEvent::ModeChanged { device_id, mode });
    }

    pub fn set_jailbroken(&self, jailbroken: bool) {
        self.state().jailbroken = jailbroken;
        self.log.record(
            LogLevel::Debug,
            LogSource::System,
            "log.demo.jailbrokenSet",
            format!("demo: jailbroken={jailbroken}"),
            Some(params([("value", jailbroken.to_string())])),
            None,
        );
    }

    pub fn fail_next_step(&self, step: Option<StepId>) {
        self.state().fail_next = step;
        self.log.record(
            LogLevel::Debug,
            LogSource::System,
            "log.demo.failureArmed",
            format!("demo: failure armed for {step:?}"),
            Some(params([("step", format!("{step:?}"))])),
            None,
        );
    }

    // --- simulated execution -----------------------------------------------

    fn new_operation(
        &self,
        kind: OperationKind,
        steps: Vec<PlanStep>,
        subject: Option<String>,
    ) -> (OperationHandle, Arc<OperationControl>) {
        let sequence = self.operation_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let handle = OperationHandle {
            operation_id: format!("op-{sequence:04}"),
            kind,
            steps,
            subject,
        };
        let control = Arc::new(OperationControl {
            cancel_requested: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            current_step: Mutex::new(None),
        });
        self.operations
            .lock()
            .expect("operations poisoned")
            .insert(handle.operation_id.clone(), control.clone());
        (handle, control)
    }

    fn spawn(&self, handle: OperationHandle, control: Arc<OperationControl>, source: LogSource) {
        let Some(driver) = self.this.upgrade() else {
            return;
        };
        tokio::spawn(async move {
            driver.execute(handle, control, source).await;
        });
    }

    async fn wait_for_dfu(&self, control: &OperationControl) -> bool {
        let deadline = tokio::time::Instant::now() + DFU_WAIT_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            let mode = self.state().device.as_ref().map(|device| device.mode);
            match mode {
                Some(DeviceMode::Dfu) => return true,
                None => return false,
                Some(_) if control.cancel_requested.load(Ordering::Relaxed) => return false,
                Some(_) => {}
            }
            let _ = tokio::time::timeout(Duration::from_millis(500), self.mode_changed.notified())
                .await;
        }
        false
    }

    fn emit_step(&self, operation_id: &str, step_id: StepId, status: StepStatus) {
        self.sink.operation(OperationEvent::StepChanged {
            operation_id: operation_id.to_owned(),
            step_id,
            status,
        });
    }

    fn fail(
        &self,
        control: &OperationControl,
        source: LogSource,
        operation_id: &str,
        step: &PlanStep,
        error: OperationError,
    ) {
        control.finished.store(true, Ordering::Relaxed);
        self.emit_step(operation_id, step.id, StepStatus::Failed);
        self.log.record(
            LogLevel::Error,
            source,
            &format!("log.{}.failed", source_kind(source)),
            format!("failed at {:?}: {:?}", step.id, error.code),
            Some(params([
                ("step", step_name(step.id).to_owned()),
                ("code", format!("{:?}", error.code)),
            ])),
            Some(operation_id),
        );
        self.sink.operation(OperationEvent::Failed {
            operation_id: operation_id.to_owned(),
            step_id: step.id,
            error,
        });
    }

    fn cancelled(
        &self,
        control: &OperationControl,
        source: LogSource,
        operation_id: &str,
        step: &PlanStep,
    ) {
        control.finished.store(true, Ordering::Relaxed);
        self.emit_step(operation_id, step.id, StepStatus::Cancelled);
        self.log.record(
            LogLevel::Warn,
            source,
            &format!("log.{}.cancelled", source_kind(source)),
            format!("cancelled at {:?}", step.id),
            Some(params([("step", step_name(step.id).to_owned())])),
            Some(operation_id),
        );
        self.sink.operation(OperationEvent::Cancelled {
            operation_id: operation_id.to_owned(),
            step_id: step.id,
        });
    }

    async fn execute(
        &self,
        handle: OperationHandle,
        control: Arc<OperationControl>,
        source: LogSource,
    ) {
        let operation_id = handle.operation_id.clone();
        let kind = handle.kind;
        self.sink.operation(OperationEvent::Started {
            operation_id: operation_id.clone(),
            kind,
        });
        self.log.record(
            LogLevel::Info,
            source,
            &format!("log.{}.started", source_kind(source)),
            format!("{kind:?} operation started"),
            None,
            Some(&operation_id),
        );

        for step in &handle.steps {
            if control.cancel_requested.load(Ordering::Relaxed) {
                self.cancelled(&control, source, &operation_id, step);
                return;
            }
            *control.current_step.lock().expect("step poisoned") = Some(step.clone());
            self.emit_step(&operation_id, step.id, StepStatus::Running);
            self.log.record(
                LogLevel::Info,
                source,
                "log.step.started",
                format!("step {:?} started", step.id),
                Some(params([("step", step_name(step.id).to_owned())])),
                Some(&operation_id),
            );

            if step.requires_action == Some(RequiredAction::EnterDfu) {
                self.sink.operation(OperationEvent::ActionRequired {
                    operation_id: operation_id.clone(),
                    step_id: step.id,
                    action: RequiredAction::EnterDfu,
                });
                self.log.record(
                    LogLevel::Info,
                    source,
                    "log.step.actionRequired",
                    "waiting for the device to enter DFU mode",
                    Some(params([("action", "enterDfu".to_owned())])),
                    Some(&operation_id),
                );
                let entered = self.wait_for_dfu(&control).await;
                if control.cancel_requested.load(Ordering::Relaxed) {
                    self.cancelled(&control, source, &operation_id, step);
                    return;
                }
                if !entered {
                    let error = if self.state().device.is_some() {
                        failure_for(step.id)
                    } else {
                        disconnected(kind)
                    };
                    self.fail(&control, source, &operation_id, step, error);
                    return;
                }
                self.sink.operation(OperationEvent::ActionResolved {
                    operation_id: operation_id.clone(),
                    step_id: step.id,
                });
            }

            let ticks = step.estimated_seconds.clamp(4, 20);
            let tick = TIME_SCALE * step.estimated_seconds / ticks;
            let fail_at = (self.state().fail_next == Some(step.id)).then_some(ticks / 2);

            for index in 1..=ticks {
                tokio::time::sleep(tick).await;
                if self.state().device.is_none() {
                    self.fail(&control, source, &operation_id, step, disconnected(kind));
                    return;
                }
                if control.cancel_requested.load(Ordering::Relaxed) && step.cancellable {
                    self.cancelled(&control, source, &operation_id, step);
                    return;
                }
                if fail_at == Some(index) {
                    self.state().fail_next = None;
                    self.fail(&control, source, &operation_id, step, failure_for(step.id));
                    return;
                }
                self.sink.operation(OperationEvent::Progress {
                    operation_id: operation_id.clone(),
                    step_id: step.id,
                    percent: (index * 100 / ticks) as u8,
                });
            }

            if step.id == StepId::RebootDevice {
                self.set_device_mode(DeviceMode::Normal);
            }
            self.emit_step(&operation_id, step.id, StepStatus::Done);
            self.log.record(
                LogLevel::Debug,
                source,
                "log.step.done",
                format!("step {:?} done", step.id),
                Some(params([("step", step_name(step.id).to_owned())])),
                Some(&operation_id),
            );
        }

        control.finished.store(true, Ordering::Relaxed);
        match kind {
            OperationKind::Preparation => self.state().jailbroken = true,
            OperationKind::Install => {
                if let Some(package_id) = handle.subject.clone() {
                    let version = fixtures::catalog()
                        .into_iter()
                        .find(|entry| entry.id == package_id)
                        .map(|entry| entry.version)
                        .unwrap_or_default();
                    let mut state = self.state();
                    state.installed.retain(|item| item.package_id != package_id);
                    state.installed.push(InstalledPackage {
                        package_id,
                        version,
                        installed_at: now_millis(),
                    });
                }
            }
        }
        self.sink.operation(OperationEvent::Finished {
            operation_id: operation_id.clone(),
        });
        self.log.record(
            LogLevel::Info,
            source,
            &format!("log.{}.finished", source_kind(source)),
            format!("{kind:?} finished successfully"),
            None,
            Some(&operation_id),
        );
    }
}

impl DeviceInventory for DemoDriver {
    fn list(&self) -> Vec<DeviceSummary> {
        self.state().device.iter().cloned().collect()
    }

    fn find(&self, device_id: &str) -> Option<DeviceSummary> {
        self.state()
            .device
            .clone()
            .filter(|device| device.id == device_id)
    }

    fn facts(&self, device_id: &str) -> Option<DeviceFacts> {
        let state = self.state();
        state
            .device
            .as_ref()
            .filter(|device| device.id == device_id)
            .map(|_| DeviceFacts {
                jailbroken: state.jailbroken,
                ssh_available: state.jailbroken,
                pairing_trusted: true,
            })
    }
}

impl WorkflowRunner for DemoDriver {
    fn run_preparation(&self, plan: &PreparationPlan) -> OperationHandle {
        let (handle, control) =
            self.new_operation(OperationKind::Preparation, plan.steps.clone(), None);
        self.spawn(handle.clone(), control, LogSource::Preparation);
        handle
    }

    fn run_install(&self, _device: &DeviceSummary, entry: &CatalogEntry) -> OperationHandle {
        let (handle, control) = self.new_operation(
            OperationKind::Install,
            fixtures::install_steps(),
            Some(entry.id.clone()),
        );
        self.spawn(handle.clone(), control, LogSource::Store);
        handle
    }

    fn cancel(&self, operation_id: &str) -> Result<(), CancelError> {
        let control = self
            .operations
            .lock()
            .expect("operations poisoned")
            .get(operation_id)
            .cloned()
            .ok_or(CancelError::UnknownOperation)?;
        if control.finished.load(Ordering::Relaxed) {
            return Err(CancelError::AlreadyFinished);
        }
        let cancellable = control
            .current_step
            .lock()
            .expect("step poisoned")
            .as_ref()
            .is_none_or(|step| step.cancellable);
        if !cancellable {
            return Err(CancelError::NotCancellable);
        }
        control.cancel_requested.store(true, Ordering::Relaxed);
        self.mode_changed.notify_waiters();
        Ok(())
    }
}

impl PackageCatalog for DemoDriver {
    fn entries(&self) -> Vec<CatalogEntry> {
        fixtures::catalog()
    }

    fn installed(&self, _device_id: &str) -> Vec<InstalledPackage> {
        self.state().installed.clone()
    }
}

fn disconnected(kind: OperationKind) -> OperationError {
    OperationError {
        code: OperationErrorCode::DeviceDisconnected,
        recoverable: true,
        retry_from_step_id: Some(match kind {
            OperationKind::Preparation => StepId::EnterDfu,
            OperationKind::Install => StepId::Resolve,
        }),
    }
}

/// Which error a step produces when it fails, and whether a retry is sensible.
fn failure_for(step: StepId) -> OperationError {
    use OperationErrorCode as Code;
    let (code, recoverable, retry) = match step {
        StepId::EnterDfu => (Code::DfuTimeout, true, Some(StepId::EnterDfu)),
        StepId::ExploitBootrom => (Code::ExploitFailed, true, Some(StepId::EnterDfu)),
        StepId::FetchResources => (Code::DownloadFailed, true, Some(StepId::FetchResources)),
        StepId::BuildRamdisk => (Code::BuildFailed, true, Some(StepId::FetchResources)),
        StepId::BootRamdisk => (Code::RamdiskBootFailed, true, Some(StepId::EnterDfu)),
        StepId::MountFilesystem => (Code::SshUnavailable, true, Some(StepId::EnterDfu)),
        StepId::InstallUntether => (Code::WriteFailed, false, None),
        StepId::RebootDevice => (
            Code::DeviceDisconnected,
            true,
            Some(StepId::VerifyJailbreak),
        ),
        StepId::VerifyJailbreak => (Code::VerificationFailed, true, Some(StepId::EnterDfu)),
        StepId::Resolve => (Code::InstallRejected, true, Some(StepId::Resolve)),
        StepId::Download => (Code::DownloadFailed, true, Some(StepId::Download)),
        StepId::Verify => (Code::ChecksumMismatch, false, None),
        StepId::Transfer => (Code::TransferFailed, true, Some(StepId::Transfer)),
        StepId::Install => (Code::InstallRejected, true, Some(StepId::Install)),
        StepId::VerifyInstall => (Code::VerificationFailed, true, Some(StepId::VerifyInstall)),
    };
    OperationError {
        code,
        recoverable,
        retry_from_step_id: retry,
    }
}

fn source_kind(source: LogSource) -> &'static str {
    match source {
        LogSource::Store => "install",
        _ => "preparation",
    }
}

fn mode_name(mode: DeviceMode) -> &'static str {
    match mode {
        DeviceMode::Normal => "normal",
        DeviceMode::Recovery => "recovery",
        DeviceMode::Dfu => "dfu",
    }
}

fn step_name(step: StepId) -> &'static str {
    match step {
        StepId::EnterDfu => "enterDfu",
        StepId::ExploitBootrom => "exploitBootrom",
        StepId::FetchResources => "fetchResources",
        StepId::BuildRamdisk => "buildRamdisk",
        StepId::BootRamdisk => "bootRamdisk",
        StepId::MountFilesystem => "mountFilesystem",
        StepId::InstallUntether => "installUntether",
        StepId::RebootDevice => "rebootDevice",
        StepId::VerifyJailbreak => "verifyJailbreak",
        StepId::Resolve => "resolve",
        StepId::Download => "download",
        StepId::Verify => "verify",
        StepId::Transfer => "transfer",
        StepId::Install => "install",
        StepId::VerifyInstall => "verifyInstall",
    }
}
