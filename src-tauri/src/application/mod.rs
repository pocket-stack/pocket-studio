//! Use cases that coordinate domain rules with the transport and tool ports.
//! This layer knows nothing about Tauri or any platform.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::catalog::{self, CatalogEntry, CompatibilityVerdict, InstalledPackage};
use crate::domain::device::{DeviceEvent, DeviceFacts, DeviceSummary};
use crate::domain::log::{LogEntry, LogLevel, LogSource};
use crate::domain::now_millis;
use crate::domain::operation::{CancelError, OperationEvent, OperationHandle};
use crate::domain::preparation::{ConsentError, ConsentRecord, PreparationPlan, validate_consent};
use crate::domain::readiness::{self, ReadinessReport};

/// Where domain events go. The Tauri layer forwards them to the webview.
pub trait EventSink: Send + Sync + 'static {
    fn device(&self, event: DeviceEvent);
    fn operation(&self, event: OperationEvent);
    fn log(&self, entry: LogEntry);
}

/// Read-only view of attached devices.
pub trait DeviceInventory: Send + Sync {
    fn list(&self) -> Vec<DeviceSummary>;
    fn find(&self, device_id: &str) -> Option<DeviceSummary>;
    fn facts(&self, device_id: &str) -> Option<DeviceFacts>;
}

/// Executes workflows against a device and streams progress through the sink.
pub trait WorkflowRunner: Send + Sync {
    fn run_preparation(&self, plan: &PreparationPlan) -> OperationHandle;
    fn run_install(&self, device: &DeviceSummary, entry: &CatalogEntry) -> OperationHandle;
    fn cancel(&self, operation_id: &str) -> Result<(), CancelError>;
}

/// Verified package manifests plus what is installed on a device.
pub trait PackageCatalog: Send + Sync {
    fn entries(&self) -> Vec<CatalogEntry>;
    fn installed(&self, device_id: &str) -> Vec<InstalledPackage>;
}

/// In-memory, exportable activity log. Every entry is mirrored to the sink.
pub struct OperationLog {
    entries: Mutex<Vec<LogEntry>>,
    sequence: AtomicU64,
    sink: Arc<dyn EventSink>,
}

impl OperationLog {
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
            sink,
        }
    }

    pub fn record(
        &self,
        level: LogLevel,
        source: LogSource,
        code: &str,
        message: impl Into<String>,
        params: Option<BTreeMap<String, String>>,
        operation_id: Option<&str>,
    ) {
        let id = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let entry = LogEntry {
            id: format!("log-{id}"),
            timestamp: now_millis(),
            level,
            source,
            code: code.to_owned(),
            message: message.into(),
            params,
            operation_id: operation_id.map(str::to_owned),
        };
        match level {
            LogLevel::Error => tracing::error!(code, "{}", entry.message),
            LogLevel::Warn => tracing::warn!(code, "{}", entry.message),
            LogLevel::Info => tracing::info!(code, "{}", entry.message),
            LogLevel::Debug => tracing::debug!(code, "{}", entry.message),
        }
        self.entries
            .lock()
            .expect("operation log poisoned")
            .push(entry.clone());
        self.sink.log(entry);
    }

    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.lock().expect("operation log poisoned").clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StudioError {
    #[error("device is not attached")]
    DeviceNotFound,
    #[error("preparation plan not found")]
    UnknownPlan,
    #[error(transparent)]
    Consent(#[from] ConsentError),
    #[error("package not found in catalog")]
    UnknownPackage,
    #[error("package is not installable on this device: {0:?}")]
    NotInstallable(CompatibilityVerdict),
    #[error(transparent)]
    Cancel(#[from] CancelError),
}

impl StudioError {
    /// Stable code for the command boundary; never a debug string.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "deviceNotFound",
            Self::UnknownPlan => "unknownPlan",
            Self::Consent(ConsentError::DisclaimerVersionMismatch) => "disclaimerOutdated",
            Self::Consent(ConsentError::ReadingTooShort) => "readingTooShort",
            Self::Consent(_) => "consentIncomplete",
            Self::UnknownPackage => "unknownPackage",
            Self::NotInstallable(CompatibilityVerdict::RequiresPreparation) => "deviceNotReady",
            Self::NotInstallable(_) => "incompatiblePackage",
            Self::Cancel(CancelError::UnknownOperation) => "unknownOperation",
            Self::Cancel(CancelError::AlreadyFinished) => "alreadyFinished",
            Self::Cancel(CancelError::NotCancellable) => "notCancellable",
        }
    }
}

/// The application façade the command layer talks to.
pub struct Studio {
    inventory: Arc<dyn DeviceInventory>,
    runner: Arc<dyn WorkflowRunner>,
    catalog: Arc<dyn PackageCatalog>,
    log: Arc<OperationLog>,
    plans: Mutex<HashMap<String, PreparationPlan>>,
    plan_sequence: AtomicU64,
}

