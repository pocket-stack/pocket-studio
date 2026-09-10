use super::{
    device::{DeviceFacts, DeviceSummary},
    installed::NativeApplication,
    operation::{OperationError, OperationHandle, PlanStep, StepId},
    store::{Artifact, ArtifactTarget},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageAction {
    Install,
    Update,
    Reinstall,
    Uninstall,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequirementState {
    Satisfied,
    Missing,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageRequest {
    pub device_id: String,
    pub app_id: String,
    pub action: PackageAction,
    pub installation_id: Option<String>,
    pub delivery: Option<super::store::RuntimeDelivery>,
    pub format: Option<String>,
    pub delete_data: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageConsent {
    pub plan_id: String,
    pub delete_data: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagePlan {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub app_id: String,
    pub names: BTreeMap<String, String>,
    pub action: PackageAction,
    pub installation: PackageInstallation,
    pub delete_data: bool,
    pub release_id: Option<String>,
    pub artifact: Option<Artifact>,
    pub target: Option<ArtifactTarget>,
    pub version: Option<String>,
    pub revision: Option<u64>,
    pub publication_id: String,
    pub sequence: u64,
    pub catalog_expires_at: u64,
    pub expires_at: u64,
    pub steps: Vec<PlanStep>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "platform",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PackageInstallation {
    Ios {
        bundle_id: String,
        previous: Option<NativeApplication>,
        appsync: RequirementState,
        jailbreak: RequirementState,
    },
    #[serde(rename = "3ds")]
    ThreeDs(Box<super::three_ds::ThreeDsPackagePlan>),
}
impl PackagePlan {
    pub fn bundle_id(&self) -> Option<&str> {
        match &self.installation {
            PackageInstallation::Ios { bundle_id, .. } => Some(bundle_id),
            _ => None,
        }
    }
    pub fn previous_ios(&self) -> Option<&NativeApplication> {
        match &self.installation {
            PackageInstallation::Ios { previous, .. } => previous.as_ref(),
            _ => None,
        }
    }
    pub fn managed(&self) -> Option<&super::three_ds::ThreeDsPackagePlan> {
        match &self.installation {
            PackageInstallation::ThreeDs(plan) => Some(plan),
            _ => None,
        }
    }
    pub fn installation_key(&self) -> String {
        match &self.installation {
            PackageInstallation::Ios { bundle_id, .. } => format!("ios:{bundle_id}"),
            PackageInstallation::ThreeDs(plan) => plan.installation_id.clone(),
        }
    }
    pub fn inspection_keys(&self) -> Vec<String> {
        match self.bundle_id() {
            Some(id) => vec![id.into()],
            None => vec![self.app_id.clone()],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackagePhase {
    Queued,
    Running,
    Verifying,
    Verified,
    Unverified,
    Failed,
    Cancelled,
    Interrupted,
}
impl PackagePhase {
    pub fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Verifying)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJob {
    pub queue_order: u64,
    pub handle: OperationHandle,
    pub plan: PackagePlan,
    pub phase: PackagePhase,
    pub step: StepId,
    pub completed_steps: Vec<StepId>,
    pub percent: u8,
    pub submitted: bool,
    pub cleanup_complete: Option<bool>,
    pub staging_path: Option<String>,
    pub error: Option<OperationError>,
    pub updated_at: u64,
}
#[derive(Clone)]
pub struct PackageObservation {
    pub device: DeviceSummary,
    pub facts: DeviceFacts,
    pub appsync: RequirementState,
    pub applications: Vec<NativeApplication>,
    pub managed: Vec<super::three_ds::ThreeDsInstallation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPackageJob {
    pub binding: String,
    pub repository_id: String,
    pub job: PackageJob,
}

pub fn package_steps(action: PackageAction) -> Vec<PlanStep> {
    let steps: &[StepId] = if action == PackageAction::Uninstall {
        &[StepId::Resolve, StepId::Uninstall, StepId::VerifyInstall]
    } else {
        &[
            StepId::Resolve,
            StepId::Download,
            StepId::Verify,
            StepId::Transfer,
            StepId::Install,
            StepId::VerifyInstall,
        ]
    };
    steps
        .iter()
        .map(|id| PlanStep {
            id: *id,
            cancellable: !matches!(
                id,
                StepId::Install | StepId::Uninstall | StepId::VerifyInstall
            ),
            point_of_no_return: matches!(id, StepId::Install | StepId::Uninstall),
            estimated_seconds: match id {
                StepId::Download => 20,
                StepId::Transfer => 15,
                _ => 5,
            },
            requires_action: None,
        })
        .collect()
}
/// Returns None for an unparseable native version. Callers must never interpret
/// that as an upgrade; unknown builds require an explicit reinstall plan.
pub fn compare_native(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let parse = |value: &str| -> Option<Vec<String>> {
        value
            .split('.')
            .map(|part| {
                if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                    None
                } else {
                    Some(part.trim_start_matches('0').to_owned())
                }
            })
            .collect()
    };
    let left = parse(left)?;
    let right = parse(right)?;
    for i in 0..left.len().max(right.len()) {
        let a = left.get(i).map(String::as_str).unwrap_or("");
        let b = right.get(i).map(String::as_str).unwrap_or("");
        let order = a.len().cmp(&b.len()).then_with(|| a.cmp(b));
        if !order.is_eq() {
            return Some(order);
        }
    }
    Some(std::cmp::Ordering::Equal)
}
pub fn same_installation(
    left: Option<&NativeApplication>,
    right: Option<&NativeApplication>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.bundle_id == b.bundle_id
                && a.product_version == b.product_version
                && a.build_number == b.build_number
                && a.application_type == b.application_type
                && (a.receipt_build_id.is_none()
                    || b.receipt_build_id.is_none()
                    || a.receipt_build_id == b.receipt_build_id)
        }
        _ => false,
    }
}

/// A complete dpkg status file can establish absence. An unreadable/malformed
/// file cannot. Provides: appsync supports compatible package identities.
pub fn appsync_status(bytes: &[u8]) -> RequirementState {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return RequirementState::Unknown;
    };
    if text.is_empty() || !text.ends_with('\n') {
        return RequirementState::Unknown;
    }
    let text = text.replace("\r\n", "\n");
    let mut count = 0;
    let mut found = false;
    let mut unrecognized = false;
    for block in text.split("\n\n").filter(|b| !b.trim().is_empty()) {
        let mut package = None;
        let mut status = None;
        let mut provides = None;
        for line in block.lines() {
            if line.starts_with([' ', '\t']) {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                return RequirementState::Unknown;
            };
            match key {
                "Package" => package = Some(value.trim()),
                "Status" => status = Some(value.trim()),
                "Provides" => provides = Some(value.trim()),
                _ => {}
            }
        }
        let (Some(package), Some(status)) = (package, status) else {
            return RequirementState::Unknown;
        };
        count += 1;
        let supplies = matches!(
            package,
            "ai.akemi.appsyncunified" | "net.angelxwind.appsyncunified" | "appsync"
        ) || provides.is_some_and(|p| {
            p.split(',')
                .any(|p| p.split_whitespace().next() == Some("appsync"))
        });
        if !supplies && package.contains("appsync") {
            unrecognized = true;
        }
        if supplies
            && status.split_whitespace().collect::<Vec<_>>().get(1..)
                == Some(&["ok", "installed"][..])
        {
            found = true;
        }
    }
    if found {
        RequirementState::Satisfied
    } else if unrecognized {
        RequirementState::Unknown
    } else if count > 0 {
        RequirementState::Missing
    } else {
        RequirementState::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    #[test]
    fn native_versions_handle_large_build_numbers_without_lexical_downgrades() {
        assert_eq!(compare_native("10", "9"), Some(Ordering::Greater));
        assert_eq!(compare_native("1.2", "1.2.0"), Some(Ordering::Equal));
        assert_eq!(
            compare_native("9999999999999999999999999999999", "10"),
            Some(Ordering::Greater)
        );
        assert_eq!(compare_native("unknown", "1"), None);
        assert_eq!(compare_native("1..2", "1.2"), None);
    }
    #[test]
    fn dpkg_status_distinguishes_installed_removed_and_unobservable_appsync() {
        assert_eq!(
            appsync_status(b"Package: ai.akemi.appsyncunified\nStatus: install ok installed\n"),
            RequirementState::Satisfied
        );
        assert_eq!(
            appsync_status(b"Package: net.angelxwind.appsyncunified\nStatus: hold ok installed\n"),
            RequirementState::Satisfied
        );
        assert_eq!(appsync_status(b"Package: compatible-package\nStatus: install ok installed\nProvides: appsync, something\n"),RequirementState::Satisfied);
        assert_eq!(
            appsync_status(
                b"Package: ai.akemi.appsyncunified\nStatus: deinstall ok config-files\n"
            ),
            RequirementState::Missing
        );
        assert_eq!(
            appsync_status(b"Package: dpkg\nStatus: install ok installed\n"),
            RequirementState::Missing
        );
        assert_eq!(
            appsync_status(b"Package: some.old.appsync\nStatus: install ok installed\n"),
            RequirementState::Unknown
        );
        for bytes in [
            b"".as_slice(),
            b"Package: truncated",
            b"not a status file\n",
        ] {
            assert_eq!(appsync_status(bytes), RequirementState::Unknown);
        }
    }
}
