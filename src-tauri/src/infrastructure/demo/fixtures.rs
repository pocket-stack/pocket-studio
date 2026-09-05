//! Demo data mirrored in `src/shared/gateway/fixtures.ts`.

use crate::domain::catalog::{CatalogEntry, InstallPolicy, PackageCategory, PackageCompatibility};
use crate::domain::device::{DeviceMode, DeviceSummary, Platform, Transport};
use crate::domain::operation::{PlanStep, StepId};

pub fn ipod_touch_4() -> DeviceSummary {
    DeviceSummary {
        id: "usb-ipod4-demo".into(),
        platform: Platform::Ios,
        model_identifier: "iPod4,1".into(),
        marketing_name: "iPod touch (4th generation)".into(),
        chip: "Apple A4 (S5L8930)".into(),
        board_config: "N81AP".into(),
        os_version: "6.1.6".into(),
        build_number: "10B500".into(),
        udid_masked: "1a2b3c…9f0e".into(),
        ecid_masked: "0000…A7F2".into(),
        serial_masked: "C3T…P4".into(),
        storage_gb: 32,
        battery_percent: 78,
        mode: DeviceMode::Normal,
        transport: Transport::Usb,
    }
}

pub fn install_steps() -> Vec<PlanStep> {
    let step = |id, cancellable, point_of_no_return, estimated_seconds| PlanStep {
        id,
        cancellable,
        point_of_no_return,
        estimated_seconds,
        requires_action: None,
    };
    vec![
        step(StepId::Resolve, true, false, 2),
        step(StepId::Download, true, false, 12),
        step(StepId::Verify, true, false, 3),
        step(StepId::Transfer, true, false, 8),
        step(StepId::Install, false, true, 10),
        step(StepId::VerifyInstall, false, false, 3),
    ]
}

fn ios6(requires_jailbreak: bool, min: &str) -> PackageCompatibility {
    PackageCompatibility {
        platform: Platform::Ios,
        models: ["iPod4,1", "iPhone3,1", "iPhone3,3", "iPad1,1"]
            .iter()
            .map(|model| (*model).to_owned())
            .collect(),
        min_os_version: min.into(),
        max_os_version: "6.1.6".into(),
        requires_jailbreak,
    }
}

struct Manifest<'a> {
    id: &'a str,
    version: &'a str,
    developer: &'a str,
    category: PackageCategory,
    size_bytes: u64,
    install_policy: InstallPolicy,
    checksum: &'a str,
    signed: bool,
    compatibility: PackageCompatibility,
    dependencies: &'a [&'a str],
    published_at: u64,
}

impl From<Manifest<'_>> for CatalogEntry {
    fn from(manifest: Manifest<'_>) -> Self {
        Self {
            id: manifest.id.into(),
            version: manifest.version.into(),
            developer: manifest.developer.into(),
            category: manifest.category,
            size_bytes: manifest.size_bytes,
            install_policy: manifest.install_policy,
            checksum_sha256: manifest.checksum.into(),
            signed: manifest.signed,
            compatibility: manifest.compatibility,
            dependencies: manifest
                .dependencies
                .iter()
                .map(|dep| (*dep).to_owned())
                .collect(),
            published_at: manifest.published_at,
        }
    }
}

pub fn catalog() -> Vec<CatalogEntry> {
    use InstallPolicy::{Deb, Ipa};
    use PackageCategory::{App, Game, Runtime, Tool};

    let manifests = [
        Manifest {
            id: "pocket-runtime",
            version: "0.9.3",
            developer: "PocketJS",
            category: Runtime,
            size_bytes: 6_720_000,
            install_policy: Deb,
            checksum: "3f9c…b21e",
            signed: true,
            compatibility: ios6(true, "6.0"),
            dependencies: &[],
            published_at: 1_787_616_000_000,
        },
        Manifest {
            id: "pocket-agent",
            version: "0.4.0",
            developer: "PocketJS",
            category: Tool,
            size_bytes: 1_180_000,
            install_policy: Deb,
            checksum: "9a71…04cd",
            signed: true,
            compatibility: ios6(true, "6.0"),
            dependencies: &["pocket-runtime"],
            published_at: 1_788_220_800_000,
        },
        Manifest {
            id: "openssh",
            version: "6.7p1-13",
            developer: "Community (Cydia/Telesphoreo)",
            category: Tool,
            size_bytes: 2_310_000,
            install_policy: Deb,
            checksum: "c0de…77aa",
            signed: false,
            compatibility: ios6(true, "3.1.3"),
            dependencies: &[],
            published_at: 1_709_424_000_000,
        },
        Manifest {
            id: "legacy-ca-bundle",
            version: "2026.08",
            developer: "Community",
            category: Tool,
            size_bytes: 410_000,
            install_policy: Deb,
            checksum: "88ef…1a90",
            signed: true,
            compatibility: ios6(true, "5.0"),
            dependencies: &[],
            published_at: 1_785_974_400_000,
        },
        Manifest {
            id: "pocket-reader",
            version: "1.2.0",
            developer: "PocketJS",
            category: App,
            size_bytes: 14_500_000,
            install_policy: Ipa,
            checksum: "51aa…e3f7",
            signed: true,
            compatibility: ios6(false, "6.0"),
            dependencies: &[],
            published_at: 1_784_505_600_000,
        },
        Manifest {
            id: "pocket-notes",
            version: "0.8.1",
            developer: "PocketJS",
            category: App,
            size_bytes: 9_200_000,
            install_policy: Ipa,
            checksum: "d4d4…9b0c",
            signed: true,
            compatibility: ios6(false, "6.0"),
            dependencies: &["pocket-runtime"],
            published_at: 1_786_579_200_000,
        },
        Manifest {
            id: "pocket-arcade",
            version: "2.1.4",
            developer: "Retro Pocket Collective",
            category: Game,
            size_bytes: 38_000_000,
            install_policy: Deb,
            checksum: "7b7b…c3c3",
            signed: true,
            compatibility: ios6(true, "6.0"),
            dependencies: &["pocket-runtime"],
            published_at: 1_782_950_400_000,
        },
        Manifest {
            id: "pocket-camera-pro",
            version: "3.0.0",
            developer: "PocketJS",
            category: App,
            size_bytes: 22_000_000,
            install_policy: Ipa,
            checksum: "e1e1…5f5f",
            signed: true,
            compatibility: PackageCompatibility {
                platform: Platform::Ios,
                models: vec!["iPhone4,1".into(), "iPhone5,1".into(), "iPod5,1".into()],
                min_os_version: "7.0".into(),
                max_os_version: "9.3.6".into(),
                requires_jailbreak: false,
            },
            dependencies: &[],
            published_at: 1_788_393_600_000,
        },
    ];

    manifests.into_iter().map(CatalogEntry::from).collect()
}
