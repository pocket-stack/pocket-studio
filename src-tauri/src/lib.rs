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
use application::store::StoreService;
use application::{OperationLog, Studio};
use commands::{AppState, TauriSink};
use domain::log::{LogLevel, LogSource};
use infrastructure::legacy_ios::LegacyIosProbe;
use infrastructure::legacy_ios::preparation::LegacyPreparationDriver;
use infrastructure::store::{SourceConfig, StaticCatalogRepository, StoreCache};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pocket_studio_lib=info".into()),
        )
        // Dependencies can print protocol payloads, pairing records and FTP
        // credentials. Never let RUST_LOG expose their wire-level logs.
        .with(
            tracing_subscriber::fmt::layer().with_filter(tracing_subscriber::filter::filter_fn(
                |metadata| metadata.target().starts_with("pocket_studio"),
            )),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    tracing::info!("starting Pocket Studio");

    let app = tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window
                    .app_handle()
                    .try_state::<AppState>()
                    .is_some_and(|state| state.studio.warn_before_close())
            {
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::plan_device_setup,
            commands::execute_device_setup,
            commands::connect_three_ds,
            commands::set_three_ds_address,
            commands::check_readiness,
            commands::check_appsync,
            commands::plan_preparation,
            commands::start_preparation,
            commands::list_catalog,
            commands::store_media,
            commands::list_installed,
            commands::plan_package,
            commands::start_package,
            commands::list_package_operations,
            commands::verify_package_operation,
            commands::cancel_operation,
            commands::list_logs,
            commands::export_logs,
        ])
        .build(tauri::generate_context!())
        .context("failed to build Tauri application")?;
    // Tauri runs setup hooks from a native event callback. Returning an error
    // there panics across the Objective-C boundary on macOS and aborts. Load
    // fallible application state before entering the event loop instead.
    initialize_services(&app).context("failed to initialize Pocket Studio services")?;
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event
            && app.state::<AppState>().studio.warn_before_close()
        {
            api.prevent_exit();
        }
    });
    Ok(())
}

fn initialize_services(app: &tauri::App) -> anyhow::Result<()> {
    let sink = Arc::new(TauriSink(app.handle().clone()));
    let log = Arc::new(OperationLog::new(sink.clone()));
    let environment_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&environment_dir)?;
    let probe = Arc::new(LegacyIosProbe::with_observation_path(
        &environment_dir.join("environment.sqlite"),
    )?);
    let three_ds = Arc::new(infrastructure::three_ds::ThreeDsBridge::new(
        environment_dir.join("3ds-pairings.json"),
    )?);
    let discovery = Arc::new(DeviceDiscovery::new(
        Arc::new(infrastructure::platforms::PlatformProbe {
            ios: probe.clone(),
            three_ds: three_ds.clone(),
        }),
        sink.clone(),
        log.clone(),
    ));
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    let preparation = Arc::new(PreparationService::new(
        Arc::new(LegacyPreparationDriver::new(
            probe.clone(),
            app.path().app_cache_dir()?.join("preparation"),
        )),
        gate.clone(),
        sink.clone(),
        log.clone(),
    ));
    let cache = Arc::new(StoreCache::open(app.path().app_cache_dir()?.join("store"))?);
    let catalog = Arc::new(StaticCatalogRepository::new(
        cache.clone(),
        Some(SourceConfig::from_environment()?),
    )?);
    let installed = Arc::new(application::installed::InstalledService::new(
        Arc::new(infrastructure::platforms::PlatformInstalled {
            ios: Arc::new(
                infrastructure::legacy_ios::installed::LegacyInstalledReader::new(probe.clone()),
            ),
            three_ds: three_ds.clone(),
        }),
        cache.clone(),
        catalog.clone(),
    ));
    let packages = Arc::new(application::packages::PackageService::new(
        Arc::new(infrastructure::platforms::PlatformPackages {
            ios: Arc::new(infrastructure::legacy_ios::packages::LegacyPackageDriver::new(probe)),
            three_ds: three_ds.clone(),
        }),
        catalog.clone(),
        cache,
        sink,
        log.clone(),
        gate.clone(),
    )?);
    let setup = Arc::new(application::device_setup::SetupService::new(
        Arc::new(infrastructure::three_ds::provisioning::NativeSetup(
            three_ds,
        )),
        catalog.clone(),
        gate,
        log.clone(),
    ));
    let store = Arc::new(StoreService::new(catalog, discovery.clone()));
    let studio = Studio::new(
        discovery.clone(),
        preparation,
        store,
        installed,
        packages,
        log.clone(),
    );
    app.manage(AppState {
        studio: Arc::new(studio),
        setup,
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
}
