//! Serializes scans and publishes atomic, revisioned snapshots. A failed scan
//! clears stale readiness instead of reporting a cached device as connected.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{EventSink, OperationLog};
use crate::domain::device::{
    DeviceEvent, DeviceFacts, DeviceSummary, DiscoveryIssue, DiscoveryIssueCode, DiscoverySnapshot,
};
use crate::domain::log::{LogLevel, LogSource};
use crate::domain::now_millis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRecord {
    pub summary: DeviceSummary,
    pub facts: DeviceFacts,
}

#[derive(Debug, Default)]
pub struct ProbeSnapshot {
    pub records: Vec<DeviceRecord>,
    pub issues: Vec<DiscoveryIssue>,
}

pub type ProbeFuture<'a> = Pin<Box<dyn Future<Output = ProbeSnapshot> + Send + 'a>>;
pub type AppSyncCheckFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), AppSyncCheckError>> + Send + 'a>>;

#[derive(Debug, thiserror::Error)]
pub enum AppSyncCheckError {
    #[error("device changed or disconnected")]
    DeviceChanged,
    #[error("device access is busy")]
    Busy,
    #[error("device SSH password was rejected")]
    Authentication,
    #[error("could not read AppSync status")]
    Unavailable,
}
impl AppSyncCheckError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceChanged => "deviceChanged",
            Self::Busy => "operationBusy",
            Self::Authentication => "sshAuthenticationFailed",
            Self::Unavailable => "appSyncCheckUnavailable",
        }
    }
}

/// Implementations may enumerate and read; they must never pair, change USB
/// mode, install, or authenticate to a privileged shell as part of a scan.
pub trait DeviceProbe: Send + Sync {
    fn scan(&self) -> ProbeFuture<'_>;
    /// Explicit user-requested authentication for a read-only package check.
    /// This must never be called implicitly by discovery or install a package.
    fn check_appsync(&self, _device_id: String, _password: String) -> AppSyncCheckFuture<'_> {
        Box::pin(async { Err(AppSyncCheckError::Unavailable) })
    }
}

#[derive(Default)]
struct InventoryState {
    records: Vec<DeviceRecord>,
    snapshot: DiscoverySnapshot,
}

pub struct DeviceDiscovery {
    probe: Arc<dyn DeviceProbe>,
    state: Mutex<InventoryState>,
    scan_lock: tokio::sync::Mutex<()>,
    sink: Arc<dyn EventSink>,
    log: Arc<OperationLog>,
    timeout: Duration,
}

impl DeviceDiscovery {
    pub async fn check_appsync(
        &self,
        device_id: String,
        password: String,
    ) -> Result<(), AppSyncCheckError> {
        let _scan = self.scan_lock.lock().await;
        tokio::time::timeout(
            Duration::from_secs(60),
            self.probe.check_appsync(device_id, password),
        )
        .await
        .map_err(|_| AppSyncCheckError::Unavailable)?
    }
    pub fn new(
        probe: Arc<dyn DeviceProbe>,
        sink: Arc<dyn EventSink>,
        log: Arc<OperationLog>,
    ) -> Self {
        Self {
            probe,
            state: Mutex::new(InventoryState::default()),
            scan_lock: tokio::sync::Mutex::new(()),
            sink,
            log,
            // USB enumeration (3s) plus parallel bounded normal reads (24s).
            timeout: Duration::from_secs(30),
        }
    }

