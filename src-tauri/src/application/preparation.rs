//! Plan-bound, single-use consent and a serialized native preparation runner.
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::{EventSink, OperationLog};
use crate::domain::{
    device::DeviceSummary,
    log::{LogLevel, LogSource},
    now_millis,
    operation::*,
    preparation::*,
};

pub type PreparationFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, PreparationError>> + Send + 'a>>;

/// Captures a physical identity privately; all subsequent mode changes must
/// match it. Creating a target and validating it are read-only.
pub trait PreparationTarget: Send + Sync {
    fn validate(&self) -> PreparationFuture<'_, ()>;
    fn execute(&mut self, step: StepId) -> PreparationFuture<'_, ()>;
}

pub trait PreparationDriver: Send + Sync {
    fn target(&self, device_id: &str) -> PreparationFuture<'_, Box<dyn PreparationTarget>>;
}

#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum PreparationError {
    #[error("device is no longer connected")]
    DeviceNotFound,
    #[error("this device or firmware is not supported for preparation")]
    Unsupported,
    #[error("the device is already jailbroken")]
    AlreadyJailbroken,
    #[error("pairing, normal mode and at least 50 percent battery are required")]
    DeviceNotReady,
    #[error("another operation is running")]
    Busy,
    #[error("the preparation plan has expired or was already used")]
    PlanExpired,
    #[error("consent is incomplete, invalid or expired")]
    InvalidConsent,
    #[error("the current step cannot be cancelled")]
    NotCancellable,
    #[error("operation was not found or has finished")]
    UnknownOperation,
    #[error("preparation failed: {0:?}")]
    Step(OperationErrorCode),
}

impl PreparationError {
    pub fn code(self) -> &'static str {
        match self {
            Self::DeviceNotFound => "deviceNotFound",
            Self::Unsupported => "preparationUnsupported",
            Self::AlreadyJailbroken => "alreadyJailbroken",
            Self::DeviceNotReady => "deviceNotReady",
            Self::Busy => "operationBusy",
            Self::PlanExpired => "planExpired",
            Self::InvalidConsent => "invalidConsent",
            Self::NotCancellable => "notCancellable",
            Self::UnknownOperation => "unknownOperation",
            Self::Step(_) => "preparationFailed",
        }
    }
}

struct PendingPlan {
    plan: PreparationPlan,
    issued_at: u64,
    target: Box<dyn PreparationTarget>,
}

struct ActiveRun {
    id: String,
    step: StepId,
    cancellable: bool,
    cancel: Arc<tokio::sync::Notify>,
}

#[derive(Default)]
struct State {
    plans: HashMap<String, PendingPlan>,
    active: Option<ActiveRun>,
}

pub struct PreparationService {
    driver: Arc<dyn PreparationDriver>,
    state: Mutex<State>,
    sink: Arc<dyn EventSink>,
    log: Arc<OperationLog>,
}

impl PreparationService {
    pub fn new(
        driver: Arc<dyn PreparationDriver>,
        sink: Arc<dyn EventSink>,
        log: Arc<OperationLog>,
    ) -> Self {
        Self {
            driver,
            state: Mutex::new(State::default()),
            sink,
            log,
        }
    }

    pub fn busy(&self) -> bool {
        self.state
            .lock()
            .expect("preparation state poisoned")
            .active
            .is_some()
    }

    pub fn warn_before_close(&self) -> bool {
        if !self.busy() {
            return false;
        }
        self.log.record(
            LogLevel::Warn,
            LogSource::Preparation,
            "log.preparation.closeBlocked",
            "Wait for preparation to finish or cancel at a safe step before closing",
            None,
            None,
        );
        true
    }

