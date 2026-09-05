use serde::{Deserialize, Serialize};

use super::device::{DeviceFacts, DeviceSummary, Platform, compare_versions};
use super::now_millis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadinessCheckId {
    PlatformSupported,
    ModelSupported,
    OsVersionSupported,
    PairingTrusted,
    Jailbroken,
    SshAvailable,
    BatteryLevel,
    PhysicalButtons,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessCheck {
    pub id: ReadinessCheckId,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadinessStatus {
    Ready,
    NeedsPreparation,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowKind {
    Jailbreak,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessReport {
    pub device_id: String,
    pub status: ReadinessStatus,
    pub checks: Vec<ReadinessCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_workflow: Option<WorkflowKind>,
    pub checked_at: u64,
}

/// Models the ramdisk jailbreak supports, with the iOS range per model taken
/// from the Legacy iOS Kit jailbreak matrix.
const SUPPORTED_MODELS: &[(&str, &str, &str)] = &[("iPod4,1", "3.1.3", "6.1.6")];

const MINIMUM_BATTERY_PERCENT: u8 = 50;

fn supported_range(model: &str) -> Option<(&'static str, &'static str)> {
    SUPPORTED_MODELS
        .iter()
        .find(|(candidate, _, _)| *candidate == model)
        .map(|(_, min, max)| (*min, *max))
}

/// Pure readiness rule: decides whether a device can join the Pocket
/// ecosystem as-is, needs preparation, or is not supported.
pub fn evaluate(device: &DeviceSummary, facts: DeviceFacts) -> ReadinessReport {
    let mut checks = Vec::with_capacity(8);
    let mut unsupported = false;

    checks.push(ReadinessCheck {
        id: ReadinessCheckId::PlatformSupported,
        status: match device.platform {
            Platform::Ios => CheckStatus::Pass,
        },
        value: Some("iOS".into()),
    });

    let range = supported_range(&device.model_identifier);
    unsupported |= range.is_none();
    checks.push(ReadinessCheck {
        id: ReadinessCheckId::ModelSupported,
        status: if range.is_some() {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        value: Some(device.model_identifier.clone()),
    });

    let os_supported = range.is_some_and(|(min, max)| {
        compare_versions(&device.os_version, min).is_ge()
            && compare_versions(&device.os_version, max).is_le()
    });
    unsupported |= !os_supported;
    checks.push(ReadinessCheck {
        id: ReadinessCheckId::OsVersionSupported,
        status: if os_supported {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        value: Some(device.os_version.clone()),
    });

    checks.push(ReadinessCheck {
        id: ReadinessCheckId::PairingTrusted,
        status: if facts.pairing_trusted {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        value: None,
    });

    checks.push(ReadinessCheck {
        id: ReadinessCheckId::Jailbroken,
        status: if facts.jailbroken {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        value: None,
    });

    checks.push(ReadinessCheck {
        id: ReadinessCheckId::SshAvailable,
        status: match (facts.jailbroken, facts.ssh_available) {
            (true, true) => CheckStatus::Pass,
            (true, false) => CheckStatus::Fail,
            (false, _) => CheckStatus::Unknown,
        },
        value: None,
    });

    checks.push(ReadinessCheck {
        id: ReadinessCheckId::BatteryLevel,
        status: if device.battery_percent >= MINIMUM_BATTERY_PERCENT {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        value: Some(format!("{}%", device.battery_percent)),
    });

    // Button health cannot be probed; the user attests it during preparation.
    checks.push(ReadinessCheck {
        id: ReadinessCheckId::PhysicalButtons,
        status: CheckStatus::Unknown,
        value: None,
    });

    let (status, required_workflow) = if unsupported {
        (ReadinessStatus::Unsupported, None)
    } else if facts.jailbroken {
        (ReadinessStatus::Ready, None)
    } else {
        (
            ReadinessStatus::NeedsPreparation,
            Some(WorkflowKind::Jailbreak),
        )
    };

    ReadinessReport {
        device_id: device.id.clone(),
        status,
        checks,
        required_workflow,
        checked_at: now_millis(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::{DeviceMode, Transport};

    fn ipod4(os_version: &str) -> DeviceSummary {
        DeviceSummary {
            id: "test".into(),
            platform: Platform::Ios,
            model_identifier: "iPod4,1".into(),
            marketing_name: "iPod touch (4th generation)".into(),
            chip: "A4".into(),
            board_config: "N81AP".into(),
            os_version: os_version.into(),
            build_number: "10B500".into(),
            udid_masked: "…".into(),
            ecid_masked: "…".into(),
            serial_masked: "…".into(),
            storage_gb: 32,
            battery_percent: 80,
            mode: DeviceMode::Normal,
            transport: Transport::Usb,
        }
    }

    #[test]
    fn stock_supported_device_needs_jailbreak() {
        let report = evaluate(&ipod4("6.1.6"), DeviceFacts::default());
        assert_eq!(report.status, ReadinessStatus::NeedsPreparation);
        assert_eq!(report.required_workflow, Some(WorkflowKind::Jailbreak));
    }

    #[test]
    fn jailbroken_device_is_ready() {
        let facts = DeviceFacts {
            jailbroken: true,
            ssh_available: true,
            pairing_trusted: true,
        };
        let report = evaluate(&ipod4("6.1.6"), facts);
        assert_eq!(report.status, ReadinessStatus::Ready);
        assert!(report.required_workflow.is_none());
    }

    #[test]
    fn os_outside_matrix_is_unsupported() {
        let report = evaluate(&ipod4("7.0"), DeviceFacts::default());
        assert_eq!(report.status, ReadinessStatus::Unsupported);
    }

    #[test]
    fn unknown_model_is_unsupported() {
        let mut device = ipod4("6.1.6");
        device.model_identifier = "iPhone9,9".into();
        let report = evaluate(&device, DeviceFacts::default());
        assert_eq!(report.status, ReadinessStatus::Unsupported);
    }
}
