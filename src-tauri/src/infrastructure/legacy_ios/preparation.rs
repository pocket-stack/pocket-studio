//! User-initiated iPod4,1 / iOS 6.1.6 preparation. Secrets, USB identities,
//! privileged SSH and device writes never cross this adapter boundary.
use super::{
    LegacyIosProbe, is_ipod4_bootrom,
    preparation_resources::{BOOT_ARGS, BootAssets, Resources, failure},
};
use crate::{
    application::preparation::{
        PreparationDriver, PreparationError, PreparationFuture, PreparationTarget,
    },
    domain::{
        operation::{OperationErrorCode, StepId},
        preparation::PreparationEntryMode,
    },
};
use legacy_ios_core::{
    BoardConfig, ConnectionId, DeviceIdentity, DeviceMode, Ecid, ProductType, Soc, Udid,
};
use legacy_ios_exploits::Limera1n;
use legacy_ios_services::{
    DeviceInspection, HostKeyPolicy, JailbreakStatus, NormalMux, RamdiskSsh, ScpPath, SshPassword,
    SshTarget, SystemMux,
};
use legacy_ios_transport::{
    DeviceLocator, IbootClient, NusbDeviceLocator, RecoveryDeviceInfo, parse_iboot_serial,
};
use legacy_ios_workflows::{
    ExploitPolicy, RamdiskBootPlan, RamdiskBootPreparation, RamdiskBootRequest, boot_ramdisk,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::time::{Instant, timeout};

pub struct LegacyPreparationDriver {
    probe: Arc<LegacyIosProbe>,
    cache: PathBuf,
}
impl LegacyPreparationDriver {
    pub fn new(probe: Arc<LegacyIosProbe>, cache: PathBuf) -> Self {
        Self { probe, cache }
    }
}
impl PreparationDriver for LegacyPreparationDriver {
    fn target(&self, device_id: &str) -> PreparationFuture<'_, Box<dyn PreparationTarget>> {
        let session_id = device_id.to_owned();
        let key = self
            .probe
            .sessions
            .lock()
            .expect("device sessions poisoned")
            .iter()
            .find(|(_, id)| *id == device_id)
            .map(|(key, _)| key.clone());
        Box::pin(async move {
            let key = key.ok_or(PreparationError::DeviceNotFound)?;
            let normal = NormalMux::new(super::platform::current().normal_backend());
            let (entry, ecid, port) = if let Some(udid) = key.strip_prefix("udid:") {
                let udid = Udid::new(udid);
                let inspection = inspect(&normal, &udid).await?;
                validate_inspection(&inspection, None)?;
                let usb = timeout(Duration::from_secs(5), NusbDeviceLocator.list())
                    .await
                    .map_err(|_| PreparationError::DeviceNotFound)?
                    .map_err(|_| PreparationError::DeviceNotFound)?;
                let port = usb
                    .iter()
                    .find(|usb| {
                        usb.mode() == DeviceMode::Normal
                            && usb.serial_number() == Some(udid.as_str())
                    })
                    .map(|usb| usb.connection_id().clone())
                    .ok_or(PreparationError::DeviceNotFound)?;
                (EntryPoint::Normal(udid), inspection.info().ecid(), port)
            } else if let Some(ecid) = key.strip_prefix("dfu:") {
                let ecid = ecid.parse().map_err(|_| PreparationError::Unsupported)?;
                let (port, _) = find_dfu(ecid).await?;
                (EntryPoint::Dfu, ecid, port)
            } else {
                return Err(PreparationError::Unsupported);
            };
            Ok(Box::new(Target {
                probe: self.probe.clone(),
                session_id,
                session_key: key,
                normal,
                entry,
                returned_udid: None,
                ecid,
                port,
                cache: self.cache.clone(),
                resources: None,
                assets: None,
                boot: None,
                ssh: None,
            }) as Box<dyn PreparationTarget>)
        })
    }
}