    pub async fn plan(&self, device: &DeviceSummary) -> Result<PreparationPlan, PreparationError> {
        if self.busy() {
            return Err(PreparationError::Busy);
        }
        let target = self.driver.target(&device.id).await?;
        let mut plan = PreparationPlan::ramdisk_jailbreak(
            uuid::Uuid::new_v4().to_string(),
            device.id.clone(),
            device
                .os_version
                .as_deref()
                .ok_or(PreparationError::Unsupported)?,
        );
        // Host work completes before asking the user to put the device in DFU.
        plan.steps = native_steps();
        let mut state = self.state.lock().expect("preparation state poisoned");
        if state.active.is_some() {
            return Err(PreparationError::Busy);
        }
        state.plans.retain(|_, p| {
            now_millis().saturating_sub(p.issued_at) < CONSENT_VALIDITY_MS
                && p.plan.device_id != device.id
        });
        if state.plans.len() >= 16 {
            state.plans.clear();
        }
        state.plans.insert(
            plan.id.clone(),
            PendingPlan {
                plan: plan.clone(),
                issued_at: now_millis(),
                target,
            },
        );
        Ok(plan)
    }

    pub async fn start(
        self: &Arc<Self>,
        consent: ConsentRecord,
    ) -> Result<OperationHandle, PreparationError> {
        let (pending, handle, cancel) = {
            let mut state = self.state.lock().expect("preparation state poisoned");
            if state.active.is_some() {
                return Err(PreparationError::Busy);
            }
            let pending = state
                .plans
                .get(&consent.plan_id)
                .ok_or(PreparationError::PlanExpired)?;
            validate_timed_consent(&pending.plan, &consent, pending.issued_at, now_millis())
                .map_err(|_| PreparationError::InvalidConsent)?;
            let pending = state
                .plans
                .remove(&consent.plan_id)
                .expect("validated plan");
            let handle = OperationHandle {
                operation_id: uuid::Uuid::new_v4().to_string(),
                kind: OperationKind::Preparation,
                steps: pending.plan.steps.clone(),
                subject: None,
            };
            let cancel = Arc::new(tokio::sync::Notify::new());
            state.active = Some(ActiveRun {
                id: handle.operation_id.clone(),
                step: StepId::FetchResources,
                cancellable: true,
                cancel: cancel.clone(),
            });
            (pending, handle, cancel)
        };
        // No device-changing action has occurred at this point. Validation is
        // performed inside the owned worker so an IPC disconnect cannot strand
        // the lock or interrupt an already-started write.
        self.log.record(
            LogLevel::Info,
            LogSource::Preparation,
            "log.preparation.nativeConsent",
            "User authorized the device-bound jailbreak plan",
            Some(super::params([
                ("plan", consent.plan_id),
                ("version", consent.disclaimer_version),
                ("riskSeconds", consent.risk_reading_seconds.to_string()),
                (
                    "risksAcknowledgedAt",
                    consent.risks_acknowledged_at.to_string(),
                ),
                (
                    "disclaimerAcceptedAt",
                    consent.disclaimer_accepted_at.to_string(),
                ),
                (
                    "prerequisites",
                    format!("{:?}", consent.prerequisites_confirmed),
                ),
                ("risks", format!("{:?}", consent.acknowledged_risk_ids)),
                (
                    "disclaimerSeconds",
                    consent.disclaimer_reading_seconds.to_string(),
                ),
            ])),
            Some(&handle.operation_id),
        );
        let service = self.clone();
        let worker_handle = handle.clone();
        tokio::spawn(async move {
            service.run(pending.target, worker_handle, cancel).await;
        });
        Ok(handle)
    }

    pub fn cancel(&self, id: &str) -> Result<(), PreparationError> {
        let state = self.state.lock().expect("preparation state poisoned");
        let active = state
            .active
            .as_ref()
            .filter(|run| run.id == id)
            .ok_or(PreparationError::UnknownOperation)?;
        if !active.cancellable {
            return Err(PreparationError::NotCancellable);
        }
        active.cancel.notify_one();
        Ok(())
    }

