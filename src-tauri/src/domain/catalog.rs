use serde::{Deserialize, Serialize};

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
    Unsupported,
    Cia,
    ThreeDsx,
    Pocket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageCompatibility {
    pub platform: String,
    pub models: Vec<String>,
    pub min_os_version: String,
    pub max_os_version: Option<String>,
    pub requires_jailbreak: bool,
}

/// Display projection for the webview. The signed App/Release/Artifact models
/// in `store` are authoritative; this projection never authorizes installation.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<CatalogDetails>,
}

/// Presentation projection of the selected signed release. It is never used as
/// an installation manifest; execution resolves the authenticated catalog again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDetails {
    pub candidates: Vec<CatalogCandidate>,
    pub app: super::store::Application,
    pub release_id: String,
    pub artifact_id: String,
    pub target_id: String,
    pub revision: u64,
    pub native_identity: Option<super::store::NativeIdentity>,
    pub verdict: super::store::StoreVerdict,
    pub history: Vec<super::store::Release>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub installation_id: String,
    pub managed: Option<super::three_ds::ThreeDsInstallation>,
    pub package_id: String,
    pub version: String,
    pub installed_at: Option<u64>,
    pub native: Option<super::installed::NativeApplication>,
    pub release_id: Option<String>,
    pub artifact_id: Option<String>,
    pub revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCandidate {
    pub delivery: super::store::RuntimeDelivery,
    pub format: String,
    pub version: String,
    pub revision: u64,
    pub release_id: String,
    pub artifact_id: String,
    pub target_id: String,
    pub verdict: super::store::StoreVerdict,
    pub requires_existing_host: bool,
    pub runtime_requirement: Option<super::store::RuntimeRequirement>,
    pub host_abi: Option<u64>,
}
