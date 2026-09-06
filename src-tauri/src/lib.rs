pub mod application;
mod commands;
pub mod domain;
pub mod infrastructure;

use std::sync::Arc;

use anyhow::Context;
use tauri::Manager;
use tracing_subscriber::prelude::*;

use application::discovery::DeviceDiscovery;
use application::{OperationLog, Studio};
use commands::{AppState, TauriSink};
use domain::log::{LogLevel, LogSource};
use infrastructure::legacy_ios::LegacyIosProbe;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pocket_studio_lib=info".into()),
        )
        // The dependency prints entire protocol payloads at debug level,
        // including pairing records. Never let RUST_LOG expose that material.
        .with(
            tracing_subscriber::fmt::layer().with_filter(tracing_subscriber::filter::filter_fn(
                |metadata| !metadata.target().starts_with("idevice"),
            )),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    tracing::info!("starting Pocket Studio");

    tauri::Builder::default()
        .setup(|app| {
            let sink = Arc::new(TauriSink(app.handle().clone()));
            let log = Arc::new(OperationLog::new(sink.clone()));
            let discovery = Arc::new(DeviceDiscovery::new(
                Arc::new(LegacyIosProbe::default()),
                sink,
                log.clone(),
            ));
            let studio = Studio::new(discovery.clone(), log.clone());
            app.manage(AppState {
                studio: Arc::new(studio),
            });
            tauri::async_runtime::spawn(async move { discovery.monitor().await });
            log.record(
                LogLevel::Info,
                LogSource::System,
                "log.system.started",
                "Pocket Studio started",
                None,
                None,
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::check_readiness,
            commands::plan_preparation,
            commands::start_preparation,
            commands::list_catalog,
            commands::list_installed,
            commands::install_package,
            commands::cancel_operation,
            commands::list_logs,
            commands::export_logs,
        ])
        .run(tauri::generate_context!())
        .context("Tauri application exited with an error")
}
