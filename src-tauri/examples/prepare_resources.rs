//! Opt-in host-only validation of every download, digest and ramdisk patch.
//! Never connects to a device. The built images are temporary and are removed.
use pocket_studio_lib::infrastructure::legacy_ios::preparation_resources::Resources;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("pocket_studio_lib=info")
        .init();
    let cache = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("usage: prepare_resources <cache-directory>"))?;
    let resources = Resources::fetch(&cache).await?;
    let assets = tokio::task::spawn_blocking(move || resources.build()).await??;
    for name in [
        "iBSS",
        "iBEC",
        "DeviceTree",
        "Kernelcache",
        "RestoreRamdisk",
    ] {
        println!(
            "{name}: {} bytes",
            std::fs::metadata(assets.path(name))?.len()
        );
    }
    println!(
        "Validated {} installation archives; no device was accessed",
        assets.packages.len()
    );
    Ok(())
}