    async fn run(
        &self,
        mut target: Box<dyn PreparationTarget>,
        handle: OperationHandle,
        cancel: Arc<tokio::sync::Notify>,
    ) {
        let id = &handle.operation_id;
        self.sink.operation(OperationEvent::Started {
            operation_id: id.clone(),
            kind: handle.kind,
        });
        let result = self.execute_steps(target.as_mut(), &handle, &cancel).await;
        let step = {
            let mut state = self.state.lock().expect("preparation state poisoned");
            let run = state.active.take().expect("active preparation");
            run.step
        };
        match result {
            Ok(()) => {
                self.sink.operation(OperationEvent::Finished {
                    operation_id: id.clone(),
                });
                self.log.record(
                    LogLevel::Info,
                    LogSource::Preparation,
                    "log.preparation.nativeFinished",
                    "Jailbreak and SSH verified after reboot",
                    None,
                    Some(id),
                );
            }
            Err(PreparationError::Step(OperationErrorCode::Cancelled)) => {
                self.changed(id, step, StepStatus::Cancelled);
                self.sink.operation(OperationEvent::Cancelled {
                    operation_id: id.clone(),
                    step_id: step,
                });
                self.log.record(
                    LogLevel::Info,
                    LogSource::Preparation,
                    "log.preparation.nativeCancelled",
                    "Preparation cancelled before device modification",
                    None,
                    Some(id),
                );
            }
            Err(error) => {
                let code = match error {
                    PreparationError::Step(code) => code,
                    PreparationError::AlreadyJailbroken => OperationErrorCode::AlreadyJailbroken,
                    PreparationError::DeviceNotFound => OperationErrorCode::DeviceDisconnected,
                    _ => OperationErrorCode::DeviceChanged,
                };
                self.changed(id, step, StepStatus::Failed);
                self.sink.operation(OperationEvent::Failed {
                    operation_id: id.clone(),
                    step_id: step,
                    error: OperationError {
                        code,
                        recoverable: matches!(
                            step,
                            StepId::FetchResources | StepId::BuildRamdisk | StepId::EnterDfu
                        ),
                        retry_from_step_id: None,
                    },
                });
                self.log.record(
                    LogLevel::Error,
                    LogSource::Preparation,
                    "log.preparation.nativeFailed",
                    format!("Preparation stopped at {step:?}: {code:?}"),
                    None,
                    Some(id),
                );
            }
        }
    }

    async fn execute_steps(
        &self,
        target: &mut dyn PreparationTarget,
        handle: &OperationHandle,
        cancel: &tokio::sync::Notify,
    ) -> Result<(), PreparationError> {
        for (index, step) in handle.steps.iter().enumerate() {
            {
                let mut state = self.state.lock().expect("preparation state poisoned");
                let active = state.active.as_mut().expect("active preparation");
                active.step = step.id;
                active.cancellable = step.cancellable;
            }
            // A cancellation accepted at the preceding checkpoint always wins
            // over entering the first non-cancellable device operation.
            if tokio::time::timeout(std::time::Duration::ZERO, cancel.notified())
                .await
                .is_ok()
            {
                return Err(PreparationError::Step(OperationErrorCode::Cancelled));
            }
            self.changed(&handle.operation_id, step.id, StepStatus::Running);
            if let Some(action) = step.requires_action {
                self.sink.operation(OperationEvent::ActionRequired {
                    operation_id: handle.operation_id.clone(),
                    step_id: step.id,
                    action,
                });
            }
            let task = async {
                if index == 0 {
                    target.validate().await?;
                }
                target.execute(step.id).await
            };
            if step.cancellable {
                tokio::select! {
                    biased;
                    _ = cancel.notified() => return Err(PreparationError::Step(OperationErrorCode::Cancelled)),
                    result = task => result?,
                }
            } else {
                task.await?;
            }
            if step.requires_action.is_some() {
                self.sink.operation(OperationEvent::ActionResolved {
                    operation_id: handle.operation_id.clone(),
                    step_id: step.id,
                });
            }
            self.changed(&handle.operation_id, step.id, StepStatus::Done);
        }
        Ok(())
    }

