//! Use cases that coordinate domain rules with the transport and tool ports.
//! This layer knows nothing about Tauri or any platform.

pub mod discovery;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::device::{DeviceEvent, DiscoverySnapshot};
use crate::domain::log::{LogEntry, LogLevel, LogSource};
use crate::domain::now_millis;
use crate::domain::operation::OperationEvent;
use crate::domain::readiness::{self, ReadinessReport};

/// Where domain events go. The Tauri layer forwards them to the webview.
pub trait EventSink: Send + Sync + 'static {
    fn device(&self, event: DeviceEvent);
    fn operation(&self, event: OperationEvent);
    fn log(&self, entry: LogEntry);
}

/// In-memory, exportable activity log. Every entry is mirrored to the sink.
pub struct OperationLog {
    entries: Mutex<Vec<LogEntry>>,
    sequence: AtomicU64,
    session_id: String,
    sink: Arc<dyn EventSink>,
}

impl OperationLog {
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
            session_id: uuid::Uuid::new_v4().to_string(),
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
            id: format!("{}-{id}", self.session_id),
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
        {
            let mut entries = self.entries.lock().expect("operation log poisoned");
            entries.push(entry.clone());
            if entries.len() > 2000 {
                entries.remove(0);
            }
        }
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
    #[error("this operation is unavailable")]
    OperationUnavailable,
}

impl StudioError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "deviceNotFound",
            Self::OperationUnavailable => "operationUnavailable",
        }
    }
}

/// Native reads use the real inventory. No simulated workflow runner is
/// connected to a physical device; writes remain unavailable at this boundary.
pub struct Studio {
    pub discovery: Arc<discovery::DeviceDiscovery>,
    log: Arc<OperationLog>,
}

impl Studio {
    pub fn new(discovery: Arc<discovery::DeviceDiscovery>, log: Arc<OperationLog>) -> Self {
        Self { discovery, log }
    }

    pub async fn list_devices(&self) -> DiscoverySnapshot {
        let snapshot = self.discovery.refresh().await;
        self.log.record(
            LogLevel::Info,
            LogSource::Device,
            "log.device.scanCompleted",
            format!(
                "User-requested device scan completed: {} device(s)",
                snapshot.devices.len()
            ),
            Some(params([("count", snapshot.devices.len().to_string())])),
            None,
        );
        snapshot
    }

    pub async fn check_readiness(&self, device_id: &str) -> Result<ReadinessReport, StudioError> {
        self.discovery.refresh().await;
        let record = self
            .discovery
            .find(device_id)
            .ok_or(StudioError::DeviceNotFound)?;
        let report = readiness::evaluate(&record.summary, record.facts);
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

    pub fn unavailable_operation<T>(&self) -> Result<T, StudioError> {
        Err(StudioError::OperationUnavailable)
    }

    pub fn logs(&self) -> Vec<LogEntry> {
        self.log.entries()
    }

    pub fn export_logs(&self) -> String {
        self.logs()
            .iter()
            .map(|entry| {
                format!(
                    "{} {:?} [{}] {}",
                    entry.timestamp, entry.level, entry.code, entry.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn params<const N: usize>(pairs: [(&str, String); N]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}
