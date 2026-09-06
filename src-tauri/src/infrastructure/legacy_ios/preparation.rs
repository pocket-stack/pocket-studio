//! User-initiated iPod4,1 / iOS 6.1.6 preparation. Secrets, USB identities,
//! privileged SSH and device writes never cross this adapter boundary.
use super::{
    LegacyIosProbe,
    preparation_resources::{BOOT_ARGS, BootAssets, Resources, failure},
};
use crate::{
    application::preparation::{
        PreparationDriver, PreparationError, PreparationFuture, PreparationTarget,
    },
    domain::operation::{OperationErrorCode, StepId},
};
use legacy_ios_core::{
    BoardConfig, ConnectionId, DeviceIdentity, DeviceMode, Ecid, ProductType, Soc, Udid,
};
use legacy_ios_exploits::Limera1n;
use legacy_ios_services::{
    DeviceInspection, HostKeyPolicy, JailbreakStatus, NormalMux, RamdiskSsh, ScpPath, SshPassword,
    SshTarget, SystemMux,
};
use legacy_ios_transport::{DeviceLocator, IbootClient, NusbDeviceLocator, parse_iboot_serial};
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
        let udid = self
            .probe
            .sessions
            .lock()
            .expect("device sessions poisoned")
            .iter()
            .find(|(_, id)| *id == device_id)
            .and_then(|(key, _)| key.strip_prefix("udid:"))
            .map(Udid::new);
        Box::pin(async move {
            let udid = udid.ok_or(PreparationError::DeviceNotFound)?;
            let normal = NormalMux::new(super::platform::current().normal_backend());
            let inspection = inspect(&normal, &udid).await?;
            validate_inspection(&inspection, None)?;
            let usb = timeout(Duration::from_secs(5), NusbDeviceLocator.list())
                .await
                .map_err(|_| PreparationError::DeviceNotFound)?
                .map_err(|_| PreparationError::DeviceNotFound)?;
            let port = usb
                .iter()
                .find(|usb| {
                    usb.mode() == DeviceMode::Normal && usb.serial_number() == Some(udid.as_str())
                })
                .map(|usb| usb.connection_id().clone())
                .ok_or(PreparationError::DeviceNotFound)?;
            Ok(Box::new(Target {
                probe: self.probe.clone(),
                session_id,
                normal,
                udid,
                ecid: inspection.info().ecid(),
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

struct Target {
    probe: Arc<LegacyIosProbe>,
    session_id: String,
    normal: NormalMux,
    udid: Udid,
    ecid: Ecid,
    port: ConnectionId,
    cache: PathBuf,
    resources: Option<Resources>,
    assets: Option<BootAssets>,
    boot: Option<RamdiskBootPreparation>,
    ssh: Option<RamdiskSsh>,
}

impl PreparationTarget for Target {
    fn validate(&self) -> PreparationFuture<'_, ()> {
        Box::pin(async {
            let inspection = inspect(&self.normal, &self.udid).await?;
            if self
                .probe
                .sessions
                .lock()
                .expect("device sessions poisoned")
                .get(&format!("udid:{}", self.udid))
                != Some(&self.session_id)
            {
                return Err(PreparationError::DeviceNotFound);
            }
            validate_inspection(&inspection, Some(self.ecid))
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
                let client = IbootClient::open(Some(self.ecid))
                    .await
                    .map_err(|_| failure(OperationErrorCode::DeviceDisconnected))?;
                let info = client.device_info();
                if client.mode() != DeviceMode::Dfu
                    || info.ecid() != Some(self.ecid)
                    || info.effective_cpid() != 0x8930
                    || info.srtg() != Some("iBoot-574.4")
                {
                    return Err(failure(OperationErrorCode::DeviceChanged));
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
                checked(&ssh, "mount.sh root", OperationErrorCode::WriteFailed).await?;
                let version = ssh
                    .system_version()
                    .await
                    .map_err(|_| failure(OperationErrorCode::VerificationFailed))?;
                let build = ssh
                    .system_build()
                    .await
                    .map_err(|_| failure(OperationErrorCode::VerificationFailed))?;
                if version != "6.1.6" || build != "10B500" {
                    return Err(failure(OperationErrorCode::DeviceChanged));
                }
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
                    if let Ok(inspection) = inspect(&self.normal, &self.udid).await {
                        if inspection.info().ecid() != self.ecid {
                            return Err(failure(OperationErrorCode::DeviceChanged));
                        }
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
            StepId::VerifyJailbreak => loop {
                if let Ok(inspection) = inspect(&self.normal, &self.udid).await {
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
