//! Pinned, digest-verified iPod4,1 / 10B500 inputs. No device I/O.
use legacy_ios_assets::{ResourceCatalog, ResourceId};
use legacy_ios_firmware::{ArtifactSpec, ArtifactStore, FirmwareArchive, FirmwareKeySet};
use legacy_ios_image::{
    HfsImage, Iboot32PatchOptions, decrypt_img3_payload, extract_image_payload,
    patch_iboot32_with_options, replace_image_payload,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{application::preparation::PreparationError, domain::operation::OperationErrorCode};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    firmware_url: String,
    firmware_sha256: String,
    firmware_size: u64,
    resources: Vec<Resource>,
    components: Vec<Component>,
}
#[derive(Deserialize)]
struct Resource {
    name: String,
    url: String,
    sha256: String,
    size: u64,
}
#[derive(Deserialize)]
struct Component {
    name: String,
    path: String,
    sha256: String,
    size: u64,
}

pub struct Resources {
    pub files: HashMap<String, Vec<u8>>,
    pub packages: Vec<(&'static str, Vec<u8>)>,
}

const PACKAGES: &[(&str, &str, bool)] = &[
    ("fstab", "jailbreak-fstab-rw", false),
    ("untether", "jailbreak-aquila-6", false),
    ("bootstrap", "jailbreak-bootstrap-freeze", true),
    ("sshdeb", "jailbreak-sshdeb", false),
    ("openssh", "jailbreak-openssh", true),
    ("openssl", "jailbreak-openssl", true),
    ("lukezgd", "jailbreak-lukezgd", false),
    ("nopatcyh", "jailbreak-nopatcyh", false),
];

pub fn failure(code: OperationErrorCode) -> PreparationError {
    PreparationError::Step(code)
}

impl Resources {
    pub async fn fetch(cache: &Path) -> Result<Self, PreparationError> {
        let manifest: Manifest = serde_json::from_str(include_str!("preparation-assets.json"))
            .expect("bundled preparation manifest");
        let store = ArtifactStore::new(cache);
        let mut files = HashMap::new();
        for resource in manifest.resources {
            let spec = ArtifactSpec::parse(&resource.url, &format!("sha256:{}", resource.sha256))
                .expect("bundled resource digest")
                .with_size(resource.size);
            let path = store.fetch(&spec).await.map_err(|error| {
                tracing::warn!(resource = resource.name, %error, "preparation resource download failed");
                asset_error(error)
            })?;
            let data = tokio::fs::read(path)
                .await
                .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
            verify(&data, &resource.sha256, resource.size)?;
            files.insert(resource.name, data);
        }
        let mut archive = None;
        tokio::fs::create_dir_all(cache)
            .await
            .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
        for component in manifest.components {
            let path = cache.join(format!("{}.component", component.sha256));
            let data = match tokio::fs::read(&path).await {
                Ok(data) => data,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if archive.is_none() {
                        let spec = ArtifactSpec::parse(
                            &manifest.firmware_url,
                            &format!("sha256:{}", manifest.firmware_sha256),
                        )
                        .expect("bundled firmware digest")
                        .with_size(manifest.firmware_size);
                        let path = store.fetch(&spec).await.map_err(asset_error)?;
                        archive = Some(
                            FirmwareArchive::open(path)
                                .map_err(|_| failure(OperationErrorCode::DownloadFailed))?,
                        );
                    }
                    let data = archive.as_ref().expect("opened firmware").read_entry(&component.path).map_err(|error| { tracing::warn!(component = component.name, %error, "firmware component download failed"); failure(OperationErrorCode::DownloadFailed) })?;
                    verify(&data, &component.sha256, component.size)?;
                    let temporary = cache.join(format!("{}.tmp", uuid::Uuid::new_v4()));
                    tokio::fs::write(&temporary, &data)
                        .await
                        .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
                    tokio::fs::rename(temporary, &path)
                        .await
                        .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
                    data
                }
                Err(_) => return Err(failure(OperationErrorCode::DownloadFailed)),
            };
            verify(&data, &component.sha256, component.size)?;
            files.insert(component.name, data);
        }
        let mut packages = Vec::new();
        for &(name, id, compressed) in PACKAGES {
            let record = ResourceCatalog::bundled()
                .get(&ResourceId::new(id))
                .expect("bundled jailbreak resource");
            let spec =
                ArtifactSpec::parse(record.source_url(), &format!("sha256:{}", record.sha256()))
                    .expect("bundled digest")
                    .with_size(record.size());
            let path = store.fetch(&spec).await.map_err(|error| {
                tracing::warn!(resource = id, %error, "jailbreak resource download failed");
                asset_error(error)
            })?;
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|_| failure(OperationErrorCode::DownloadFailed))?;
            verify(&bytes, record.sha256(), record.size())?;
            packages.push((name, if compressed { gunzip(&bytes)? } else { bytes }));
        }
        Ok(Self { files, packages })
    }

    /// Build in an operation-owned temporary directory. Cancellation may drop
    /// the caller while this blocking host task finishes, so the directory is
    /// owned by the task and cannot be reused by another run.
    pub fn build(self) -> Result<BootAssets, PreparationError> {
        let nonce = uuid::Uuid::new_v4().to_string();
        let work = tempfile::tempdir().map_err(build_error)?;
        let keys = FirmwareKeySet::parse(&self.files["keys.json"]).map_err(build_error)?;
        for name in [
            "iBSS",
            "iBEC",
            "DeviceTree",
            "Kernelcache",
            "RestoreRamdisk",
        ] {
            let key = keys
                .key(name)
                .ok_or(failure(OperationErrorCode::BuildFailed))?;
            let encrypted = &self.files[name];
            let decrypted = match (key.key(), key.iv()) {
                (Some(key), Some(iv)) => {
                    decrypt_img3_payload(encrypted, key, iv).map_err(build_error)?
                }
                (None, None) => encrypted.clone(),
                _ => return Err(failure(OperationErrorCode::BuildFailed)),
            };
            let output = match name {
                "iBSS" | "iBEC" => {
                    let payload = extract_image_payload(&decrypted, None).map_err(build_error)?;
                    let patched = patch_iboot32_with_options(
                        &payload,
                        &Iboot32PatchOptions {
                            debug: true,
                            boot_args: Some(BOOT_ARGS.into()),
                            ..Default::default()
                        },
                    )
                    .map_err(build_error)?;
                    if patched == payload {
                        return Err(failure(OperationErrorCode::BuildFailed));
                    }
                    replace_image_payload(&decrypted, &patched, None).map_err(build_error)?
                }
                "RestoreRamdisk" => {
                    let payload = extract_image_payload(&decrypted, None).map_err(build_error)?;
                    let mut hfs = HfsImage::parse(payload).map_err(build_error)?;
                    tracing::debug!("growing ramdisk");
                    hfs.grow(32_000_000).map_err(build_error)?;
                    tracing::info!("adding SpringBoard ramdisk configuration");
                    hfs.untar(&self.files["sbplist.tar"]).map_err(build_error)?;
                    let ssh_tar = gunzip(&self.files["ssh.tar.gz"])?;
                    // The library importer preserves the root metadata and
                    // stock directory symlinks while merging upstream archives.
                    tracing::info!("adding SSH ramdisk payload");
                    hfs.untar(&ssh_tar).map_err(build_error)?;
                    tracing::info!("renaming ramdisk reboot tools");
                    hfs.move_entry("/sbin/reboot", "/sbin/reboot_bak")
                        .map_err(build_error)?;
                    hfs.move_entry("/sbin/halt", "/sbin/halt_bak")
                        .map_err(build_error)?;
                    hfs.add_file("/pocket-studio-session", nonce.as_bytes())
                        .map_err(build_error)?;
                    replace_image_payload(&decrypted, &hfs.into_bytes(), None)
                        .map_err(build_error)?
                }
                _ => decrypted,
            };
            std::fs::write(work.path().join(name), output).map_err(build_error)?;
        }
        let payload =
            legacy_ios_exploits::patch_a4_shellcode(self.files["limera1n-shellcode.bin"].clone())
                .map_err(build_error)?;
        Ok(BootAssets {
            work,
            payload,
            packages: self.packages,
            nonce,
        })
    }
}

