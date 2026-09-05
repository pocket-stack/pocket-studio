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
    pub model_identifier: String,
    pub marketing_name: String,
    pub chip: String,
    pub board_config: String,
    pub os_version: String,
    pub build_number: String,
    pub udid_masked: String,
    pub ecid_masked: String,
    pub serial_masked: String,
    pub storage_gb: u32,
    pub battery_percent: u8,
    pub mode: DeviceMode,
    pub transport: Transport,
}

/// Facts about the device that only a read-only probe can reveal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceFacts {
    pub jailbroken: bool,
    pub ssh_available: bool,
    pub pairing_trusted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DeviceEvent {
    Attached { device: Box<DeviceSummary> },
    Detached { device_id: String },
    ModeChanged { device_id: String, mode: DeviceMode },
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