enum EntryPoint {
    Normal(Udid),
    Dfu,
}

struct Target {
    probe: Arc<LegacyIosProbe>,
    session_id: String,
    normal: NormalMux,
    session_key: String,
    entry: EntryPoint,
    returned_udid: Option<Udid>,
    ecid: Ecid,
    port: ConnectionId,
    cache: PathBuf,
    resources: Option<Resources>,
    assets: Option<BootAssets>,
    boot: Option<RamdiskBootPreparation>,
    ssh: Option<RamdiskSsh>,
}

impl PreparationTarget for Target {
    fn entry_mode(&self) -> PreparationEntryMode {
        match self.entry {
            EntryPoint::Normal(_) => PreparationEntryMode::Normal,
            EntryPoint::Dfu => PreparationEntryMode::Dfu,
        }
    }
    fn validate(&self) -> PreparationFuture<'_, ()> {
        Box::pin(async {
            match &self.entry {
                EntryPoint::Normal(udid) => {
                    validate_inspection(&inspect(&self.normal, udid).await?, Some(self.ecid))?
                }
                EntryPoint::Dfu => {
                    find_dfu(self.ecid).await?;
                }
            }
            if self
                .probe
                .sessions
                .lock()
                .expect("device sessions poisoned")
                .get(&self.session_key)
                != Some(&self.session_id)
            {
                return Err(PreparationError::DeviceNotFound);
            }
            Ok(())
        })
    }
    fn execute(&mut self, step: StepId) -> PreparationFuture<'_, ()> {
        Box::pin(async move {
            let duration = match step {
                StepId::FetchResources => 600,
                StepId::BuildRamdisk => 180,
                StepId::EnterDfu => 180,
                StepId::InstallUntether => 300,
                StepId::VerifyJailbreak => 120,
                _ => 120,
            };
            timeout(Duration::from_secs(duration), self.execute_step(step))
                .await
                .map_err(|_| {
                    failure(match step {
                        StepId::FetchResources => OperationErrorCode::DownloadFailed,
                        StepId::BuildRamdisk => OperationErrorCode::BuildFailed,
                        StepId::EnterDfu => OperationErrorCode::DfuTimeout,
                        StepId::ExploitBootrom => OperationErrorCode::ExploitFailed,
                        StepId::BootRamdisk => OperationErrorCode::RamdiskBootFailed,
                        StepId::MountFilesystem => OperationErrorCode::SshUnavailable,
                        StepId::InstallUntether => OperationErrorCode::WriteFailed,
                        StepId::RebootDevice => OperationErrorCode::RebootTimeout,
                        _ => OperationErrorCode::VerificationFailed,
                    })
                })?
        })
    }
}