    pub async fn refresh(&self) -> DiscoverySnapshot {
        let requested_revision = self
            .state
            .lock()
            .expect("device inventory poisoned")
            .snapshot
            .revision;
        let _guard = self.scan_lock.lock().await;
        {
            let state = self.state.lock().expect("device inventory poisoned");
            if state.snapshot.revision != requested_revision {
                // A concurrent refresh completed after this request began.
                // Share that result rather than queueing another slow scan.
                return state.snapshot.clone();
            }
        }
        let observed = tokio::time::timeout(self.timeout, self.probe.scan())
            .await
            .unwrap_or_else(|_| ProbeSnapshot {
                records: vec![],
                issues: vec![DiscoveryIssue {
                    code: DiscoveryIssueCode::ProbeTimeout,
                    device_id: None,
                }],
            });
        let (snapshot, changed, attached, detached) = {
            let mut state = self.state.lock().expect("device inventory poisoned");
            let changed =
                state.records != observed.records || state.snapshot.issues != observed.issues;
            let attached = observed
                .records
                .iter()
                .filter(|record| {
                    !state
                        .records
                        .iter()
                        .any(|old| old.summary.id == record.summary.id)
                })
                .count();
            let detached = state
                .records
                .iter()
                .filter(|old| {
                    !observed
                        .records
                        .iter()
                        .any(|record| record.summary.id == old.summary.id)
                })
                .count();
            state.snapshot = DiscoverySnapshot {
                revision: state.snapshot.revision + 1,
                devices: observed
                    .records
                    .iter()
                    .map(|record| record.summary.clone())
                    .collect(),
                reports: observed
                    .records
                    .iter()
                    .map(|record| crate::domain::readiness::evaluate(&record.summary, record.facts))
                    .collect(),
                issues: observed.issues,
                checked_at: now_millis(),
            };
            state.records = observed.records;
            (state.snapshot.clone(), changed, attached, detached)
        };
        if changed {
            self.log.record(
                if snapshot.issues.is_empty() { LogLevel::Info } else { LogLevel::Warn },
                LogSource::Device, "log.device.inventoryUpdated",
                format!("USB inventory updated: {} device(s), {attached} attached, {detached} detached, {} diagnostic(s)", snapshot.devices.len(), snapshot.issues.len()),
                Some(super::params([("count", snapshot.devices.len().to_string())])), None,
            );
            self.sink.device(DeviceEvent::Snapshot {
                snapshot: snapshot.clone(),
            });
            for issue in &snapshot.issues {
                self.log.record(
                    LogLevel::Warn,
                    LogSource::Device,
                    "log.device.diagnostic",
                    format!("Device diagnostic: {}", issue.code.code()),
                    Some(super::params([("issue", issue.code.code().to_owned())])),
                    None,
                );
            }
        }
        snapshot
    }

    pub fn find(&self, device_id: &str) -> Option<DeviceRecord> {
        self.state
            .lock()
            .expect("device inventory poisoned")
            .records
            .iter()
            .find(|record| record.summary.id == device_id)
            .cloned()
    }

