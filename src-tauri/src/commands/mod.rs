//! Thin Tauri boundary: deserialize arguments, call the application façade,
//! map errors to stable codes, forward domain events to the webview.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::application::{EventSink, Studio, StudioError};
use crate::domain::catalog::{CatalogEntry, InstalledPackage};
use crate::domain::device::{DeviceEvent, DeviceMode, DeviceSummary};
use crate::domain::log::LogEntry;
use crate::domain::operation::{OperationEvent, OperationHandle, StepId};
use crate::domain::preparation::{ConsentRecord, PreparationPlan};
use crate::domain::readiness::ReadinessReport;
use crate::infrastructure::demo::DemoDriver;

pub const DEVICE_EVENT: &str = "studio://device";
pub const OPERATION_EVENT: &str = "studio://operation";
pub const LOG_EVENT: &str = "studio://log";

pub struct AppState {
    pub studio: Arc<Studio>,
    pub demo: Arc<DemoDriver>,
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl From<StudioError> for CommandError {
    fn from(error: StudioError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

type CommandResult<T> = Result<T, CommandError>;

/// Forwards domain events to the main window.
pub struct TauriSink(pub AppHandle);

impl TauriSink {
    fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) {
        if let Err(error) = self.0.emit(event, payload) {
            tracing::warn!(event, %error, "failed to emit event to webview");
        }
    }
}

impl EventSink for TauriSink {
    fn device(&self, event: DeviceEvent) {
        self.emit(DEVICE_EVENT, event);
    }

    fn operation(&self, event: OperationEvent) {
        self.emit(OPERATION_EVENT, event);
    }

    fn log(&self, entry: LogEntry) {
        self.emit(LOG_EVENT, entry);
    }
}

#[tauri::command]
pub async fn list_devices(state: State<'_, AppState>) -> CommandResult<Vec<DeviceSummary>> {
    Ok(state.studio.list_devices())
}

#[tauri::command]
pub async fn check_readiness(
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<ReadinessReport> {
    Ok(state.studio.check_readiness(&device_id)?)
}

#[tauri::command]
pub async fn plan_preparation(
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<PreparationPlan> {
    Ok(state.studio.plan_preparation(&device_id)?)
}

#[tauri::command]
pub async fn start_preparation(
    state: State<'_, AppState>,
    consent: ConsentRecord,
) -> CommandResult<OperationHandle> {
    tracing::info!(plan = %consent.plan_id, "user requested device preparation");
    Ok(state.studio.start_preparation(consent)?)
}

#[tauri::command]
pub async fn list_catalog(state: State<'_, AppState>) -> CommandResult<Vec<CatalogEntry>> {
    Ok(state.studio.catalog())
}

#[tauri::command]
pub async fn list_installed(
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<Vec<InstalledPackage>> {
    Ok(state.studio.installed(&device_id)?)
}

#[tauri::command]
pub async fn install_package(
    state: State<'_, AppState>,
    device_id: String,
    package_id: String,
) -> CommandResult<OperationHandle> {
    tracing::info!(package = %package_id, "user requested package install");
    Ok(state.studio.install(&device_id, &package_id)?)
}

#[tauri::command]
pub async fn cancel_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> CommandResult<()> {
    tracing::info!(operation = %operation_id, "user requested cancellation");
    Ok(state.studio.cancel(&operation_id)?)
}

#[tauri::command]
pub async fn list_logs(state: State<'_, AppState>) -> CommandResult<Vec<LogEntry>> {
    Ok(state.studio.logs())
}

#[tauri::command]
pub async fn export_logs(state: State<'_, AppState>) -> CommandResult<String> {
    Ok(state.studio.export_logs())
}

// --- demo controls (no hardware is involved) ------------------------------

#[tauri::command]
pub async fn demo_attach_device(state: State<'_, AppState>) -> CommandResult<()> {
    state.demo.attach_device();
    Ok(())
}

#[tauri::command]
pub async fn demo_detach_device(state: State<'_, AppState>) -> CommandResult<()> {
    state.demo.detach_device();
    Ok(())
}

#[tauri::command]
pub async fn demo_set_device_mode(
    state: State<'_, AppState>,
    mode: DeviceMode,
) -> CommandResult<()> {
    state.demo.set_device_mode(mode);
    Ok(())
}

#[tauri::command]
pub async fn demo_set_jailbroken(
    state: State<'_, AppState>,
    jailbroken: bool,
) -> CommandResult<()> {
    state.demo.set_jailbroken(jailbroken);
    Ok(())
}

#[tauri::command]
pub async fn demo_fail_next_step(
    state: State<'_, AppState>,
    step_id: Option<StepId>,
) -> CommandResult<()> {
    state.demo.fail_next_step(step_id);
    Ok(())
}
