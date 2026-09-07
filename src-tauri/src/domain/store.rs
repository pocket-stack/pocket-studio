//! Version 1 of Pocket Store's signed, static catalog. No I/O lives here.
use std::collections::{BTreeMap, HashMap, HashSet};

use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::catalog::{
    CatalogDetails, CatalogEntry, InstallPolicy, PackageCategory, PackageCompatibility,
};
use super::device::{DeviceFacts, DeviceMode, DeviceSummary, Platform};

pub const MAX_CATALOG_BYTES: usize = 1_000_000;
pub const MAX_BLOB_BYTES: u64 = 512 * 1024 * 1024;
pub const CATALOG_LIFETIME_MS: u64 = 30 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlobRef {
    pub sha256: String,
    pub size_bytes: u64,
    pub content_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Publisher {
    pub id: String,
    pub name: String,
    pub verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AppText {
    pub name: String,
    pub summary: String,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AppSource {
    pub repository: String,
    pub path: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Listing {
    Listed,
    Unlisted,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaRole {
    Icon,
    Screenshot,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Media {
    pub role: MediaRole,
    pub blob: BlobRef,
    #[serde(deserialize_with = "required_option")]
    pub locale: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub target_id: Option<String>,
    pub sort_order: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Application {
    pub id: String,
    pub slug: String,
    pub listing: Listing,
    pub publisher: Publisher,
    pub category: PackageCategory,
    pub source: AppSource,
    pub locales: BTreeMap<String, AppText>,
    pub media: Vec<Media>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OsRequirement {
    pub min: String,
    pub max: String,
    pub builds: Vec<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeDelivery {
    Bundled,
    Shared,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub jailbreak: bool,
    pub appsync: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTarget {
    pub target_id: String,
    pub platform: String,
    pub arch: String,
    pub models: Vec<String>,
    pub os: OsRequirement,
    #[serde(deserialize_with = "required_option")]
    pub host_abi: Option<u64>,
    pub runtime_delivery: RuntimeDelivery,
    pub installer_id: String,
    pub requires: Requirements,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeIdentity {
    pub bundle_id: String,
    pub version: String,
    pub build_number: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub source_commit: String,
    #[serde(deserialize_with = "required_option")]
    pub runtime_commit: Option<String>,
    pub manifest_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    pub format: String,
    pub blob: BlobRef,
    pub build_id: String,
    pub native_identity: NativeIdentity,
    pub provenance: Provenance,
    pub targets: Vec<ArtifactTarget>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseStatus {
    Published,
    Yanked,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub id: String,
    pub app_id: String,
    pub version: String,
    pub revision: u64,
    pub notes: BTreeMap<String, String>,
    pub published_at: u64,
    pub status: ReleaseStatus,
    pub artifacts: Vec<Artifact>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Parent {
    pub publication_id: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub repository_id: String,
    pub channel: String,
    pub publication_id: String,
    pub sequence: u64,
    #[serde(deserialize_with = "required_option")]
    pub parent: Option<Parent>,
    pub created_at: u64,
    pub expires_at: u64,
    pub apps: Vec<Application>,
    pub releases: Vec<Release>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogPointer {
    pub schema_version: u32,
    pub repository_id: String,
    pub channel: String,
    pub publication_id: String,
    pub sequence: u64,
    pub catalog_sha256: String,
    pub key_id: String,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trust {
    pub repository_id: String,
    pub keys: BTreeMap<String, String>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CatalogError {
    #[error("unsupported catalog schema")]
    UnsupportedSchema,
    #[error("invalid catalog data")]
    InvalidCatalog,
    #[error("catalog source or signing key is not trusted")]
    UntrustedSource,
    #[error("catalog checksum does not match")]
    ChecksumMismatch,
    #[error("catalog signature is invalid")]
    InvalidSignature,
    #[error("catalog identity changed or sequence moved backwards")]
    CatalogRollback,
}
impl CatalogError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedSchema => "unsupportedCatalogSchema",
            Self::InvalidCatalog => "invalidCatalog",
            Self::UntrustedSource => "untrustedCatalog",
            Self::ChecksumMismatch => "checksumMismatch",
            Self::InvalidSignature => "invalidCatalogSignature",
            Self::CatalogRollback => "catalogRollback",
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedCatalog {
    catalog: Catalog,
    pointer: CatalogPointer,
}
impl VerifiedCatalog {
    pub fn verify(
        pointer: CatalogPointer,
        bytes: &[u8],
        trust: &Trust,
        minimum_sequence: u64,
    ) -> Result<Self, CatalogError> {
        if pointer.schema_version != 1 {
            return Err(CatalogError::UnsupportedSchema);
        }
        if pointer.repository_id != trust.repository_id {
            return Err(CatalogError::UntrustedSource);
        }
        let encoded = trust
            .keys
            .get(&pointer.key_id)
            .ok_or(CatalogError::UntrustedSource)?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(CatalogError::InvalidCatalog);
        }
        if sha256_hex(bytes) != pointer.catalog_sha256 {
            return Err(CatalogError::ChecksumMismatch);
        }
        let key_bytes: [u8; 32] = STANDARD
            .decode(encoded)
            .map_err(|_| CatalogError::UntrustedSource)?
            .try_into()
            .map_err(|_| CatalogError::UntrustedSource)?;
        let key =
            VerifyingKey::from_bytes(&key_bytes).map_err(|_| CatalogError::UntrustedSource)?;
        let signature = Signature::from_slice(
            &STANDARD
                .decode(&pointer.signature)
                .map_err(|_| CatalogError::InvalidSignature)?,
        )
        .map_err(|_| CatalogError::InvalidSignature)?;
        key.verify_strict(bytes, &signature)
            .map_err(|_| CatalogError::InvalidSignature)?;
        let catalog: Catalog =
            serde_json::from_slice(bytes).map_err(|_| CatalogError::InvalidCatalog)?;
        catalog.validate()?;
        if catalog.repository_id != pointer.repository_id
            || catalog.publication_id != pointer.publication_id
            || catalog.sequence != pointer.sequence
            || catalog.channel != pointer.channel
            || catalog.sequence < minimum_sequence
        {
            return Err(CatalogError::CatalogRollback);
        }
        Ok(Self { catalog, pointer })
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    pub fn pointer(&self) -> &CatalogPointer {
        &self.pointer
    }
    pub fn expired_at(&self, now: u64) -> bool {
        now >= self.catalog.expires_at || now.saturating_add(300_000) < self.catalog.created_at
    }
    pub fn listings(
        &self,
        device: Option<&DeviceSummary>,
        facts: DeviceFacts,
        now: u64,
    ) -> Vec<CatalogEntry> {
        self.catalog
            .apps
            .iter()
            .filter_map(|app| {
                let selection = self.select(&app.id, device, facts, now)?;
                let Selection {
                    release,
                    artifact,
                    target,
                    verdict,
                } = selection;
                Some(CatalogEntry {
                    id: app.id.clone(),
                    version: release.version.clone(),
                    developer: app.publisher.name.clone(),
                    category: app.category,
                    size_bytes: artifact.blob.size_bytes,
                    install_policy: if artifact.format == "ipa" {
                        InstallPolicy::Ipa
                    } else {
                        InstallPolicy::Unsupported
                    },
                    checksum_sha256: artifact.blob.sha256.clone(),
                    signed: true,
                    compatibility: PackageCompatibility {
                        platform: target.platform.clone(),
                        models: target.models.clone(),
                        min_os_version: target.os.min.clone(),
                        max_os_version: target.os.max.clone(),
                        requires_jailbreak: target.requires.jailbreak,
                    },
                    dependencies: vec![],
                    published_at: release.published_at,
                    details: Some(CatalogDetails {
                        app: app.clone(),
                        release_id: release.id.clone(),
                        artifact_id: artifact.id.clone(),
                        target_id: target.target_id.clone(),
                        revision: release.revision,
                        native_identity: artifact.native_identity.clone(),
                        verdict,
                        history: self
                            .catalog
                            .releases
                            .iter()
                            .filter(|r| r.app_id == app.id)
                            .cloned()
                            .collect(),
                    }),
                })
            })
            .collect()
    }
    pub fn select(
        &self,
        app_id: &str,
        device: Option<&DeviceSummary>,
        facts: DeviceFacts,
        now: u64,
    ) -> Option<Selection<'_>> {
        let app = self.catalog.apps.iter().find(|a| a.id == app_id)?;
        let mut releases: Vec<_> = self
            .catalog
            .releases
            .iter()
            .filter(|r| r.app_id == app_id)
            .collect();
        releases.sort_by(|a, b| {
            semver::Version::parse(&b.version)
                .expect("validated version")
                .cmp(&semver::Version::parse(&a.version).expect("validated version"))
                .then_with(|| b.revision.cmp(&a.revision))
        });
        let mut fallback = None;
        let mut needs_preparation = None;
        let mut uncertain = None;
        for release in releases {
            for artifact in &release.artifacts {
                for target in &artifact.targets {
                    let verdict = if app.listing == Listing::Unlisted
                        || release.status == ReleaseStatus::Yanked
                    {
                        StoreVerdict::Withdrawn
                    } else if self.expired_at(now) {
                        StoreVerdict::CatalogExpired
                    } else {
                        evaluate_target(artifact, target, device, facts)
                    };
                    let selection = Selection {
                        release,
                        artifact,
                        target,
                        verdict,
                    };
                    if verdict == StoreVerdict::Compatible {
                        return Some(selection);
                    }
                    if verdict == StoreVerdict::RequiresPreparation && needs_preparation.is_none() {
                        needs_preparation = Some(selection);
                    }
                    if verdict == StoreVerdict::UnknownDevice && uncertain.is_none() {
                        uncertain = Some(selection);
                    }
                    if fallback.is_none() {
                        fallback = Some(selection);
                    }
                }
            }
        }
        needs_preparation.or(uncertain).or(fallback)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Selection<'a> {
    pub release: &'a Release,
    pub artifact: &'a Artifact,
    pub target: &'a ArtifactTarget,
    pub verdict: StoreVerdict,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StoreVerdict {
    Compatible,
    RequiresPreparation,
    UnsupportedModel,
    UnsupportedOs,
    UnsupportedInstaller,
    UnknownDevice,
    NoDevice,
    Withdrawn,
    CatalogExpired,
}

pub fn evaluate_target(
    artifact: &Artifact,
    target: &ArtifactTarget,
    device: Option<&DeviceSummary>,
    facts: DeviceFacts,
) -> StoreVerdict {
    if artifact.format != "ipa"
        || artifact.blob.size_bytes > MAX_BLOB_BYTES
        || target.installer_id != "ios-user-ipa"
        || target.runtime_delivery != RuntimeDelivery::Bundled
        || target.platform != "ios"
        || target.arch != "armv7"
    {
        return StoreVerdict::UnsupportedInstaller;
    }
    let Some(device) = device else {
        return StoreVerdict::NoDevice;
    };
    let (Some(model), Some(os)) = (&device.model_identifier, &device.os_version) else {
        return StoreVerdict::UnknownDevice;
    };
    if device.platform != Platform::Ios || !target.models.contains(model) {
        return StoreVerdict::UnsupportedModel;
    }
    let Some(os) = os_version(os) else {
        return StoreVerdict::UnknownDevice;
    };
    if os < os_version(&target.os.min).expect("validated version")
        || os > os_version(&target.os.max).expect("validated version")
    {
        return StoreVerdict::UnsupportedOs;
    }
    if !target.os.builds.is_empty() {
        let Some(build) = &device.build_number else {
            return StoreVerdict::UnknownDevice;
        };
        if !target.os.builds.contains(build) {
            return StoreVerdict::UnsupportedOs;
        }
    }
    if device.mode != DeviceMode::Normal || facts.pairing_trusted != Some(true) {
        return StoreVerdict::RequiresPreparation;
    }
    if target.requires.jailbreak && facts.jailbroken != Some(true) {
        return StoreVerdict::RequiresPreparation;
    }
    StoreVerdict::Compatible
}

impl Catalog {
    fn validate(&self) -> Result<(), CatalogError> {
        if self.schema_version != 1 {
            return Err(CatalogError::UnsupportedSchema);
        }
        if self.channel != "stable"
            || uuid::Uuid::parse_str(&self.repository_id).is_err()
            || uuid::Uuid::parse_str(&self.publication_id).is_err()
            || self.sequence == 0
            || self.sequence > 9_007_199_254_740_991
            || !self
                .expires_at
                .checked_sub(self.created_at)
                .is_some_and(|d| d > 0 && d <= CATALOG_LIFETIME_MS)
        {
            return Err(CatalogError::InvalidCatalog);
        }
        if self.parent.as_ref().is_some_and(|p| {
            uuid::Uuid::parse_str(&p.publication_id).is_err() || !is_digest(&p.sha256)
        }) {
            return Err(CatalogError::InvalidCatalog);
        }
        let mut apps = HashSet::new();
        let mut slugs = HashSet::new();
        let mut releases = HashSet::new();
        let mut versions = HashSet::new();
        let mut artifacts = HashSet::new();
        let mut owners = HashMap::new();
        let mut blobs = HashMap::new();
        for app in &self.apps {
            if !is_id(&app.id)
                || !is_id(&app.slug)
                || !apps.insert(&app.id)
                || !slugs.insert(&app.slug)
                || !app.locales.contains_key("en")
                || !is_id(&app.publisher.id)
            {
                return Err(CatalogError::InvalidCatalog);
            }
            for media in &app.media {
                validate_blob(&media.blob, &mut blobs)?;
            }
        }
        for release in &self.releases {
            if !apps.contains(&release.app_id)
                || uuid::Uuid::parse_str(&release.id).is_err()
                || !releases.insert(&release.id)
                || !versions.insert((&release.app_id, &release.version, release.revision))
                || semver::Version::parse(&release.version).is_err()
                || release.revision == 0
                || !release.notes.contains_key("en")
                || release.artifacts.is_empty()
            {
                return Err(CatalogError::InvalidCatalog);
            }
            for artifact in &release.artifacts {
                validate_blob(&artifact.blob, &mut blobs)?;
                if uuid::Uuid::parse_str(&artifact.id).is_err()
                    || !artifacts.insert(&artifact.id)
                    || !is_id(&artifact.format)
                    || !is_id(&artifact.native_identity.bundle_id)
                    || artifact.build_id.is_empty()
                    || artifact.targets.is_empty()
                    || !is_digest(&artifact.provenance.manifest_sha256)
                {
                    return Err(CatalogError::InvalidCatalog);
                }
                let mut targets = HashSet::new();
                for target in &artifact.targets {
                    let (Some(min), Some(max)) =
                        (os_version(&target.os.min), os_version(&target.os.max))
                    else {
                        return Err(CatalogError::InvalidCatalog);
                    };
                    if min > max
                        || !is_id(&target.target_id)
                        || !targets.insert(&target.target_id)
                        || target.models.is_empty()
                        || !is_id(&target.platform)
                        || !is_id(&target.arch)
                        || !is_id(&target.installer_id)
                    {
                        return Err(CatalogError::InvalidCatalog);
                    }
                    let key = (&target.platform, &artifact.native_identity.bundle_id);
                    if owners
                        .insert(key, &release.app_id)
                        .is_some_and(|owner| owner != &release.app_id)
                    {
                        return Err(CatalogError::InvalidCatalog);
                    }
                }
            }
        }
        Ok(())
    }
}
fn validate_blob<'a>(
    blob: &'a BlobRef,
    seen: &mut HashMap<&'a str, &'a BlobRef>,
) -> Result<(), CatalogError> {
    if !is_digest(&blob.sha256)
        || blob.size_bytes > 9_007_199_254_740_991
        || blob.content_type.is_empty()
        || blob.content_type.contains(['\r', '\n'])
        || seen
            .insert(&blob.sha256, blob)
            .is_some_and(|prior| prior != blob)
    {
        return Err(CatalogError::InvalidCatalog);
    }
    Ok(())
}
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn is_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn os_version(value: &str) -> Option<[u32; 3]> {
    let mut result = [0; 3];
    let parts: Vec<_> = value.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        result[index] = part.parse().ok()?;
    }
    Some(result)
}
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[derive(Deserialize)]
    struct Fixture {
        catalog_utf8: String,
        pointer: CatalogPointer,
        trust: Trust,
    }
    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../../../contracts/store-v1/fixture-v1.json")).unwrap()
    }
    #[test]
    fn publisher_signature_is_verified_without_reserializing_json() {
        let f = fixture();
        let verified =
            VerifiedCatalog::verify(f.pointer.clone(), f.catalog_utf8.as_bytes(), &f.trust, 0)
                .unwrap();
        assert_eq!(verified.catalog().apps[0].id, "dev.example.notes");
        assert_ne!(
            verified.catalog().apps[0].id,
            verified.catalog().releases[0].artifacts[0]
                .native_identity
                .bundle_id
        );
        assert!(matches!(
            VerifiedCatalog::verify(
                f.pointer.clone(),
                format!("{} ", f.catalog_utf8).as_bytes(),
                &f.trust,
                0
            ),
            Err(CatalogError::ChecksumMismatch)
        ));
        assert!(matches!(
            VerifiedCatalog::verify(f.pointer, f.catalog_utf8.as_bytes(), &f.trust, 2),
            Err(CatalogError::CatalogRollback)
        ));
    }
    #[test]
    fn unknown_keys_and_modified_signatures_fail() {
        let mut f = fixture();
        f.pointer.key_id = "other".into();
        assert!(matches!(
            VerifiedCatalog::verify(f.pointer.clone(), f.catalog_utf8.as_bytes(), &f.trust, 0),
            Err(CatalogError::UntrustedSource)
        ));
        f.pointer.key_id = "fixture".into();
        f.pointer.signature = STANDARD.encode([0u8; 64]);
        assert!(matches!(
            VerifiedCatalog::verify(f.pointer, f.catalog_utf8.as_bytes(), &f.trust, 0),
            Err(CatalogError::InvalidSignature)
        ));
    }
    #[test]
    fn expired_catalogs_remain_readable_but_cannot_resolve_an_install() {
        let f = fixture();
        let verified =
            VerifiedCatalog::verify(f.pointer, f.catalog_utf8.as_bytes(), &f.trust, 0).unwrap();
        let expires = verified.catalog().expires_at;
        assert_eq!(
            verified
                .select("dev.example.notes", None, DeviceFacts::default(), expires)
                .unwrap()
                .verdict,
            StoreVerdict::CatalogExpired
        );
    }
    #[test]
    fn unsupported_formats_are_readable_without_claiming_an_installer() {
        let f = fixture();
        let mut c: Catalog = serde_json::from_str(&f.catalog_utf8).unwrap();
        c.releases[0].artifacts[0].format = "pocket".into();
        c.validate().unwrap();
        let a = &c.releases[0].artifacts[0];
        assert_eq!(
            evaluate_target(a, &a.targets[0], None, DeviceFacts::default()),
            StoreVerdict::UnsupportedInstaller
        );
    }
    fn signed_catalog(edit: impl FnOnce(&mut Catalog)) -> VerifiedCatalog {
        let f = fixture();
        let mut catalog: Catalog = serde_json::from_str(&f.catalog_utf8).unwrap();
        edit(&mut catalog);
        let bytes = serde_json::to_vec(&catalog).unwrap();
        let mut pointer = f.pointer;
        pointer.catalog_sha256 = sha256_hex(&bytes);
        pointer.signature =
            STANDARD.encode(SigningKey::from_bytes(&[1; 32]).sign(&bytes).to_bytes());
        VerifiedCatalog::verify(pointer, &bytes, &f.trust, 0).unwrap()
    }
    fn device() -> DeviceSummary {
        DeviceSummary {
            id: "session".into(),
            platform: Platform::Ios,
            model_identifier: Some("iPod4,1".into()),
            marketing_name: "iPod touch".into(),
            chip: None,
            board_config: None,
            os_version: Some("6.1.6".into()),
            build_number: Some("10B500".into()),
            udid_masked: None,
            ecid_masked: None,
            serial_masked: None,
            storage_gb: None,
            storage_total_bytes: None,
            storage_free_bytes: None,
            battery_percent: None,
            mode: DeviceMode::Normal,
            transport: super::super::device::Transport::Usb,
        }
    }
    fn ready() -> DeviceFacts {
        DeviceFacts {
            pairing_trusted: Some(true),
            jailbroken: Some(true),
            ssh_available: Some(true),
        }
    }
    #[test]
    fn newest_compatible_release_is_selected_and_unknown_device_facts_do_not_pass() {
        let verified = signed_catalog(|catalog| {
            let mut newer = catalog.releases[0].clone();
            newer.id = uuid::Uuid::new_v4().to_string();
            newer.version = "2.0.0".into();
            newer.artifacts[0].id = uuid::Uuid::new_v4().to_string();
            newer.artifacts[0].targets[0].os = OsRequirement {
                min: "7.0.0".into(),
                max: "7.1.2".into(),
                builds: vec![],
            };
            catalog.releases.push(newer);
        });
        let now = verified.catalog().created_at;
        let mut device = device();
        let selected = verified
            .select("dev.example.notes", Some(&device), ready(), now)
            .unwrap();
        assert_eq!(selected.release.version, "1.0.0");
        assert_eq!(selected.verdict, StoreVerdict::Compatible);
        device.build_number = None;
        assert_eq!(
            verified
                .select("dev.example.notes", Some(&device), ready(), now)
                .unwrap()
                .verdict,
            StoreVerdict::UnknownDevice
        );
        let verified = signed_catalog(|_| {});
        assert_eq!(
            verified
                .select("dev.example.notes", Some(&device), ready(), now)
                .unwrap()
                .verdict,
            StoreVerdict::UnknownDevice
        );
        device.build_number = Some("10B500".into());
        let stock = DeviceFacts {
            jailbroken: Some(false),
            ..ready()
        };
        assert_eq!(
            verified
                .select("dev.example.notes", Some(&device), stock, now)
                .unwrap()
                .verdict,
            StoreVerdict::RequiresPreparation
        );
    }
    #[test]
    fn unlisted_apps_keep_their_history_but_cannot_resolve_an_install() {
        let verified = signed_catalog(|catalog| catalog.apps[0].listing = Listing::Unlisted);
        let entries = verified.listings(Some(&device()), ready(), verified.catalog().created_at);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].details.as_ref().unwrap().verdict,
            StoreVerdict::Withdrawn
        );
        assert!(!entries[0].details.as_ref().unwrap().history.is_empty());
    }
}