impl Studio {
    pub fn new(
        inventory: Arc<dyn DeviceInventory>,
        runner: Arc<dyn WorkflowRunner>,
        catalog: Arc<dyn PackageCatalog>,
        log: Arc<OperationLog>,
    ) -> Self {
        Self {
            inventory,
            runner,
            catalog,
            log,
            plans: Mutex::new(HashMap::new()),
            plan_sequence: AtomicU64::new(0),
        }
    }

    pub fn list_devices(&self) -> Vec<DeviceSummary> {
        self.inventory.list()
    }

    fn device(&self, device_id: &str) -> Result<(DeviceSummary, DeviceFacts), StudioError> {
        let device = self
            .inventory
            .find(device_id)
            .ok_or(StudioError::DeviceNotFound)?;
        let facts = self
            .inventory
            .facts(device_id)
            .ok_or(StudioError::DeviceNotFound)?;
        Ok((device, facts))
    }

    /// Read-only. Must never change the device.
    pub fn check_readiness(&self, device_id: &str) -> Result<ReadinessReport, StudioError> {
        let (device, facts) = self.device(device_id)?;
        let report = readiness::evaluate(&device, facts);
        self.log.record(
            LogLevel::Info,
            LogSource::Device,
            "log.device.readinessChecked",
            format!("readiness: {:?}", report.status),
            Some(params([("status", format!("{:?}", report.status))])),
            None,
        );
        Ok(report)
    }

    pub fn plan_preparation(&self, device_id: &str) -> Result<PreparationPlan, StudioError> {
        let (device, _) = self.device(device_id)?;
        let sequence = self.plan_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let plan = PreparationPlan::ramdisk_jailbreak(
            format!("plan-{sequence}"),
            device.id.clone(),
            &device.os_version,
        );
        self.log.record(
            LogLevel::Debug,
            LogSource::Preparation,
            "log.preparation.planned",
            format!("plan {} created", plan.id),
            Some(params([("plan", plan.id.clone())])),
            None,
        );
        self.plans
            .lock()
            .expect("plans poisoned")
            .insert(plan.id.clone(), plan.clone());
        Ok(plan)
    }

    /// The only entry point into a device-changing workflow. Refuses to run
    /// without consent that binds to the plan.
    pub fn start_preparation(
        &self,
        consent: ConsentRecord,
    ) -> Result<OperationHandle, StudioError> {
        let plan = self
            .plans
            .lock()
            .expect("plans poisoned")
            .get(&consent.plan_id)
            .cloned()
            .ok_or(StudioError::UnknownPlan)?;
        self.device(&plan.device_id)?;
        validate_consent(&plan, &consent)?;
        let handle = self.runner.run_preparation(&plan);
        self.log.record(
            LogLevel::Info,
            LogSource::Preparation,
            "log.preparation.consentRecorded",
            format!(
                "consent recorded for plan {} (risks read {}s, disclaimer {} read {}s)",
                plan.id,
                consent.risk_reading_seconds,
                consent.disclaimer_version,
                consent.disclaimer_reading_seconds
            ),
            Some(params([("plan", plan.id.clone())])),
            Some(&handle.operation_id),
        );
        Ok(handle)
    }

    pub fn catalog(&self) -> Vec<CatalogEntry> {
        self.catalog.entries()
    }

    pub fn installed(&self, device_id: &str) -> Result<Vec<InstalledPackage>, StudioError> {
        self.device(device_id)?;
        Ok(self.catalog.installed(device_id))
    }

    pub fn install(
        &self,
        device_id: &str,
        package_id: &str,
    ) -> Result<OperationHandle, StudioError> {
        let (device, facts) = self.device(device_id)?;
        let entry = self
            .catalog
            .entries()
            .into_iter()
            .find(|entry| entry.id == package_id)
            .ok_or(StudioError::UnknownPackage)?;
        match catalog::evaluate(&entry, &device, facts.jailbroken) {
            CompatibilityVerdict::Compatible => {}
            verdict => return Err(StudioError::NotInstallable(verdict)),
        }
        Ok(self.runner.run_install(&device, &entry))
    }

    pub fn cancel(&self, operation_id: &str) -> Result<(), StudioError> {
        Ok(self.runner.cancel(operation_id)?)
    }

    pub fn logs(&self) -> Vec<LogEntry> {
        self.log.entries()
    }

    /// Stub: a real implementation writes the entries to the platform log
    /// directory and returns the path.
    pub fn export_logs(&self) -> String {
        let path = "~/Library/Logs/PocketStudio/session.log".to_owned();
        self.log.record(
            LogLevel::Info,
            LogSource::System,
            "log.system.exported",
            format!("log exported to {path}"),
            Some(params([("path", path.clone())])),
            None,
        );
        path
    }
}

pub fn params<const N: usize>(pairs: [(&str, String); N]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}
