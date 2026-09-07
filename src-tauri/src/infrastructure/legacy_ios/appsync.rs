//! Explicitly authorized system-package preparation over an identity-bound USB SSH session.
use super::preparation_resources::failure;
use crate::{
    application::preparation::PreparationError,
    domain::{
        operation::OperationErrorCode,
        packages::{RequirementState, appsync_status},
        store::sha256_hex,
    },
};
use legacy_ios_core::Udid;
use legacy_ios_firmware::{ArtifactSpec, ArtifactStore};
use legacy_ios_services::{
    HostKeyPolicy, NormalDevice, RamdiskSsh, ScpPath, SshPassword, SystemMux,
};
use serde::Deserialize;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::time::timeout;

#[derive(Clone, Deserialize)]
pub struct PackageSpec {
    pub label: String,
    pub package: String,
    pub version: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}
pub fn specs() -> Vec<PackageSpec> {
    serde_json::from_str(include_str!("appsync-assets.json"))
        .expect("bundled AppSync package manifest")
}
#[derive(Default)]
pub struct AppSyncSetup {
    assets: Vec<(PackageSpec, Vec<u8>)>,
    pub session: Option<Arc<RamdiskSsh>>,
    staging: Option<String>,
}
impl AppSyncSetup {
    pub async fn fetch(&mut self, cache: &Path) -> Result<(), PreparationError> {
        let store = ArtifactStore::new(cache);
        for spec in specs() {
            let artifact = ArtifactSpec::parse(&spec.url, &format!("sha256:{}", spec.sha256))
                .expect("pinned package source")
                .with_size(spec.size);
            let path = store
                .fetch(&artifact)
                .await
                .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
            let data = tokio::fs::read(path)
                .await
                .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
            if data.len() as u64 != spec.size || sha256_hex(&data) != spec.sha256 {
                return Err(failure(OperationErrorCode::ChecksumMismatch));
            }
            self.assets.push((spec, data));
        }
        Ok(())
    }
    pub async fn connect(
        &mut self,
        device: &NormalDevice,
        password: &SshPassword,
        expected_key: Option<&str>,
    ) -> Result<(), PreparationError> {
        let mux = SystemMux::default();
        let fingerprint = match expected_key {
            Some(key) => key.to_owned(),
            None => host_key(device.udid()).await?,
        };
        let ssh = timeout(
            Duration::from_secs(15),
            RamdiskSsh::connect(
                &mux,
                SshTarget::Udid(device.udid().clone()),
                "root",
                password,
                HostKeyPolicy::Sha256(fingerprint),
            ),
        )
        .await
        .map_err(|_| failure(OperationErrorCode::SshUnavailable))?
        .map_err(|error| match error {
            legacy_ios_services::SshError::AuthenticationRejected => {
                PreparationError::SshAuthenticationFailed
            }
            _ => failure(OperationErrorCode::SshUnavailable),
        })?;
        let version = ssh
            .download(
                &ScpPath::new("/System/Library/CoreServices/SystemVersion.plist")
                    .expect("constant path"),
                65536,
            )
            .await
            .map_err(|_| failure(OperationErrorCode::DeviceChanged))?;
        let value = plist::Value::from_reader(std::io::Cursor::new(version))
            .map_err(|_| failure(OperationErrorCode::DeviceChanged))?;
        let dictionary = value
            .as_dictionary()
            .ok_or(failure(OperationErrorCode::DeviceChanged))?;
        if dictionary
            .get("ProductVersion")
            .and_then(plist::Value::as_string)
            != Some("6.1.6")
            || dictionary
                .get("ProductBuildVersion")
                .and_then(plist::Value::as_string)
                != Some("10B500")
        {
            return Err(failure(OperationErrorCode::DeviceChanged));
        }
        let model = ssh
            .execute("/usr/sbin/sysctl -n hw.machine")
            .await
            .map_err(|_| failure(OperationErrorCode::DeviceChanged))?;
        if !model.success() || model.stdout() != b"iPod4,1\n" {
            return Err(failure(OperationErrorCode::DeviceChanged));
        }
        self.session = Some(Arc::new(ssh));
        Ok(())
    }
    pub async fn install(&mut self) -> Result<(), PreparationError> {
        let ssh = self
            .session
            .as_ref()
            .ok_or(failure(OperationErrorCode::SshUnavailable))?;
        if self.assets.len() != specs().len() {
            return Err(failure(OperationErrorCode::DownloadFailed));
        }
        let stage = format!("/var/root/Library/PocketStudio/{}", uuid::Uuid::new_v4());
        let mkdir =
            format!("umask 077; mkdir -p /var/root/Library/PocketStudio && mkdir '{stage}'");
        if !ssh
            .execute(&mkdir)
            .await
            .map_err(|_| failure(OperationErrorCode::AppSyncInstallFailed))?
            .success()
        {
            return Err(failure(OperationErrorCode::AppSyncInstallFailed));
        }
        self.staging = Some(stage.clone());
        for (spec, data) in &self.assets {
            let path =
                ScpPath::new(format!("{stage}/{}.deb", spec.package)).expect("pinned package path");
            ssh.upload(&path, data)
                .await
                .map_err(|_| failure(OperationErrorCode::TransferFailed))?;
            let actual = ssh
                .download(&path, spec.size)
                .await
                .map_err(|_| failure(OperationErrorCode::ChecksumMismatch))?;
            if sha256_hex(&actual) != spec.sha256 {
                return Err(failure(OperationErrorCode::ChecksumMismatch));
            }
        }
        let result = ssh
            .execute(&install_script(&stage, &specs()))
            .await
            .map_err(|_| failure(OperationErrorCode::AppSyncInstallFailed))?;
        if !result.success() {
            return Err(failure(if matches!(result.exit_status(), Some(71 | 72)) {
                OperationErrorCode::AppSyncDependencies
            } else {
                OperationErrorCode::AppSyncInstallFailed
            }));
        }
        Ok(())
    }
    pub async fn activate(&self) -> Result<(), PreparationError> {
        let ssh = self
            .session
            .as_ref()
            .ok_or(failure(OperationErrorCode::SshUnavailable))?;
        // This is also the service reloading mechanism used by AppSync's postinst.
        if !ssh
            .execute("/bin/launchctl stop com.apple.mobile.installd")
            .await
            .map_err(|_| failure(OperationErrorCode::AppSyncInstallFailed))?
            .success()
        {
            return Err(failure(OperationErrorCode::AppSyncInstallFailed));
        }
        Ok(())
    }
    pub async fn verify(&self) -> Result<Arc<RamdiskSsh>, PreparationError> {
        let ssh = self
            .session
            .as_ref()
            .ok_or(failure(OperationErrorCode::SshUnavailable))?;
        if !ssh
            .execute(&verification_script(&specs()))
            .await
            .is_ok_and(|output| output.success())
        {
            return Err(failure(OperationErrorCode::AppSyncVerificationFailed));
        }
        if let Some(stage) = &self.staging {
            let cleanup = ssh.execute(&format!("rm -rf '{stage}'")).await;
            if !cleanup.is_ok_and(|output| output.success()) {
                tracing::warn!("AppSync installed; owned staging cleanup was incomplete");
            }
        }
        Ok(ssh.clone())
    }
}
use legacy_ios_services::SshTarget;
pub async fn host_key(udid: &Udid) -> Result<String, PreparationError> {
    timeout(
        Duration::from_secs(8),
        RamdiskSsh::host_key_fingerprint(&SystemMux::default(), udid),
    )
    .await
    .map_err(|_| failure(OperationErrorCode::SshUnavailable))?
    .map_err(|_| failure(OperationErrorCode::SshUnavailable))
}
pub async fn read_status(ssh: &RamdiskSsh) -> Option<bool> {
    let bytes = timeout(
        Duration::from_secs(2),
        ssh.download(
            &ScpPath::new("/var/lib/dpkg/status").expect("constant path"),
            4 * 1024 * 1024,
        ),
    )
    .await
    .ok()?
    .ok()?;
    match appsync_status(&bytes) {
        RequirementState::Satisfied => Some(true),
        RequirementState::Missing => Some(false),
        RequirementState::Unknown => None,
    }
}
fn install_script(stage: &str, packages: &[PackageSpec]) -> String {
    let mut script = String::from("set -eu\nexport PATH=/usr/bin:/bin:/usr/sbin:/sbin\nset --\n");
    for p in packages {
        script.push_str(&format!("version=$(dpkg-query -W -f='${{Version}}' '{id}' 2>/dev/null || true)\nstatus=$(dpkg-query -W -f='${{Status}}' '{id}' 2>/dev/null || true)\nif test \"$status\" = 'install ok installed' || test \"$status\" = 'hold ok installed'; then\n  if dpkg --compare-versions \"$version\" ge '{version}'; then :; else set -- \"$@\" '{stage}/{id}.deb'; fi\nelse\n  if test -n \"$version\" && dpkg --compare-versions \"$version\" gt '{version}'; then exit 71; fi\n  set -- \"$@\" '{stage}/{id}.deb'\nfi\n",id=p.package,version=p.version));
    }
    script.push_str(&format!("if test \"$#\" -gt 0; then\n  dpkg --simulate --install \"$@\" >'{stage}/preflight.log' 2>&1 || exit 72\n  dpkg --install \"$@\" >'{stage}/install.log' 2>&1 || exit 73\nfi\n"));
    script
}