    pub async fn monitor(&self) {
        loop {
            self.refresh().await;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{log::LogEntry, operation::OperationEvent, readiness::ReadinessStatus};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Sink(Mutex<Vec<DeviceEvent>>);
    impl EventSink for Sink {
        fn device(&self, event: DeviceEvent) {
            self.0.lock().unwrap().push(event);
        }
        fn operation(&self, _: OperationEvent) {}
        fn log(&self, _: LogEntry) {}
    }
    struct Probe(Mutex<VecDeque<Option<ProbeSnapshot>>>);
    impl DeviceProbe for Probe {
        fn scan(&self) -> ProbeFuture<'_> {
            let next = self.0.lock().unwrap().pop_front().unwrap();
            Box::pin(async move {
                match next {
                    Some(snapshot) => snapshot,
                    None => std::future::pending().await,
                }
            })
        }
    }
    fn connected() -> ProbeSnapshot {
        ProbeSnapshot {
            records: vec![DeviceRecord {
                summary: serde_json::from_value(serde_json::json!({
                    "id": "session-a", "platform": "ios", "modelIdentifier": "iPod4,1",
                    "marketingName": "iPod touch 4", "osVersion": "6.1.6", "mode": "normal", "transport": "usb"
                })).unwrap(),
                facts: DeviceFacts { appsync_last_observation: None, appsync_installed: Some(true), jailbroken: Some(true), pairing_trusted: Some(true), ssh_available: Some(true) },
            }], issues: vec![],
        }
    }
    fn discovery(scans: Vec<Option<ProbeSnapshot>>) -> (DeviceDiscovery, Arc<Sink>) {
        let sink = Arc::new(Sink::default());
        let probe = Arc::new(Probe(Mutex::new(scans.into())));
        let log = Arc::new(OperationLog::new(sink.clone()));
        let mut service = DeviceDiscovery::new(probe, sink.clone(), log);
        service.timeout = Duration::from_millis(10);
        (service, sink)
    }

    #[tokio::test]
    async fn detaching_removes_cached_readiness_and_publishes_an_empty_snapshot() {
        let (service, sink) = discovery(vec![Some(connected()), Some(ProbeSnapshot::default())]);
        let first = service.refresh().await;
        assert_eq!(first.reports[0].status, ReadinessStatus::Ready);
        let second = service.refresh().await;
        assert!(second.devices.is_empty() && second.reports.is_empty());
        assert!(service.find("session-a").is_none());
        assert!(second.revision > first.revision);
        let events = sink.0.lock().unwrap();
        let DeviceEvent::Snapshot { snapshot } = events.last().unwrap();
        assert!(snapshot.devices.is_empty());
    }

    #[tokio::test]
    async fn a_manual_refresh_shares_an_in_flight_monitor_scan() {
        struct DelayedProbe {
            calls: std::sync::atomic::AtomicUsize,
            entered: tokio::sync::Notify,
            finish: tokio::sync::Notify,
        }
        impl DeviceProbe for DelayedProbe {
            fn scan(&self) -> ProbeFuture<'_> {
                Box::pin(async {
                    self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    self.entered.notify_one();
                    self.finish.notified().await;
                    connected()
                })
            }
        }
        let probe = Arc::new(DelayedProbe {
            calls: Default::default(),
            entered: Default::default(),
            finish: Default::default(),
        });
        let sink = Arc::new(Sink::default());
        let service = Arc::new(DeviceDiscovery::new(
            probe.clone(),
            sink.clone(),
            Arc::new(OperationLog::new(sink)),
        ));
        let monitor = {
            let service = service.clone();
            tokio::spawn(async move { service.refresh().await })
        };
        probe.entered.notified().await;
        let manual = service.refresh();
        tokio::pin!(manual);
        tokio::select! {
            biased;
            _ = &mut manual => panic!("refresh should wait for the in-flight scan"),
            _ = tokio::task::yield_now() => {},
        }
        probe.finish.notify_one();
        let monitor_result = monitor.await.unwrap();
        let manual_result = manual.await;
        assert_eq!(monitor_result.revision, manual_result.revision);
        assert_eq!(manual_result.reports[0].status, ReadinessStatus::Ready);
        assert_eq!(probe.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn timeout_invalidates_a_previously_ready_device() {
        let (service, _) = discovery(vec![Some(connected()), None]);
        service.refresh().await;
        let failed = service.refresh().await;
        assert_eq!(failed.issues[0].code, DiscoveryIssueCode::ProbeTimeout);
        assert!(failed.devices.is_empty() && failed.reports.is_empty());
        assert!(service.find("session-a").is_none());
    }

    #[tokio::test]
    async fn unchanged_polls_do_not_flood_events_but_manual_reads_advance_the_revision() {
        let (service, sink) = discovery(vec![Some(connected()), Some(connected())]);
        let first = service.refresh().await;
        let second = service.refresh().await;
        assert!(second.revision > first.revision);
        assert_eq!(sink.0.lock().unwrap().len(), 1);
        let payload = serde_json::to_value(&sink.0.lock().unwrap()[0]).unwrap();
        assert_eq!(payload["type"], "snapshot");
        assert_eq!(payload["snapshot"]["reports"][0]["deviceId"], "session-a");
        assert_eq!(
            payload["snapshot"]["devices"][0]["modelIdentifier"],
            "iPod4,1"
        );
    }
}