pub const BOOT_ARGS: &str =
    "rd=md0 -v amfi=0xff amfi_get_out_of_my_way=1 cs_enforcement_disable=1 pio-error=0";
pub struct BootAssets {
    work: tempfile::TempDir,
    pub payload: Vec<u8>,
    pub packages: Vec<(&'static str, Vec<u8>)>,
    pub nonce: String,
}
impl BootAssets {
    pub fn path(&self, name: &str) -> PathBuf {
        self.work.path().join(name)
    }
}

fn verify(data: &[u8], digest: &str, size: u64) -> Result<(), PreparationError> {
    if data.len() as u64 != size || format!("{:x}", Sha256::digest(data)) != digest {
        return Err(failure(OperationErrorCode::ChecksumMismatch));
    }
    Ok(())
}
fn asset_error(error: legacy_ios_firmware::ArtifactError) -> PreparationError {
    use legacy_ios_firmware::ArtifactError;
    failure(match error {
        ArtifactError::DigestMismatch { .. } => OperationErrorCode::ChecksumMismatch,
        _ => OperationErrorCode::DownloadFailed,
    })
}
fn gunzip(data: &[u8]) -> Result<Vec<u8>, PreparationError> {
    let mut output = Vec::new();
    flate2::read::GzDecoder::new(data)
        .take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut output)
        .map_err(build_error)?;
    if output.len() > 128 * 1024 * 1024 {
        return Err(failure(OperationErrorCode::BuildFailed));
    }
    Ok(output)
}

fn build_error(error: impl std::fmt::Display) -> PreparationError {
    tracing::warn!(%error, "ramdisk construction failed");
    failure(OperationErrorCode::BuildFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_corrupt_or_truncated_cached_inputs() {
        let digest = format!("{:x}", Sha256::digest(b"trusted"));
        assert!(verify(b"trusted", &digest, 7).is_ok());
        assert_eq!(
            verify(b"changed", &digest, 7),
            Err(failure(OperationErrorCode::ChecksumMismatch))
        );
        assert_eq!(
            verify(b"trust", &digest, 7),
            Err(failure(OperationErrorCode::ChecksumMismatch))
        );
        assert!(legacy_ios_exploits::patch_a4_shellcode(vec![0; 368]).is_err());
    }
}
