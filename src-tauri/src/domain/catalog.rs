use serde::{Deserialize, Serialize};

use super::device::{DeviceSummary, Platform, compare_versions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageCategory {
    Runtime,
    Tool,
    App,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallPolicy {
    Deb,
    Ipa,
    Bootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageCompatibility {
    pub platform: Platform,
    pub models: Vec<String>,
    pub min_os_version: String,
    pub max_os_version: String,
    pub requires_jailbreak: bool,
}

/// A verified catalog manifest entry. Display text lives in the webview
/// locale files keyed by `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub version: String,
    pub developer: String,
    pub category: PackageCategory,
    pub size_bytes: u64,
    pub install_policy: InstallPolicy,
    pub checksum_sha256: String,
    pub signed: bool,
    pub compatibility: PackageCompatibility,
    pub dependencies: Vec<String>,
    pub published_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub package_id: String,
    pub version: String,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityVerdict {
    Compatible,
    RequiresPreparation,
    UnsupportedModel,
    UnsupportedOs,
}

/// Pure compatibility rule shared by the installer gate and the UI.
pub fn evaluate(
    entry: &CatalogEntry,
    device: &DeviceSummary,
    jailbroken: bool,
) -> CompatibilityVerdict {
    let compat = &entry.compatibility;
    if compat.platform != device.platform || !compat.models.contains(&device.model_identifier) {
        return CompatibilityVerdict::UnsupportedModel;
    }
    if compare_versions(&device.os_version, &compat.min_os_version).is_lt()
        || compare_versions(&device.os_version, &compat.max_os_version).is_gt()
    {
        return CompatibilityVerdict::UnsupportedOs;
    }
    if compat.requires_jailbreak && !jailbroken {
        return CompatibilityVerdict::RequiresPreparation;
    }
    CompatibilityVerdict::Compatible
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::{DeviceMode, Transport};

    fn device(model: &str, os: &str) -> DeviceSummary {
        DeviceSummary {
            id: "d".into(),
            platform: Platform::Ios,
            model_identifier: model.into(),
            marketing_name: String::new(),
            chip: String::new(),
            board_config: String::new(),
            os_version: os.into(),
            build_number: String::new(),
            udid_masked: String::new(),
            ecid_masked: String::new(),
            serial_masked: String::new(),
            storage_gb: 0,
            battery_percent: 100,
            mode: DeviceMode::Normal,
            transport: Transport::Usb,
        }
    }

    fn entry(requires_jailbreak: bool) -> CatalogEntry {
        CatalogEntry {
            id: "p".into(),
            version: "1".into(),
            developer: String::new(),
            category: PackageCategory::Tool,
            size_bytes: 1,
            install_policy: InstallPolicy::Deb,
            checksum_sha256: String::new(),
            signed: true,
            compatibility: PackageCompatibility {
                platform: Platform::Ios,
                models: vec!["iPod4,1".into()],
                min_os_version: "6.0".into(),
                max_os_version: "6.1.6".into(),
                requires_jailbreak,
            },
            dependencies: vec![],
            published_at: 0,
        }
    }

    #[test]
    fn jailbreak_requirement_gates_stock_devices() {
        let stock = device("iPod4,1", "6.1.6");
        assert_eq!(
            evaluate(&entry(true), &stock, false),
            CompatibilityVerdict::RequiresPreparation
        );
        assert_eq!(
            evaluate(&entry(true), &stock, true),
            CompatibilityVerdict::Compatible
        );
        assert_eq!(
            evaluate(&entry(false), &stock, false),
            CompatibilityVerdict::Compatible
        );
    }

    #[test]
    fn model_and_os_are_checked_before_jailbreak() {
        assert_eq!(
            evaluate(&entry(true), &device("iPhone4,1", "6.1.6"), true),
            CompatibilityVerdict::UnsupportedModel
        );
        assert_eq!(
            evaluate(&entry(true), &device("iPod4,1", "5.1.1"), true),
            CompatibilityVerdict::UnsupportedOs
        );
    }
}
