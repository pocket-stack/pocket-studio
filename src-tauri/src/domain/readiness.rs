use serde::{Deserialize, Serialize};

use super::device::{DeviceFacts, DeviceMode, DeviceSummary, Platform, compare_versions};
use super::now_millis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadinessCheckId {
    PlatformSupported,
    ModelSupported,
    OsVersionSupported,
    PairingTrusted,
    Jailbroken,
    AppSyncInstalled,
    SshAvailable,
    BatteryLevel,
    PhysicalButtons,
    NormalMode,
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
    NeedsAttention,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowKind {
    Jailbreak,
    AppSync,
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
    let range = device.model_identifier.as_deref().and_then(supported_range);
    let model_supported = device.model_identifier.as_ref().map(|_| range.is_some());
    let os_supported = device.os_version.as_deref().and_then(|version| {
        range.map(|(min, max)| {
            compare_versions(version, min).is_ge() && compare_versions(version, max).is_le()
        })
    });
    let checks = vec![
        check(
            ReadinessCheckId::PlatformSupported,
            Some(device.platform == Platform::Ios),
            Some("iOS".into()),
        ),
        check(
            ReadinessCheckId::ModelSupported,
            model_supported,
            device.model_identifier.clone(),
        ),
        check(
            ReadinessCheckId::OsVersionSupported,
            os_supported,
            device.os_version.clone(),
        ),
        check(
            ReadinessCheckId::NormalMode,
            Some(device.mode == DeviceMode::Normal),
            None,
        ),
        check(
            ReadinessCheckId::PairingTrusted,
            facts.pairing_trusted,
            None,
        ),
        check(ReadinessCheckId::Jailbroken, facts.jailbroken, None),
        check(
            ReadinessCheckId::AppSyncInstalled,
            facts.appsync_installed,
            None,
        ),
        check(ReadinessCheckId::SshAvailable, facts.ssh_available, None),
        ReadinessCheck {
            id: ReadinessCheckId::BatteryLevel,
            status: match device.battery_percent {
                Some(percent) if percent >= MINIMUM_BATTERY_PERCENT => CheckStatus::Pass,
                Some(_) => CheckStatus::Warn,
                None => CheckStatus::Unknown,
            },
            value: device.battery_percent.map(|percent| format!("{percent}%")),
        },
        check(ReadinessCheckId::PhysicalButtons, None, None),
    ];
    let (status, required_workflow) =
        if model_supported == Some(false) || os_supported == Some(false) {
            (ReadinessStatus::Unsupported, None)
        } else if device.mode != DeviceMode::Normal
            || model_supported != Some(true)
            || os_supported != Some(true)
            || facts.pairing_trusted != Some(true)
        {
            (ReadinessStatus::NeedsAttention, None)
        } else if facts.jailbroken == Some(false) {
            (
                ReadinessStatus::NeedsPreparation,
                Some(WorkflowKind::Jailbreak),
            )
        } else if facts.jailbroken == Some(true) && facts.ssh_available == Some(true) {
            match facts.appsync_installed {
                Some(true) => (ReadinessStatus::Ready, None),
                Some(false) => (
                    ReadinessStatus::NeedsPreparation,
                    Some(WorkflowKind::AppSync),
                ),
                None => (ReadinessStatus::NeedsAttention, Some(WorkflowKind::AppSync)),
            }
        } else {
            (ReadinessStatus::NeedsAttention, None)
        };

    ReadinessReport {
        device_id: device.id.clone(),
        status,
        checks,
        required_workflow,
        checked_at: now_millis(),
    }
}

fn check(id: ReadinessCheckId, value: Option<bool>, detail: Option<String>) -> ReadinessCheck {
    ReadinessCheck {
        id,
        status: match value {
            Some(true) => CheckStatus::Pass,
            Some(false) => CheckStatus::Fail,
            None => CheckStatus::Unknown,
        },
        value: detail,
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
            model_identifier: Some("iPod4,1".into()),
            marketing_name: "iPod touch (4th generation)".into(),
            chip: Some("A4".into()),
            board_config: Some("N81AP".into()),
            os_version: Some(os_version.into()),
            build_number: Some("10B500".into()),
            udid_masked: Some("…".into()),
            ecid_masked: Some("…".into()),
            serial_masked: Some("…".into()),
            storage_gb: Some(32),
            battery_percent: Some(80),
            storage_total_bytes: None,
            storage_free_bytes: None,
            mode: DeviceMode::Normal,
            transport: Transport::Usb,
        }
    }

    #[test]
    fn stock_supported_device_needs_jailbreak() {
        let report = evaluate(
            &ipod4("6.1.6"),
            DeviceFacts {
                appsync_installed: Some(true),
                pairing_trusted: Some(true),
                jailbroken: Some(false),
                ssh_available: None,
            },
        );
        assert_eq!(report.status, ReadinessStatus::NeedsPreparation);
        assert_eq!(report.required_workflow, Some(WorkflowKind::Jailbreak));
    }

    #[test]
    fn jailbroken_device_is_ready() {
        let facts = DeviceFacts {
            appsync_installed: Some(true),
            jailbroken: Some(true),
            ssh_available: Some(true),
            pairing_trusted: Some(true),
        };
        let report = evaluate(&ipod4("6.1.6"), facts);
        assert_eq!(report.status, ReadinessStatus::Ready);
        assert!(report.required_workflow.is_none());
    }

    #[test]
    fn missing_and_unknown_appsync_offer_only_appsync_preparation() {
        for (appsync_installed, status) in [
            (Some(false), ReadinessStatus::NeedsPreparation),
            (None, ReadinessStatus::NeedsAttention),
        ] {
            let report = evaluate(
                &ipod4("6.1.6"),
                DeviceFacts {
                    appsync_installed,
                    pairing_trusted: Some(true),
                    jailbroken: Some(true),
                    ssh_available: Some(true),
                },
            );
            assert_eq!(report.status, status);
            assert_eq!(report.required_workflow, Some(WorkflowKind::AppSync));
            assert_eq!(
                report
                    .checks
                    .iter()
                    .find(|c| c.id == ReadinessCheckId::AppSyncInstalled)
                    .unwrap()
                    .status,
                if appsync_installed.is_none() {
                    CheckStatus::Unknown
                } else {
                    CheckStatus::Fail
                }
            );
        }
    }

    #[test]
    fn os_outside_matrix_is_unsupported() {
        let report = evaluate(&ipod4("7.0"), DeviceFacts::default());
        assert_eq!(report.status, ReadinessStatus::Unsupported);
    }

    #[test]
    fn unknown_model_is_unsupported() {
        let mut device = ipod4("6.1.6");
        device.model_identifier = Some("iPhone9,9".into());
        let report = evaluate(&device, DeviceFacts::default());
        assert_eq!(report.status, ReadinessStatus::Unsupported);
    }

    #[test]
    fn unknown_jailbreak_does_not_request_a_destructive_workflow() {
        let facts = DeviceFacts {
            appsync_installed: Some(true),
            pairing_trusted: Some(true),
            ssh_available: Some(true),
            jailbroken: None,
        };
        let report = evaluate(&ipod4("6.1.6"), facts);
        assert_eq!(report.status, ReadinessStatus::NeedsAttention);
        assert_eq!(report.required_workflow, None);
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.id == ReadinessCheckId::Jailbroken)
                .unwrap()
                .status,
            CheckStatus::Unknown
        );
    }

    #[test]
    fn ready_requires_both_paired_access_and_ssh() {
        for facts in [
            DeviceFacts {
                appsync_installed: Some(true),
                jailbroken: Some(true),
                ssh_available: Some(true),
                pairing_trusted: None,
            },
            DeviceFacts {
                appsync_installed: Some(true),
                jailbroken: Some(true),
                ssh_available: Some(false),
                pairing_trusted: Some(true),
            },
        ] {
            assert_eq!(
                evaluate(&ipod4("6.1.6"), facts).status,
                ReadinessStatus::NeedsAttention
            );
        }
    }

    #[test]
    fn missing_information_and_dfu_are_not_misreported_as_unsupported() {
        let mut device = ipod4("6.1.6");
        device.mode = DeviceMode::Dfu;
        device.model_identifier = None;
        device.os_version = None;
        device.battery_percent = None;
        let report = evaluate(&device, DeviceFacts::default());
        assert_eq!(report.status, ReadinessStatus::NeedsAttention);
        assert_eq!(report.required_workflow, None);
        for id in [
            ReadinessCheckId::ModelSupported,
            ReadinessCheckId::OsVersionSupported,
            ReadinessCheckId::BatteryLevel,
        ] {
            assert_eq!(
                report
                    .checks
                    .iter()
                    .find(|check| check.id == id)
                    .unwrap()
                    .status,
                CheckStatus::Unknown
            );
        }
    }
}
