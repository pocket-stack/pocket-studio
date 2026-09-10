//! Independent platform ports share one queue; a timed out adapter only loses
//! its own observation. No platform can erase another platform's inventory.
use super::three_ds::ThreeDsBridge;
use crate::{
    application::{discovery::*, installed::*, packages::*},
    domain::device::{DiscoveryIssue, DiscoveryIssueCode, Platform},
};
use std::{sync::Arc, time::Duration};
pub struct PlatformProbe {
    pub ios: Arc<dyn DeviceProbe>,
    pub three_ds: Arc<dyn DeviceProbe>,
}
impl DeviceProbe for PlatformProbe {
    fn scan(&self) -> ProbeFuture<'_> {
        Box::pin(async {
            async fn bounded(probe: &dyn DeviceProbe) -> ProbeSnapshot {
                tokio::time::timeout(Duration::from_secs(28), probe.scan())
                    .await
                    .unwrap_or_else(|_| ProbeSnapshot {
                        records: vec![],
                        issues: vec![DiscoveryIssue {
                            code: DiscoveryIssueCode::ProbeTimeout,
                            device_id: None,
                        }],
                    })
            }
            let (mut ios, mut three_ds) =
                tokio::join!(bounded(self.ios.as_ref()), bounded(self.three_ds.as_ref()));
            ios.records.append(&mut three_ds.records);
            ios.issues.append(&mut three_ds.issues);
            ios
        })
    }
    fn check_appsync(&self, id: String, password: String) -> AppSyncCheckFuture<'_> {
        self.ios.check_appsync(id, password)
    }
}
pub struct PlatformInstalled {
    pub ios: Arc<dyn InstalledReader>,
    pub three_ds: Arc<ThreeDsBridge>,
}
impl PlatformInstalled {
    fn reader(&self, id: &str) -> &dyn InstalledReader {
        if self.three_ds.contains(id) {
            self.three_ds.as_ref()
        } else {
            self.ios.as_ref()
        }
    }
}
impl InstalledReader for PlatformInstalled {
    fn platform(&self, id: &str) -> Result<Platform, InstalledError> {
        self.reader(id).platform(id)
    }
    fn binding(&self, id: &str) -> Result<String, InstalledError> {
        self.reader(id).binding(id)
    }
    fn read<'a>(
        &'a self,
        id: &'a str,
        apps: &'a [String],
    ) -> InstalledFuture<'a, crate::domain::installed::InstallationObservation> {
        self.reader(id).read(id, apps)
    }
}
pub struct PlatformPackages {
    pub ios: Arc<dyn PackageDriver>,
    pub three_ds: Arc<ThreeDsBridge>,
}
impl PackageDriver for PlatformPackages {
    fn target(&self, id: &str) -> Result<Arc<dyn PackageTarget>, PackageError> {
        if self.three_ds.contains(id) {
            self.three_ds.target(id)
        } else {
            self.ios.target(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Broken;
    impl DeviceProbe for Broken {
        fn scan(&self) -> ProbeFuture<'_> {
            Box::pin(async {
                ProbeSnapshot {
                    records: vec![],
                    issues: vec![DiscoveryIssue {
                        code: DiscoveryIssueCode::UsbUnavailable,
                        device_id: None,
                    }],
                }
            })
        }
    }
    struct Connected;
    impl DeviceProbe for Connected {
        fn scan(&self) -> ProbeFuture<'_> {
            Box::pin(async {
                ProbeSnapshot{records:vec![DeviceRecord{summary:serde_json::from_value(serde_json::json!({"id":"3ds","platform":"3ds","marketingName":"New 3DS LL","mode":"normal","transport":"network"})).unwrap(),facts:Default::default()}],issues:vec![]}
            })
        }
    }
    #[tokio::test]
    async fn an_ios_discovery_failure_does_not_clear_a_3ds_observation() {
        let probe = PlatformProbe {
            ios: Arc::new(Broken),
            three_ds: Arc::new(Connected),
        };
        let result = probe.scan().await;
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].summary.platform, Platform::ThreeDs);
        assert_eq!(result.issues.len(), 1);
    }
}
