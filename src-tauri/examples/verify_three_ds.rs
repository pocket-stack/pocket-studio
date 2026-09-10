//! Read-only integration check against a selected Runtime (including Azahar).
//! Pass an IP:port and a path to its key; never put a key in process arguments.
use pocket_studio_lib::infrastructure::three_ds::{provisioning::parse_token, wire::Connection};
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let address = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("expected IP:port and key-file path"))?
        .parse()?;
    let path = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("expected key-file path"))?;
    let token = parse_token(&std::fs::read(path)?)?;
    let mut connection = Connection::connect(address, &token).await?;
    let info = connection.info().await?;
    let installed = connection.inventory().await?;
    println!(
        "3DS protocol verified: model={}, host ABI={}, installations={}",
        info.model.as_deref().unwrap_or("unknown"),
        info.host.host_abi,
        installed.iter().filter(|record| record.installed).count()
    );
    Ok(())
}
