use super::store::{NativeIdentity, RuntimeDelivery, RuntimeProvides};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsDetails {
    pub region: Option<String>,
    pub firmware_revision: Option<u32>,
    pub firmware: Option<String>,
    pub runtime: RuntimeProvides,
    pub host_abi: u64,
    pub host_app_id: String,
    pub launcher: bool,
    pub busy: bool,
    pub native_management: bool,
    pub hardware_verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuestHealth {
    Untested,
    Accepted,
    Rejected,
}

/// Device-owned installation evidence. Aliases accept the native wire spelling;
/// persisted observations and webview projections use camelCase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsInstallation {
    #[serde(alias = "installation_id")]
    pub installation_id: String,
    #[serde(alias = "app_id")]
    pub app_id: String,
    #[serde(alias = "container_id")]
    pub container_id: String,
    pub generation: u32,
    pub format: String,
    pub delivery: RuntimeDelivery,
    pub installed: bool,
    pub health: GuestHealth,
    pub title: String,
    pub version: String,
    pub revision: Option<u64>,
    #[serde(alias = "build_id")]
    pub build_id: Option<String>,
    #[serde(alias = "guest_sha256")]
    pub guest_sha256: String,
    #[serde(alias = "native_version")]
    pub native_version: Option<String>,
    #[serde(alias = "native_build_id")]
    pub native_build_id: Option<String>,
    #[serde(alias = "native_identity")]
    pub native_identity: Option<NativeIdentity>,
    #[serde(alias = "runtime_id")]
    pub runtime_id: Option<String>,
    #[serde(alias = "runtime_version")]
    pub runtime_version: Option<String>,
    #[serde(alias = "host_abi")]
    pub host_abi: Option<u64>,
    #[serde(default)]
    pub unavailable: bool,
}
impl ThreeDsInstallation {
    pub fn matches_application_artifact(&self, artifact: &super::store::Artifact) -> bool {
        if self.unavailable
            || !self.installed
            || self.build_id.as_deref() != Some(&artifact.build_id)
        {
            return false;
        }
        if artifact.format == "pocket" {
            return self.guest_sha256 == artifact.blob.sha256;
        }
        if artifact.format != self.format {
            return false;
        }
        // A guest restored from an older native package keeps that build's
        // application revision while its current native host stays upgraded.
        match (&self.native_identity, &artifact.native_identity) {
            (
                Some(NativeIdentity::ThreeDsTitle {
                    title_id: current, ..
                }),
                Some(NativeIdentity::ThreeDsTitle {
                    title_id: source, ..
                }),
            ) => current == source,
            (
                Some(NativeIdentity::HomebrewFile {
                    entrypoint: current,
                }),
                Some(NativeIdentity::HomebrewFile { entrypoint: source }),
            ) => current == source,
            _ => false,
        }
    }
    pub fn matches_artifact(&self, artifact: &super::store::Artifact) -> bool {
        if self.unavailable || !self.installed {
            return false;
        }
        if artifact.format == "pocket" {
            return self.guest_sha256 == artifact.blob.sha256;
        }
        self.native_identity == artifact.native_identity
            && self.native_build_id.as_deref() == Some(&artifact.build_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsPackagePlan {
    pub installation_id: String,
    pub delivery: RuntimeDelivery,
    pub format: String,
    pub expected_generation: u32,
    pub previous: Option<ThreeDsInstallation>,
    pub native_identity: Option<NativeIdentity>,
    pub updates_host: bool,
}
impl ThreeDsPackagePlan {
    /// The standalone app whose own runtime will run this `.pocket` guest, if
    /// the plan updates one rather than the launcher.
    pub fn standalone_host(&self) -> Option<&ThreeDsInstallation> {
        if self.delivery == RuntimeDelivery::Bundled && self.format == "pocket" {
            self.previous.as_ref()
        } else {
            None
        }
    }
}

pub fn installation_id(container: &str, app: &str) -> String {
    super::store::sha256_hex(format!("{container}\n{app}").as_bytes())
}
pub fn parse_firmware(value: &str) -> Option<(String, u32)> {
    let value = value.strip_prefix("Ver. ").unwrap_or(value);
    let (version, tail) = value.split_once('-')?;
    super::store::os_version(version)?;
    let revision = tail
        .trim_end_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .ok()?;
    Some((version.into(), revision))
}
pub fn supported_model(value: &str) -> bool {
    matches!(value, "KTR" | "RED" | "JAN")
}
/// Largest `.pocket` guest a host will accept; bigger bundles are not an
/// environment problem the user could prepare their way out of.
pub const MAX_GUEST_BYTES: u64 = 24 * 1024 * 1024;

pub fn evaluate_target(
    artifact: &super::store::Artifact,
    target: &super::store::ArtifactTarget,
    device: Option<&super::device::DeviceSummary>,
    facts: super::device::DeviceFacts,
) -> super::store::StoreVerdict {
    evaluate_target_for(artifact, target, device, facts, None)
}

/// Like [`evaluate_target`], but a `.pocket` guest is judged against the host
/// that will run it: the connected launcher for a shared install, or the
/// standalone application's own runtime when `standalone` is given.
pub fn evaluate_target_for(
    artifact: &super::store::Artifact,
    target: &super::store::ArtifactTarget,
    device: Option<&super::device::DeviceSummary>,
    facts: super::device::DeviceFacts,
    standalone: Option<&ThreeDsInstallation>,
) -> super::store::StoreVerdict {
    use super::store::{RuntimeCapability as C, StoreVerdict as V, os_version};
    if target.arch != "armv6k"
        || !matches!(
            (artifact.format.as_str(), target.installer_id.as_str()),
            ("pocket", "pocket-runtime") | ("cia", "3ds-cia") | ("3dsx", "3dsx-file")
        )
        || artifact.blob.size_bytes > super::store::MAX_BLOB_BYTES
        || artifact.format == "pocket" && artifact.blob.size_bytes > MAX_GUEST_BYTES
    {
        return V::UnsupportedInstaller;
    }
    let Some(device) = device else {
        return V::NoDevice;
    };
    if device.platform != super::device::Platform::ThreeDs {
        return V::UnsupportedModel;
    }
    let Some(model) = device.model_identifier.as_deref() else {
        return V::UnknownDevice;
    };
    if !supported_model(model) || !target.models.iter().any(|m| m == model) {
        return V::UnsupportedModel;
    }
    let Some(os) = device.os_version.as_deref().and_then(os_version) else {
        return V::UnknownDevice;
    };
    if os < os_version(&target.os.min).expect("validated OS")
        || target
            .os
            .max
            .as_deref()
            .is_some_and(|max| os > os_version(max).expect("validated OS"))
    {
        return V::UnsupportedOs;
    }
    let Some(details) = device.three_ds.as_ref() else {
        return V::RequiresPreparation;
    };
    if facts.pairing_trusted != Some(true)
        || matches!(
            target.requires,
            super::store::Requirements::ThreeDs { cfw: true }
        ) && facts.cfw != Some(true)
    {
        return V::RequiresPreparation;
    }
    if !target.os.builds.is_empty()
        && !details
            .firmware
            .as_ref()
            .is_some_and(|value| target.os.builds.contains(value))
    {
        return V::UnsupportedOs;
    }
    if artifact.format == "pocket" {
        let Some(required) = &target.runtime_requirement else {
            return V::UnsupportedInstaller;
        };
        // The connected host writes the guest; the owning host runs it.
        if !details.runtime.capabilities.contains(&C::GuestUpdate) {
            return V::RequiresPreparation;
        }
        let (abi, runtime_id, runtime_version) = match standalone {
            Some(host) => (
                host.host_abi,
                host.runtime_id.as_deref(),
                host.runtime_version.as_deref(),
            ),
            None => (
                Some(details.host_abi),
                Some(details.runtime.id.as_str()),
                Some(details.runtime.version.as_str()),
            ),
        };
        if target.host_abi != abi || runtime_id != Some(required.id.as_str()) {
            return V::RequiresPreparation;
        }
        match (
            runtime_version.and_then(|v| semver::Version::parse(v).ok()),
            semver::Version::parse(&required.min_version),
        ) {
            (Some(actual), Ok(minimum)) if actual >= minimum => {}
            _ => return V::RequiresPreparation,
        }
    } else if !details.launcher || (artifact.format == "cia" && !details.native_management) {
        return V::RequiresPreparation;
    }
    V::Compatible
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn firmware_and_installation_identity_do_not_mix_regions_or_containers() {
        assert_eq!(parse_firmware("11.17.0-50J"), Some(("11.17.0".into(), 50)));
        assert!(parse_firmware("11.17.0").is_none());
        assert_ne!(
            installation_id("launcher", "dev.app"),
            installation_id("cia-000400000ff00000", "dev.app")
        );
        assert!(!supported_model("CTR"));
    }
}
