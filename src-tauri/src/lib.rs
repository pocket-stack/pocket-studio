mod application;
mod commands;
mod domain;
mod infrastructure;

use std::sync::Arc;

use anyhow::Context;
use tauri::Manager;

use application::{OperationLog, Studio};
use commands::{AppState, TauriSink};
use domain::log::{LogLevel, LogSource};
use infrastructure::demo::DemoDriver;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pocket_studio=info".into()),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    tracing::info!("starting Pocket Studio");

    tauri::Builder::default()
        .setup(|app| {
            let sink = Arc::new(TauriSink(app.handle().clone()));
            let log = Arc::new(OperationLog::new(sink.clone()));
            // The demo driver stands in for every port until real USB, tool
            // and catalog adapters exist. Nothing here touches hardware.
            let demo = DemoDriver::new(sink, log.clone());
            let studio = Studio::new(demo.clone(), demo.clone(), demo.clone(), log.clone());
            app.manage(AppState {
                studio: Arc::new(studio),
                demo,
            });
            log.record(
                LogLevel::Info,
                LogSource::System,
                "log.system.started",
                "Pocket Studio started (native demo driver)",
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
            commands::demo_attach_device,
            commands::demo_detach_device,
            commands::demo_set_device_mode,
            commands::demo_set_jailbroken,
            commands::demo_fail_next_step,
        ])
        .run(tauri::generate_context!())
        .context("Tauri application exited with an error")
}
