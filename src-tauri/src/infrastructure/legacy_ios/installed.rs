use super::LegacyIosProbe;
use crate::{
    application::installed::{InstalledError, InstalledFuture, InstalledReader},
    domain::{
        installed::{InstallationObservation, NativeApplication},
        now_millis,
        store::sha256_hex,
    },
};
use legacy_ios_services::{AppFilter, AppIdentifier};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::time::{Instant, timeout};

pub struct LegacyInstalledReader {
    pub(super) probe: Arc<LegacyIosProbe>,
}
impl LegacyInstalledReader {
    pub fn new(probe: Arc<LegacyIosProbe>) -> Self {
        Self { probe }
    }
}
impl LegacyIosProbe {
    pub(super) fn application_session_key(
        &self,
        device_id: &str,
    ) -> Result<String, InstalledError> {
        self.sessions
            .lock()
            .map_err(|_| InstalledError::DeviceUnavailable)?
            .iter()
            .find(|(key, id)| id.as_str() == device_id && key.starts_with("udid:"))
            .map(|(key, _)| key.clone())
            .ok_or(InstalledError::DeviceUnavailable)
    }
}
impl InstalledReader for LegacyInstalledReader {
    fn binding(&self, device_id: &str) -> Result<String, InstalledError> {
        Ok(sha256_hex(
            self.probe.application_session_key(device_id)?.as_bytes(),
        ))
    }
    fn read<'a>(
        &'a self,
        device_id: &'a str,
        bundle_ids: &'a [String],
    ) -> InstalledFuture<'a, InstallationObservation> {
        Box::pin(async move {
            let key = self.probe.application_session_key(device_id)?;
            let _access = timeout(Duration::from_secs(3), self.probe.normal_access.read())
                .await
                .map_err(|_| InstalledError::ReadFailed)?;
            if bundle_ids.is_empty() {
                return Ok(InstallationObservation {
                    applications: vec![],
                    observed_at: now_millis(),
                });
            }
            let udid = legacy_ios_core::Udid::new(
                key.strip_prefix("udid:").expect("validated session key"),
            );
            let device = timeout(Duration::from_secs(5), self.probe.normal.find_device(&udid))
                .await
                .map_err(|_| InstalledError::DeviceUnavailable)?
                .map_err(|_| InstalledError::DeviceUnavailable)?;
            let applications = read_registered(&device, bundle_ids).await?;
            if self.probe.application_session_key(device_id)? != key {
                return Err(InstalledError::DeviceUnavailable);
            }
            Ok(InstallationObservation {
                applications,
                observed_at: now_millis(),
            })
        })
    }
}
pub(super) async fn read_registered(
    device: &legacy_ios_services::NormalDevice,
    bundle_ids: &[String],
) -> Result<Vec<NativeApplication>, InstalledError> {
    let ids = bundle_ids
        .iter()
        .map(AppIdentifier::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| InstalledError::ReadFailed)?;
    let apps = device
        .lookup_apps(AppFilter::All, &ids)
        .await
        .map_err(|_| InstalledError::ReadFailed)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut applications = Vec::with_capacity(apps.len());
    for app in apps {
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_secs(2));
        let receipt_build_id = if remaining.is_zero() {
            None
        } else {
            match timeout(remaining, device.app_build_receipt(&app)).await {
                Ok(Ok(bytes)) => receipt_build(&bytes, app.bundle_id()),
                _ => None,
            }
        };
        applications.push(NativeApplication {
            bundle_id: app.bundle_id().into(),
            product_version: app.product_version().map(str::to_owned),
            build_number: app.build_version().map(str::to_owned),
            application_type: app.application_type().map(str::to_owned),
            receipt_build_id,
        });
    }
    Ok(applications)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    schema: u32,
    bundle_id: String,
    build_id: String,
}
pub(super) fn receipt_build(bytes: &[u8], bundle_id: &str) -> Option<String> {
    let receipt: Receipt = serde_json::from_slice(bytes).ok()?;
    (receipt.schema == 1
        && receipt.bundle_id == bundle_id
        && !receipt.build_id.is_empty()
        && receipt.build_id.len() <= 128
        && receipt
            .build_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')))
    .then_some(receipt.build_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipt_must_belong_to_the_registered_app() {
        let bytes = br#"{"schema":1,"bundleId":"native.notes","buildId":"abc123"}"#;
        assert_eq!(receipt_build(bytes, "native.notes"), Some("abc123".into()));
        assert_eq!(receipt_build(bytes, "native.clock"), None);
        assert_eq!(
            receipt_build(
                br#"{"schema":2,"bundleId":"native.notes","buildId":"abc123"}"#,
                "native.notes"
            ),
            None
        );
        assert_eq!(receipt_build(b"not-json", "native.notes"), None);
    }
}
