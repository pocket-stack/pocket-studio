//! Read-only integration check for a public catalog and its actual IPA artifacts.
//! No device discovery, installation, pairing or preparation is performed.
use anyhow::{Context, ensure};
use legacy_ios_services::IpaPackage;
use pocket_studio_lib::{
    application::store::{CatalogRepository, DownloadControl},
    domain::{now_millis, store::ReleaseStatus},
    infrastructure::store::{SourceConfig, StaticCatalogRepository, StoreCache},
};
use std::{path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let app_id = args
        .next()
        .context("usage: verify_store APP_ID CACHE_DIRECTORY (with POCKET_STORE_CONFIG)")?;
    let cache = PathBuf::from(
        args.next()
            .context("provide a cache directory for online/offline verification")?,
    );
    ensure!(args.next().is_none(), "unexpected arguments");
    let repository = StaticCatalogRepository::new(
        Arc::new(StoreCache::open(cache)?),
        SourceConfig::from_environment()?,
    )?;
    let read = repository.read(true).await?;
    let verified = read
        .verified
        .context("configure a trusted public store source")?;
    ensure!(!verified.expired_at(now_millis()), "catalog is expired");
    ensure!(
        verified.catalog().apps.iter().any(|app| app.id == app_id),
        "application is absent from the catalog"
    );
    let app = verified
        .catalog()
        .apps
        .iter()
        .find(|app| app.id == app_id)
        .expect("validated application");
    for media in &app.media {
        repository.media(&media.blob.sha256).await?;
    }
    let mut artifacts = Vec::new();
    for release in
        verified.catalog().releases.iter().filter(|release| {
            release.app_id == app_id && release.status == ReleaseStatus::Published
        })
    {
        for artifact in &release.artifacts {
            let downloaded = repository
                .download(&artifact.blob, DownloadControl::default())
                .await?;
            if artifact.format == "ipa" {
                let package = IpaPackage::open(downloaded.path()).await?;
                let native = package.metadata();
                ensure!(
                    native.bundle_id().as_str() == artifact.native_identity.bundle_id
                        && native.product_version() == Some(&artifact.native_identity.version)
                        && native.build_version() == Some(&artifact.native_identity.build_number),
                    "IPA native identity mismatch"
                );
            }
            artifacts.push(serde_json::json!({"artifactId":artifact.id,"version":release.version,"revision":release.revision,"sha256":downloaded.sha256(),"bytes":artifact.blob.size_bytes}));
        }
    }
    ensure!(
        !artifacts.is_empty(),
        "application has no published artifacts"
    );
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"source":read.origin,"issue":read.issue,"repositoryId":verified.catalog().repository_id,"sequence":verified.catalog().sequence,"appId":app_id,"mediaVerified":app.media.len(),"artifacts":artifacts})
        )?
    );
    Ok(())
}
