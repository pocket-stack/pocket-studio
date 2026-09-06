//! Read-only Legacy-iOS-Kit-rs adapter. Device handles and full identifiers never
//! leave this module. A scan opens existing services but never pairs or writes.

mod platform;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use idevice::{Idevice, IdeviceError, services::lockdown::LockdownClient};
use legacy_ios_assets::DeviceDatabase;
use legacy_ios_core::{BoardConfig, DeviceMode as LegacyMode, ProductType};
use legacy_ios_services::{JailbreakStatus, NormalDevice, NormalMux, ServiceError};
use legacy_ios_transport::{DeviceLocator, NusbDeviceLocator, parse_iboot_serial};
use tokio::time::timeout;

use crate::application::discovery::{DeviceProbe, DeviceRecord, ProbeFuture, ProbeSnapshot};
use crate::domain::device::{
    DeviceFacts, DeviceMode, DeviceSummary, DiscoveryIssue, DiscoveryIssueCode, Platform, Transport,
};

const READ_TIMEOUT: Duration = Duration::from_secs(3);

pub struct LegacyIosProbe {
    normal: NormalMux,
    host: Box<dyn platform::HostEnvironment>,
    sessions: Mutex<HashMap<String, String>>,
}

impl Default for LegacyIosProbe {
    fn default() -> Self {
        let host = platform::current();
        Self {
            normal: NormalMux::new(host.normal_backend()),
            host,
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl DeviceProbe for LegacyIosProbe {
    fn scan(&self) -> ProbeFuture<'_> {
        Box::pin(self.scan_devices())
    }
}

impl LegacyIosProbe {
    async fn scan_devices(&self) -> ProbeSnapshot {
        let (usb, normal) = tokio::join!(
            timeout(READ_TIMEOUT, NusbDeviceLocator.list()),
            timeout(READ_TIMEOUT, self.normal.list_devices()),
        );
        let mut snapshot = ProbeSnapshot::default();
        let usb = match usb {
            Ok(Ok(devices)) => devices,
            _ => {
                snapshot
                    .issues
                    .push(issue(DiscoveryIssueCode::UsbUnavailable, None));
                vec![]
            }
        };
        let normal = match normal {
            Ok(Ok(devices)) => devices,
            _ => {
                snapshot
                    .issues
                    .push(issue(self.host.mux_unavailable(), None));
                vec![]
            }
        };
        let mut seen = HashSet::new();
        let normal_udids: HashSet<_> = normal.iter().map(|device| device.udid().as_str()).collect();
        for device in &normal {
            let key = format!("udid:{}", device.udid());
            seen.insert(key.clone());
            let id = self.session_id(&key);
            let (record, issues) = probe_normal(device, id).await;
            snapshot.records.push(record);
            snapshot.issues.extend(issues);
        }
        for device in usb {
            if device.mode() == LegacyMode::Normal
                && device
                    .serial_number()
                    .is_some_and(|serial| normal_udids.contains(serial))
            {
                continue;
            }
            let key = match (device.mode(), device.serial_number()) {
                (LegacyMode::Normal, Some(udid)) => format!("udid:{udid}"),
                _ => format!("usb:{}", device.connection_id()),
            };
            seen.insert(key.clone());
            let mut summary = empty_summary(self.session_id(&key), mode(device.mode()));
            if let Some(name) = device.product_name() {
                summary.marketing_name = name.to_owned();
            }
            let boot_info = parse_iboot_serial(device.serial_number().unwrap_or_default());
            summary.ecid_masked = boot_info
                .ecid()
                .map(|ecid| mask_identifier(&ecid.to_string()));
            summary.chip = boot_info.cpid().map(|cpid| {
                if cpid == 0x8930 {
                    "A4".into()
                } else {
                    format!("CPID {cpid:04X}")
                }
            });
            if summary.mode == DeviceMode::Normal {
                summary.udid_masked = device.serial_number().map(mask_identifier);
            }
            snapshot.records.push(DeviceRecord {
                summary,
                facts: DeviceFacts::default(),
            });
        }
        self.sessions
            .lock()
            .expect("device sessions poisoned")
            .retain(|key, _| seen.contains(key));
        snapshot
            .records
            .sort_by(|left, right| left.summary.id.cmp(&right.summary.id));
        snapshot
    }

    fn session_id(&self, key: &str) -> String {
        self.sessions
            .lock()
            .expect("device sessions poisoned")
            .entry(key.to_owned())
            .or_insert_with(|| uuid::Uuid::new_v4().to_string())
            .clone()
    }
}

async fn probe_normal(device: &NormalDevice, id: String) -> (DeviceRecord, Vec<DiscoveryIssue>) {
    let mut summary = empty_summary(id, DeviceMode::Normal);
    summary.udid_masked = Some(mask_identifier(device.udid().as_str()));
    let mut facts = DeviceFacts::default();
    let mut issues = vec![];
    let (fallback, inspection) = tokio::join!(
        timeout(READ_TIMEOUT, partial_info(device)),
        device.inspect(),
    );
    if let Ok(Ok(info)) = fallback {
        apply_info(&mut summary, info);
    }
    match inspection {
        Ok(inspection) => {
            let info = inspection.info();
            apply_info(
                &mut summary,
                PartialInfo {
                    product: Some(info.product_type().to_string()),
                    board: Some(format!(
                        "{}AP",
                        info.board_config().as_str().to_ascii_uppercase()
                    )),
                    version: Some(info.product_version().to_owned()),
                    build: Some(info.build_version().to_owned()),
                    ecid: None,
                    serial: inspection.serial_number().map(str::to_owned),
                    battery: inspection.battery_percent(),
                },
            );
            summary.ecid_masked = Some(mask_identifier(&info.ecid().to_string()));
            if let Some(storage) = inspection.storage() {
                summary.storage_gb = u32::try_from(storage.total_bytes() / 1_000_000_000).ok();
                summary.storage_total_bytes = Some(storage.total_bytes());
                summary.storage_free_bytes = Some(storage.free_bytes());
            }
            facts.pairing_trusted = Some(true);
            facts.ssh_available = inspection.ssh_available();
            facts.jailbroken = match inspection.jailbreak() {
                JailbreakStatus::Detected { .. } => Some(true),
                JailbreakStatus::Unknown => None,
            };
        }
        Err(error) => {
            if matches!(
                &error,
                ServiceError::Idevice(
                    IdeviceError::InvalidHostID | IdeviceError::UserDeniedPairing
                )
            ) || matches!(&error, ServiceError::LockdownRejected { code, .. } if code == "InvalidHostID" || code == "UserDeniedPairing")
            {
                facts.pairing_trusted = Some(false);
            }
            let code = match error {
                ServiceError::LegacyTls(_) => DiscoveryIssueCode::PairingSessionFailed,
                ServiceError::SessionTimeout => DiscoveryIssueCode::ProbeTimeout,
                _ => DiscoveryIssueCode::PairingUnavailable,
            };
            issues.push(issue(code, Some(&summary.id)));
        }
    }
    if summary.model_identifier.is_none() || summary.os_version.is_none() {
        issues.push(issue(
            DiscoveryIssueCode::DeviceInfoUnavailable,
            Some(&summary.id),
        ));
    }
    (DeviceRecord { summary, facts }, issues)
}

#[derive(Default)]
struct PartialInfo {
    product: Option<String>,
    board: Option<String>,
    version: Option<String>,
    build: Option<String>,
    ecid: Option<u64>,
    serial: Option<String>,
    battery: Option<u8>,
}

fn apply_info(summary: &mut DeviceSummary, info: PartialInfo) {
    let database = DeviceDatabase::bundled();
    // HardwareModel stays authoritative on modified firmware; upstream also
    // identifies N81AP as iPod4,1 even if ProductType reports an iPhone model.
    let profile = info
        .board
        .as_ref()
        .and_then(|board| {
            database.find_board_config(&BoardConfig::new(
                board.to_ascii_lowercase().trim_end_matches("ap"),
            ))
        })
        .or_else(|| {
            info.product
                .as_ref()
                .and_then(|product| database.find_product(&ProductType::new(product.clone())))
        });
    summary.model_identifier = profile
        .map(|profile| profile.product_type().to_string())
        .or(info.product);
    summary.marketing_name = profile.map_or_else(
        || {
            summary
                .model_identifier
                .clone()
                .unwrap_or_else(|| "Apple USB device".into())
        },
        |profile| profile.name().to_owned(),
    );
    summary.chip = profile.map(|profile| profile.soc().to_string());
    summary.board_config = info.board.map(|board| board.to_ascii_uppercase());
    summary.os_version = info.version;
    summary.build_number = info.build;
    summary.ecid_masked = info.ecid.map(|ecid| mask_identifier(&format!("{ecid:X}")));
    summary.serial_masked = info.serial.as_deref().map(mask_identifier);
    summary.battery_percent = info.battery;
}

async fn partial_info(device: &NormalDevice) -> Result<PartialInfo, ServiceError> {
    let connection = device.connect_port(LockdownClient::LOCKDOWND_PORT).await?;
    let mut client = LockdownClient::new(Idevice::new(Box::new(connection), "pocket-studio"));
    Ok(read_fields(&mut client).await)
}

async fn read_fields(client: &mut LockdownClient) -> PartialInfo {
    // Older lockdownd versions prohibit some individual keys before a trusted
    // session. Query the allowlist independently and retain every known fact.
    let product = string_value(client, "ProductType").await;
    let board = string_value(client, "HardwareModel").await;
    let version = string_value(client, "ProductVersion").await;
    let build = string_value(client, "BuildVersion").await;
    let ecid = client
        .get_value(Some("UniqueChipID"), None)
        .await
        .ok()
        .and_then(|value| value.as_unsigned_integer());
    let serial = string_value(client, "SerialNumber").await;
    let battery = client
        .get_value(
            Some("BatteryCurrentCapacity"),
            Some("com.apple.mobile.battery"),
        )
        .await
        .ok()
        .and_then(|value| value.as_unsigned_integer())
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| *value <= 100);
    PartialInfo {
        product,
        board,
        version,
        build,
        ecid,
        serial,
        battery,
    }
}

async fn string_value(client: &mut LockdownClient, key: &str) -> Option<String> {
    client
        .get_value(Some(key), None)
        .await
        .ok()
        .and_then(|value| value.as_string().map(str::to_owned))
}

fn issue(code: DiscoveryIssueCode, device_id: Option<&str>) -> DiscoveryIssue {
    DiscoveryIssue {
        code,
        device_id: device_id.map(str::to_owned),
    }
}

pub fn mask_identifier(value: &str) -> String {
    let characters: Vec<_> = value.chars().collect();
    if characters.len() < 8 {
        return "••••".into();
    }
    format!(
        "{}…{}",
        characters[..3].iter().collect::<String>(),
        characters[characters.len() - 3..]
            .iter()
            .collect::<String>()
    )
}

fn mode(value: LegacyMode) -> DeviceMode {
    match value {
        LegacyMode::Normal => DeviceMode::Normal,
        LegacyMode::Recovery => DeviceMode::Recovery,
        LegacyMode::Dfu => DeviceMode::Dfu,
        LegacyMode::Wtf => DeviceMode::Wtf,
        LegacyMode::Kis => DeviceMode::Kis,
        LegacyMode::Restore => DeviceMode::Recovery,
        LegacyMode::Ramdisk => DeviceMode::Recovery,
    }
}

fn empty_summary(id: String, mode: DeviceMode) -> DeviceSummary {
    DeviceSummary {
        id,
        platform: Platform::Ios,
        marketing_name: "Apple USB device".into(),
        model_identifier: None,
        chip: None,
        board_config: None,
        os_version: None,
        build_number: None,
        udid_masked: None,
        ecid_masked: None,
        serial_masked: None,
        storage_gb: None,
        storage_total_bytes: None,
        storage_free_bytes: None,
        battery_percent: None,
        mode,
        transport: Transport::Usb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_identity_wins_over_a_modified_firmware_product_type() {
        let mut summary = empty_summary("opaque-session".into(), DeviceMode::Normal);
        apply_info(
            &mut summary,
            PartialInfo {
                product: Some("iPhone3,1".into()),
                board: Some("N81AP".into()),
                version: Some("7.1.2".into()),
                ..PartialInfo::default()
            },
        );
        assert_eq!(summary.model_identifier.as_deref(), Some("iPod4,1"));
        assert_eq!(summary.os_version.as_deref(), Some("7.1.2"));
    }
    #[test]
    fn protected_product_type_does_not_discard_readable_board_and_version() {
        let mut summary = empty_summary("opaque-session".into(), DeviceMode::Normal);
        apply_info(
            &mut summary,
            PartialInfo {
                board: Some("N81AP".into()),
                version: Some("6.1.6".into()),
                build: Some("10B500".into()),
                ..PartialInfo::default()
            },
        );
        assert_eq!(summary.model_identifier.as_deref(), Some("iPod4,1"));
        assert_eq!(summary.os_version.as_deref(), Some("6.1.6"));
        assert_eq!(summary.chip.as_deref(), Some("A4"));
        assert_eq!(summary.serial_masked, None);
        assert_eq!(summary.battery_percent, None);
    }
    #[test]
    fn identifiers_are_masked_even_when_short_or_unicode() {
        for value in [
            "abc",
            "1234567",
            "测试设备",
            "12345678901234567890",
            "测试设备编号非常长",
        ] {
            let masked = mask_identifier(value);
            assert!(!masked.contains(value));
        }
        assert_eq!(mask_identifier("1234567890123"), "123…123");
    }
    #[test]
    fn an_unknown_board_does_not_default_to_the_demo_device() {
        let mut summary = empty_summary("opaque-session".into(), DeviceMode::Dfu);
        apply_info(
            &mut summary,
            PartialInfo {
                board: Some("unrecognized".into()),
                ..PartialInfo::default()
            },
        );
        assert_eq!(summary.model_identifier, None);
        assert_eq!(summary.os_version, None);
    }
}
