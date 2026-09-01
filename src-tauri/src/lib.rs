#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pocket_studio=info".into()),
        )
        .try_init()
        .ok();

    tracing::info!("starting Pocket Studio");

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .map_err(Into::into)
}