fn verification_script(packages: &[PackageSpec]) -> String {
    let mut script = String::from("set -eu\nexport PATH=/usr/bin:/bin:/usr/sbin:/sbin\n");
    for p in packages {
        script.push_str(&format!("status=$(dpkg-query -W -f='${{Status}}' '{id}')\n{{ test \"$status\" = 'install ok installed' || test \"$status\" = 'hold ok installed'; }} || exit 74\nversion=$(dpkg-query -W -f='${{Version}}' '{id}')\ndpkg --compare-versions \"$version\" ge '{version}' || exit 74\n", id=p.package, version=p.version));
    }
    script
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    // Only shell functions run here: neither host dpkg nor device I/O is used.
    fn run_script(
        script: &str,
        status: &str,
        version: &str,
        preflight: &str,
    ) -> std::process::Output {
        let mocks = r#"
query_mock() {
    case "$2" in
        '-f=${Version}') printf '%s' "$TEST_VERSION" ;;
        '-f=${Status}') printf '%s' "$TEST_STATUS" ;;
        *) exit 99 ;;
    esac
}
dpkg() {
    case "$1" in
        --compare-versions)
            case "$3" in ge) test "$2" -ge "$4" ;; gt) test "$2" -gt "$4" ;; *) exit 98 ;; esac ;;
        --simulate) printf 'preflight\n' >> "$TEST_CALLS"; return "$TEST_PREFLIGHT" ;;
        --install) printf 'install\n' >> "$TEST_CALLS" ;;
        *) exit 97 ;;
    esac
}
"#;
        let dir = tempfile::tempdir().unwrap();
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(format!(
                "{mocks}\n{}",
                script.replace("dpkg-query", "query_mock")
            ))
            .env("TEST_VERSION", version)
            .env("TEST_STATUS", status)
            .env("TEST_PREFLIGHT", preflight)
            .env("TEST_CALLS", dir.path().join("calls"))
            .output()
            .unwrap();
        let calls = std::fs::read(dir.path().join("calls")).unwrap_or_default();
        std::process::Output {
            stdout: calls,
            ..output
        }
    }

    fn package() -> PackageSpec {
        let mut package = specs().pop().unwrap();
        // Small integers let the shell mock compare versions without host dpkg.
        package.version = "2".into();
        package
    }

    #[test]
    fn dependency_failure_stops_before_system_write_and_success_runs_preflight_first() {
        let stage = tempfile::tempdir().unwrap();
        let script = install_script(stage.path().to_str().unwrap(), &[package()]);
        let failure = run_script(&script, "", "", "1");
        assert_eq!(failure.status.code(), Some(72));
        assert_eq!(failure.stdout, b"preflight\n");
        let success = run_script(&script, "", "", "0");
        assert!(success.status.success(), "{:?}", success.stderr);
        assert_eq!(success.stdout, b"preflight\ninstall\n");
    }

    #[test]
    fn installed_newer_versions_are_retained_and_broken_newer_versions_are_not_downgraded() {
        let stage = tempfile::tempdir().unwrap();
        let script = install_script(stage.path().to_str().unwrap(), &[package()]);
        for status in ["install ok installed", "hold ok installed"] {
            let result = run_script(&script, status, "3", "0");
            assert!(result.status.success());
            assert!(result.stdout.is_empty());
        }
        let result = run_script(&script, "install ok unpacked", "3", "0");
        assert_eq!(result.status.code(), Some(71));
        assert!(result.stdout.is_empty());
        let result = run_script(&script, "install ok installed", "1", "0");
        assert!(result.status.success());
        assert_eq!(result.stdout, b"preflight\ninstall\n");
    }

    #[test]
    fn verification_requires_configured_packages_at_the_required_version() {
        let script = verification_script(&[package()]);
        assert!(
            run_script(&script, "hold ok installed", "3", "0")
                .status
                .success()
        );
        for (status, version) in [
            ("install ok unpacked", "2"),
            ("install ok installed", "1"),
            ("", ""),
        ] {
            assert!(!run_script(&script, status, version, "0").status.success());
        }
    }
}
