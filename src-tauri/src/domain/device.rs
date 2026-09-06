use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Ios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceMode {
    Normal,
    Recovery,
    Dfu,
    Wtf,
    Kis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Transport {
    Usb,
    Network,
}

/// Read-only identity of an attached device. Identifiers are masked before
/// they leave the infrastructure layer; the full values never reach the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub id: String,
    pub platform: Platform,
    pub model_identifier: Option<String>,
    pub marketing_name: String,
    pub chip: Option<String>,
    pub board_config: Option<String>,
    pub os_version: Option<String>,
    pub build_number: Option<String>,
    pub udid_masked: Option<String>,
    pub ecid_masked: Option<String>,
    pub serial_masked: Option<String>,
    pub storage_gb: Option<u32>,
    pub storage_total_bytes: Option<u64>,
    pub storage_free_bytes: Option<u64>,
    pub battery_percent: Option<u8>,
    pub mode: DeviceMode,
    pub transport: Transport,
}

/// Facts about the device that only a read-only probe can reveal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceFacts {
    pub jailbroken: Option<bool>,
    pub ssh_available: Option<bool>,
    pub pairing_trusted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DeviceEvent {
    Snapshot { snapshot: DiscoverySnapshot },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryIssueCode {
    UsbUnavailable,
    MacosMuxUnavailable,
    LinuxMuxUnavailable,
    WindowsMuxUnavailable,
    DeviceInfoUnavailable,
    PairingUnavailable,
    PairingSessionFailed,
    ProbeTimeout,
}

impl DiscoveryIssueCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::UsbUnavailable => "usbUnavailable",
            Self::MacosMuxUnavailable => "macosMuxUnavailable",
            Self::LinuxMuxUnavailable => "linuxMuxUnavailable",
            Self::WindowsMuxUnavailable => "windowsMuxUnavailable",
            Self::DeviceInfoUnavailable => "deviceInfoUnavailable",
            Self::PairingUnavailable => "pairingUnavailable",
            Self::PairingSessionFailed => "pairingSessionFailed",
            Self::ProbeTimeout => "probeTimeout",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryIssue {
    pub code: DiscoveryIssueCode,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySnapshot {
    pub revision: u64,
    pub devices: Vec<DeviceSummary>,
    pub reports: Vec<super::readiness::ReadinessReport>,
    pub issues: Vec<DiscoveryIssue>,
    pub checked_at: u64,
}

/// Compare dotted version strings numerically (`6.1.6` > `6.1`).
pub fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| -> Vec<u32> {
        value
            .split('.')
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    };
    let left = parse(left);
    let right = parse(right);
    let length = left.len().max(right.len());
    for index in 0..length {
        let l = left.get(index).copied().unwrap_or(0);
        let r = right.get(index).copied().unwrap_or(0);
        match l.cmp(&r) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn versions_compare_numerically_not_lexically() {
        assert_eq!(compare_versions("6.1.6", "6.1"), Ordering::Greater);
        assert_eq!(compare_versions("9.3.4", "10.0"), Ordering::Less);
        assert_eq!(compare_versions("6.1", "6.1.0"), Ordering::Equal);
    }
}