    fn changed(&self, id: &str, step: StepId, status: StepStatus) {
        self.sink.operation(OperationEvent::StepChanged {
            operation_id: id.into(),
            step_id: step,
            status,
        });
        if status == StepStatus::Running {
            self.log.record(
                LogLevel::Info,
                LogSource::Preparation,
                "log.preparation.nativeStep",
                format!("Preparation step: {step:?}"),
                Some(super::params([("step", format!("{step:?}"))])),
                Some(id),
            );
        }
    }
}

pub fn native_steps() -> Vec<PlanStep> {
    [
        (StepId::FetchResources, true, false, 120),
        (StepId::BuildRamdisk, true, false, 30),
        (StepId::EnterDfu, true, false, 180),
        (StepId::ExploitBootrom, false, false, 15),
        (StepId::BootRamdisk, false, false, 45),
        (StepId::MountFilesystem, false, true, 30),
        (StepId::InstallUntether, false, true, 90),
        (StepId::RebootDevice, false, false, 90),
        (StepId::VerifyJailbreak, false, false, 60),
    ]
    .into_iter()
    .map(
        |(id, cancellable, point_of_no_return, estimated_seconds)| PlanStep {
            id,
            cancellable,
            point_of_no_return,
            estimated_seconds,
            requires_action: (id == StepId::EnterDfu).then_some(RequiredAction::EnterDfu),
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{device::DeviceEvent, log::LogEntry};

    #[derive(Default)]
    struct Sink {
        events: Mutex<Vec<OperationEvent>>,
        changed: tokio::sync::Notify,
    }
    impl EventSink for Sink {
        fn device(&self, _: DeviceEvent) {}
        fn log(&self, _: LogEntry) {}
        fn operation(&self, event: OperationEvent) {
            self.events.lock().unwrap().push(event);
            self.changed.notify_one();
        }
    }
    struct Driver {
        calls: Arc<Mutex<Vec<StepId>>>,
        fail: Option<StepId>,
        block: Option<StepId>,
        gate: Arc<tokio::sync::Notify>,
    }
    struct Target {
        calls: Arc<Mutex<Vec<StepId>>>,
        fail: Option<StepId>,
        block: Option<StepId>,
        gate: Arc<tokio::sync::Notify>,
    }
    impl PreparationDriver for Driver {
        fn target(&self, _: &str) -> PreparationFuture<'_, Box<dyn PreparationTarget>> {
            Box::pin(async {
                Ok(Box::new(Target {
                    calls: self.calls.clone(),
                    fail: self.fail,
                    block: self.block,
                    gate: self.gate.clone(),
                }) as Box<dyn PreparationTarget>)
            })
        }
    }
    impl PreparationTarget for Target {
        fn validate(&self) -> PreparationFuture<'_, ()> {
            Box::pin(async { Ok(()) })
        }
        fn execute(&mut self, step: StepId) -> PreparationFuture<'_, ()> {
            Box::pin(async move {
                self.calls.lock().unwrap().push(step);
                if self.fail == Some(step) {
                    return Err(PreparationError::Step(OperationErrorCode::WriteFailed));
                }
                if self.block == Some(step) {
                    self.gate.notified().await;
                }
                Ok(())
            })
        }
    }
    fn setup(
        fail: Option<StepId>,
        block: Option<StepId>,
    ) -> (Arc<PreparationService>, Arc<Sink>, Arc<Driver>) {
        let sink = Arc::new(Sink::default());
        let driver = Arc::new(Driver {
            calls: Arc::default(),
            fail,
            block,
            gate: Arc::default(),
        });
        let service = Arc::new(PreparationService::new(
            driver.clone(),
            sink.clone(),
            Arc::new(OperationLog::new(sink.clone())),
        ));
        (service, sink, driver)
    }
    async fn consent(service: &PreparationService) -> ConsentRecord {
        let device = serde_json::from_value(serde_json::json!({"id":"session", "platform":"ios", "marketingName":"iPod touch 4", "osVersion":"6.1.6", "mode":"normal", "transport":"usb"})).unwrap();
        let plan = service.plan(&device).await.unwrap();
        let now = now_millis();
        service
            .state
            .lock()
            .unwrap()
            .plans
            .get_mut(&plan.id)
            .unwrap()
            .issued_at = now - 36_000;
        ConsentRecord {
            plan_id: plan.id,
            prerequisites_confirmed: plan.prerequisites,
            acknowledged_risk_ids: plan.risks.iter().map(|risk| risk.id).collect(),
            risk_reading_seconds: 15,
            risks_acknowledged_at: now - 21_000,
            disclaimer_version: plan.disclaimer_version,
            disclaimer_reading_seconds: 20,
            disclaimer_accepted_at: now - 1_000,
        }
    }
    async fn wait(sink: &Sink, predicate: impl Fn(&OperationEvent) -> bool) {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let notified = sink.changed.notified();
                if sink.events.lock().unwrap().iter().any(&predicate) {
                    return;
                }
                notified.await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn planning_and_incomplete_consent_never_execute_steps() {
        let (service, _, driver) = setup(None, None);
        let mut consent = consent(&service).await;
        assert!(driver.calls.lock().unwrap().is_empty());
        consent.acknowledged_risk_ids.clear();
        assert_eq!(
            service.start(consent).await,
            Err(PreparationError::InvalidConsent)
        );
        assert!(!service.busy());
        assert!(driver.calls.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn successful_execution_consumes_consent_once_and_verifies_last() {
        let (service, sink, driver) = setup(None, None);
        let consent = consent(&service).await;
        service.start(consent.clone()).await.unwrap();
        wait(&sink, |event| {
            matches!(event, OperationEvent::Finished { .. })
        })
        .await;
        assert_eq!(
            driver.calls.lock().unwrap().last(),
            Some(&StepId::VerifyJailbreak)
        );
        assert_eq!(
            service.start(consent).await,
            Err(PreparationError::PlanExpired)
        );
    }
    #[tokio::test]
    async fn failure_stops_writes_and_never_reports_success() {
        let (service, sink, driver) = setup(Some(StepId::InstallUntether), None);
        service.start(consent(&service).await).await.unwrap();
        wait(&sink, |event| {
            matches!(event, OperationEvent::Failed { .. })
        })
        .await;
        assert_eq!(
            driver.calls.lock().unwrap().last(),
            Some(&StepId::InstallUntether)
        );
        assert!(
            !sink
                .events
                .lock()
                .unwrap()
                .iter()
                .any(|event| matches!(event, OperationEvent::Finished { .. }))
        );
        assert!(!service.busy());
    }
    #[tokio::test]
    async fn waiting_for_dfu_is_cancellable_and_concurrent_start_is_rejected() {
        let (service, sink, driver) = setup(None, Some(StepId::EnterDfu));
        let consent = consent(&service).await;
        let handle = service.start(consent.clone()).await.unwrap();
        wait(&sink, |event| {
            matches!(event, OperationEvent::ActionRequired { .. })
        })
        .await;
        assert_eq!(service.start(consent).await, Err(PreparationError::Busy));
        service.cancel(&handle.operation_id).unwrap();
        wait(&sink, |event| {
            matches!(event, OperationEvent::Cancelled { .. })
        })
        .await;
        assert!(
            !driver
                .calls
                .lock()
                .unwrap()
                .contains(&StepId::ExploitBootrom)
        );
    }
    #[tokio::test]
    async fn critical_step_rejects_cancellation() {
        let (service, sink, driver) = setup(None, Some(StepId::InstallUntether));
        let handle = service.start(consent(&service).await).await.unwrap();
        wait(&sink, |event| {
            matches!(
                event,
                OperationEvent::StepChanged {
                    step_id: StepId::InstallUntether,
                    status: StepStatus::Running,
                    ..
                }
            )
        })
        .await;
        assert_eq!(
            service.cancel(&handle.operation_id),
            Err(PreparationError::NotCancellable)
        );
        driver.gate.notify_one();
        wait(&sink, |event| {
            matches!(event, OperationEvent::Finished { .. })
        })
        .await;
    }
}
