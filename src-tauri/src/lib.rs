pub mod application;
mod commands;
pub mod domain;
pub mod infrastructure;

use std::sync::Arc;

use anyhow::Context;
use tauri::Manager;
use tracing_subscriber::prelude::*;

use application::discovery::DeviceDiscovery;
use application::preparation::PreparationService;
use application::{OperationLog, Studio};
use commands::{AppState, TauriSink};
use domain::log::{LogLevel, LogSource};
use infrastructure::legacy_ios::LegacyIosProbe;
use infrastructure::legacy_ios::preparation::LegacyPreparationDriver;

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
                |metadata| metadata.target().starts_with("pocket_studio"),
            )),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    tracing::info!("starting Pocket Studio");

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window
                    .app_handle()
                    .state::<AppState>()
                    .studio
                    .preparation
                    .warn_before_close()
            {
                api.prevent_close();
            }
        })
        .setup(|app| {
            let sink = Arc::new(TauriSink(app.handle().clone()));
            let log = Arc::new(OperationLog::new(sink.clone()));
            let probe = Arc::new(LegacyIosProbe::default());
            let discovery = Arc::new(DeviceDiscovery::new(
                probe.clone(),
                sink.clone(),
                log.clone(),
            ));
            let preparation = Arc::new(PreparationService::new(
                Arc::new(LegacyPreparationDriver::new(
                    probe,
                    app.path().app_cache_dir()?.join("preparation"),
                )),
                sink,
                log.clone(),
            ));
            let studio = Studio::new(discovery.clone(), preparation, log.clone());
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
        .build(tauri::generate_context!())
        .context("failed to build Tauri application")?
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event
                && app
                    .state::<AppState>()
                    .studio
                    .preparation
                    .warn_before_close()
            {
                api.prevent_exit();
            }
        });
    Ok(())
}