impl Target {
    async fn execute_step(&mut self, step: StepId) -> Result<(), PreparationError> {
        match step {
            StepId::FetchResources => {
                self.resources = Some(Resources::fetch(&self.cache).await?);
            }
            StepId::BuildRamdisk => {
                let resources = self
                    .resources
                    .take()
                    .ok_or(failure(OperationErrorCode::BuildFailed))?;
                let assets = tokio::task::spawn_blocking(move || resources.build())
                    .await
                    .map_err(|_| failure(OperationErrorCode::BuildFailed))??;
                let device = DeviceIdentity::new(ProductType::new("iPod4,1"), Soc::A4)
                    .with_board_config(BoardConfig::new("n81"))
                    .with_ecid(self.ecid);
                let request = RamdiskBootRequest {
                    device,
                    ibss: assets.path("iBSS"),
                    ibec: Some(assets.path("iBEC")),
                    ramdisk: Some(assets.path("RestoreRamdisk")),
                    device_tree: assets.path("DeviceTree"),
                    trust_cache: None,
                    kernel: assets.path("Kernelcache"),
                    ticket: None,
                    boot_args: Some(BOOT_ARGS.into()),
                    exploit: ExploitPolicy::AlreadyPwned,
                };
                self.boot = Some(
                    tokio::task::spawn_blocking(move || {
                        let plan = RamdiskBootPlan::resolve(request)
                            .map_err(|_| failure(OperationErrorCode::BuildFailed))?;
                        RamdiskBootPreparation::new(&plan, &plan.confirm_destructive())
                            .map_err(|_| failure(OperationErrorCode::BuildFailed))
                    })
                    .await
                    .map_err(|_| failure(OperationErrorCode::BuildFailed))??,
                );
                self.assets = Some(assets);
                // Downloads may take minutes. Refresh battery, OS and pairing
                // before asking the user to enter DFU.
                self.validate().await?;
            }
            StepId::EnterDfu => self.wait_for_dfu().await?,
            StepId::ExploitBootrom => {
                self.port = find_dfu(self.ecid).await?.0;
                let client = IbootClient::open(Some(self.ecid))
                    .await
                    .map_err(|_| failure(OperationErrorCode::DeviceDisconnected))?;
                let info = client.device_info();
                if client.mode() != DeviceMode::Dfu {
                    return Err(failure(OperationErrorCode::DeviceChanged));
                }
                validate_dfu_info(info, self.ecid)?;
                if info.pwned().is_some() {
                    return Ok(());
                }
                let payload = self
                    .assets
                    .as_ref()
                    .ok_or(failure(OperationErrorCode::BuildFailed))?
                    .payload
                    .clone();
                let pwned = Limera1n::new(payload)
                    .map_err(|_| failure(OperationErrorCode::ExploitFailed))?
                    .exploit(client)
                    .await
                    .map_err(|_| failure(OperationErrorCode::ExploitFailed))?;
                if pwned.device_info().ecid() != Some(self.ecid)
                    || pwned.device_info().pwned().is_none()
                {
                    return Err(failure(OperationErrorCode::ExploitFailed));
                }
            }
            StepId::BootRamdisk => {
                boot_ramdisk(
                    self.boot
                        .as_ref()
                        .ok_or(failure(OperationErrorCode::BuildFailed))?,
                    self.ecid,
                    &mut |_| {},
                )
                .await
                .map_err(|_| failure(OperationErrorCode::RamdiskBootFailed))?;
            }
            StepId::MountFilesystem => {
                let ssh = self.connect_ramdisk().await?;
                checked(&ssh, READ_ONLY_ROOT, OperationErrorCode::VerificationFailed).await?;
                let version = ssh
                    .system_version()
                    .await
                    .map_err(|_| failure(OperationErrorCode::VerificationFailed))?;
                let build = ssh
                    .system_build()
                    .await
                    .map_err(|_| failure(OperationErrorCode::VerificationFailed))?;
                verify_disk_firmware(&version, &build)?;
                let check = ssh.execute("if test -e /mnt1/bin/bash; then exit 42; else test -d /mnt1/System/Library/CoreServices; fi").await.map_err(|_| failure(OperationErrorCode::VerificationFailed))?;
                if check.exit_status() == Some(42) {
                    return Err(PreparationError::AlreadyJailbroken);
                }
                if !check.success() {
                    return Err(failure(OperationErrorCode::VerificationFailed));
                }
                self.ssh = Some(ssh);
            }
            StepId::InstallUntether => self.install().await?,
            StepId::RebootDevice => {
                let ssh = self
                    .ssh
                    .take()
                    .ok_or(failure(OperationErrorCode::SshUnavailable))?;
                checked(&ssh, "sync", OperationErrorCode::WriteFailed).await?;
                // The reply can disappear when reboot succeeds. Only observing
                // the original paired device return constitutes completion.
                let _ = timeout(Duration::from_secs(10), ssh.execute("/sbin/reboot_bak")).await;
                let _ = ssh.disconnect().await;
                loop {
                    if let Some(udid) = self.find_returned_device().await? {
                        self.returned_udid = Some(udid);
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
            StepId::VerifyJailbreak => loop {
                let udid = self
                    .returned_udid
                    .as_ref()
                    .ok_or(failure(OperationErrorCode::DeviceDisconnected))?;
                if let Ok(inspection) = inspect(&self.normal, udid).await {
                    let info = inspection.info();
                    if info.ecid() != self.ecid
                        || info.board_config().as_str() != "n81"
                        || info.product_version() != "6.1.6"
                        || info.build_version() != "10B500"
                    {
                        return Err(failure(OperationErrorCode::DeviceChanged));
                    }
                    if matches!(inspection.jailbreak(), JailbreakStatus::Detected { .. })
                        && inspection.ssh_available() == Some(true)
                    {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            },
            _ => return Err(PreparationError::Unsupported),
        }
        Ok(())
    }

    async fn find_returned_device(&self) -> Result<Option<Udid>, PreparationError> {
        let devices = self
            .normal
            .list_devices()
            .await
            .map_err(|_| failure(OperationErrorCode::DeviceDisconnected))?;
        let mut identities = Vec::new();
        for device in devices {
            if let EntryPoint::Normal(udid) = &self.entry
                && device.udid() != udid
            {
                continue;
            }
            if let Ok(Ok(info)) = timeout(Duration::from_secs(5), device.query_info()).await {
                identities.push((device.udid().clone(), info.ecid()));
            }
        }
        matching_udid(self.ecid, &identities)
    }

    async fn wait_for_dfu(&self) -> Result<(), PreparationError> {
        loop {
            let devices = timeout(Duration::from_secs(5), NusbDeviceLocator.list())
                .await
                .map_err(|_| failure(OperationErrorCode::DeviceDisconnected))?
                .map_err(|_| failure(OperationErrorCode::DeviceDisconnected))?;
            if devices.iter().any(|device| {
                device.mode() == DeviceMode::Dfu
                    && parse_iboot_serial(device.serial_number().unwrap_or_default()).ecid()
                        == Some(self.ecid)
            }) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn connect_ramdisk(&self) -> Result<RamdiskSsh, PreparationError> {
        let mux = SystemMux::default();
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let usb = timeout(Duration::from_secs(5), NusbDeviceLocator.list())
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let serial = usb
                .iter()
                .find(|device| {
                    device.connection_id() == &self.port && device.mode() == DeviceMode::Normal
                })
                .and_then(|device| device.serial_number());
            if let (Some(serial), Ok(devices)) = (serial, mux.list_mux_devices().await)
                && let Some(device) = devices
                    .iter()
                    .find(|device| device.udid().as_str() == serial)
                && let Ok(Ok(ssh)) = timeout(
                    Duration::from_secs(8),
                    RamdiskSsh::connect(
                        &mux,
                        SshTarget::DeviceId(device.id()),
                        "root",
                        &SshPassword::new("alpine"),
                        HostKeyPolicy::AcceptEphemeral,
                    ),
                )
                .await
            {
                // This nonce exists only in the ramdisk we booted over
                // the ECID-bound chain, never on normal iOS.
                let output = ssh
                    .execute("cat /pocket-studio-session")
                    .await
                    .map_err(|_| failure(OperationErrorCode::SshUnavailable))?;
                if !output.success()
                    || output.stdout()
                        != self
                            .assets
                            .as_ref()
                            .ok_or(failure(OperationErrorCode::BuildFailed))?
                            .nonce
                            .as_bytes()
                {
                    return Err(failure(OperationErrorCode::DeviceChanged));
                }
                return Ok(ssh);
            }
            if Instant::now() >= deadline {
                return Err(failure(OperationErrorCode::SshUnavailable));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn install(&self) -> Result<(), PreparationError> {
        let ssh = self
            .ssh
            .as_ref()
            .ok_or(failure(OperationErrorCode::SshUnavailable))?;
        checked(ssh, WRITABLE_ROOT, OperationErrorCode::WriteFailed).await?;
        checked(ssh, "mount.sh pv", OperationErrorCode::WriteFailed).await?;
        let assets = self
            .assets
            .as_ref()
            .ok_or(failure(OperationErrorCode::BuildFailed))?;
        for (name, bytes) in &assets.packages {
            // Names are a closed native list, never IPC input. `&&` preserves
            // extraction failures; cleanup cannot turn them into success.
            let path = format!("/mnt1/pocket-studio-{name}.tar");
            ssh.upload(&ScpPath::new(&path).expect("constant package path"), bytes)
                .await
                .map_err(|_| failure(OperationErrorCode::TransferFailed))?;
            checked(
                ssh,
                &format!("tar -xf {path} -C /mnt1 && rm -f {path}"),
                OperationErrorCode::WriteFailed,
            )
            .await?;
            if *name == "bootstrap" {
                checked(ssh, "rm -f /mnt1/Library/MobileSubstrate/DynamicLibraries/patcyh* /mnt1/private/lib/dpkg/info/com.saurik.patcyh* /mnt1/usr/lib/libpatcyh.dylib", OperationErrorCode::WriteFailed).await?;
            }
        }
        checked(
            ssh,
            "test -d /mnt1/Applications/Cydia.app && test -x /mnt1/usr/sbin/sshd && sync",
            OperationErrorCode::VerificationFailed,
        )
        .await
    }
}

async fn checked(
    ssh: &RamdiskSsh,
    command: &str,
    code: OperationErrorCode,
) -> Result<(), PreparationError> {
    let output = ssh.execute(command).await.map_err(|_| failure(code))?;
    if !output.success() {
        return Err(failure(code));
    }
    Ok(())
}

async fn inspect(normal: &NormalMux, udid: &Udid) -> Result<DeviceInspection, PreparationError> {
    timeout(Duration::from_secs(15), async {
        let device = normal
            .find_device(udid)
            .await
            .map_err(|_| PreparationError::DeviceNotFound)?;
        device
            .inspect()
            .await
            .map_err(|_| PreparationError::DeviceNotReady)
    })
    .await
    .map_err(|_| PreparationError::DeviceNotReady)?
}

fn validate_inspection(
    inspection: &DeviceInspection,
    ecid: Option<Ecid>,
) -> Result<(), PreparationError> {
    let info = inspection.info();
    if ecid.is_some_and(|ecid| ecid != info.ecid()) {
        return Err(failure(OperationErrorCode::DeviceChanged));
    }
    if info.product_type().as_str() != "iPod4,1"
        || info.board_config().as_str() != "n81"
        || info.product_version() != "6.1.6"
        || info.build_version() != "10B500"
    {
        return Err(PreparationError::Unsupported);
    }
    if matches!(inspection.jailbreak(), JailbreakStatus::Detected { .. }) {
        return Err(PreparationError::AlreadyJailbroken);
    }
    if inspection
        .battery_percent()
        .is_none_or(|battery| battery < 50)
    {
        return Err(PreparationError::DeviceNotReady);
    }
    Ok(())
}

// No fsck or writable mounting is requested before reading
// SystemVersion.plist. The two partition layouts follow the pinned mount.sh.
const READ_ONLY_ROOT: &str = "while ! test -b /dev/disk0s1s1 && ! test -b /dev/disk0s1; do sleep 1; done; if test -b /dev/disk0s1s1; then mount_hfs -o rdonly /dev/disk0s1s1 /mnt1; else mount_hfs -o rdonly /dev/disk0s1 /mnt1; fi";
const WRITABLE_ROOT: &str = "umount /mnt1 && if test -b /dev/disk0s1s1; then mount_hfs /dev/disk0s1s1 /mnt1; else mount_hfs /dev/disk0s1 /mnt1; fi";

fn verify_disk_firmware(version: &str, build: &str) -> Result<(), PreparationError> {
    if version != "6.1.6" || build != "10B500" {
        return Err(failure(OperationErrorCode::DeviceChanged));
    }
    Ok(())
}

fn validate_dfu_info(info: &RecoveryDeviceInfo, ecid: Ecid) -> Result<(), PreparationError> {
    if !is_ipod4_bootrom(info)
        || info.ecid() != Some(ecid)
        || ecid.get() == 0
        || info.srtg() != Some("iBoot-574.4")
    {
        return Err(PreparationError::Unsupported);
    }
    Ok(())
}

async fn find_dfu(ecid: Ecid) -> Result<(ConnectionId, RecoveryDeviceInfo), PreparationError> {
    let devices = timeout(Duration::from_secs(5), NusbDeviceLocator.list())
        .await
        .map_err(|_| PreparationError::DeviceNotFound)?
        .map_err(|_| PreparationError::DeviceNotFound)?;
    let mut matches = devices.iter().filter_map(|device| {
        let info = parse_iboot_serial(device.serial_number().unwrap_or_default());
        (device.mode() == DeviceMode::Dfu && info.ecid() == Some(ecid))
            .then_some((device.connection_id().clone(), info))
    });
    let result = matches.next().ok_or(PreparationError::DeviceNotFound)?;
    if matches.next().is_some() {
        return Err(failure(OperationErrorCode::DeviceChanged));
    }
    validate_dfu_info(&result.1, ecid)?;
    Ok(result)
}

fn matching_udid(
    ecid: Ecid,
    identities: &[(Udid, Ecid)],
) -> Result<Option<Udid>, PreparationError> {
    let mut matching = identities
        .iter()
        .filter(|(_, candidate)| *candidate == ecid);
    let first = matching.next().map(|(udid, _)| udid.clone());
    if matching.next().is_some() {
        return Err(failure(OperationErrorCode::DeviceChanged));
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dfu_requires_the_exact_board_rom_and_identity() {
        let ecid = Ecid::new(0x1234);
        for serial in [
            "CPID:8930 BDID:08 ECID:1234 SRTG:[iBoot-574.4]",
            "CPID:8930 BDID:08 ECID:1234 SRTG:[iBoot-574.4] PWND:[limera1n]",
        ] {
            assert!(validate_dfu_info(&parse_iboot_serial(serial), ecid).is_ok());
        }
        for serial in [
            "CPID:8930 BDID:02 ECID:1234 SRTG:[iBoot-574.4]",
            "CPID:8930 BDID:08 ECID:5678 SRTG:[iBoot-574.4]",
            "CPID:8930 BDID:08 SRTG:[iBoot-574.4]",
            "CPID:8930 BDID:08 ECID:1234 SRTG:[iBoot-1430.9.3]",
        ] {
            assert!(validate_dfu_info(&parse_iboot_serial(serial), ecid).is_err());
        }
    }

    #[test]
    fn unknown_dfu_firmware_is_rejected_before_installation() {
        assert!(verify_disk_firmware("6.1.6", "10B500").is_ok());
        for (version, build) in [("5.1.1", "9B206"), ("6.1.6", "unknown"), ("", "")] {
            assert!(verify_disk_firmware(version, build).is_err());
        }
    }

    #[test]
    fn reboot_matches_the_original_ecid_not_the_first_connected_device() {
        let original = Ecid::new(1);
        let other = Ecid::new(2);
        let candidates = [
            (Udid::new("other"), other),
            (Udid::new("original"), original),
        ];
        assert_eq!(
            matching_udid(original, &candidates),
            Ok(Some(Udid::new("original")))
        );
        assert_eq!(matching_udid(original, &candidates[..1]), Ok(None));
        assert!(
            matching_udid(
                original,
                &[(Udid::new("one"), original), (Udid::new("two"), original)]
            )
            .is_err()
        );
    }
}
