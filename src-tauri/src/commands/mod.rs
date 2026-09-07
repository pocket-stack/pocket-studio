//! Thin Tauri boundary: deserialize arguments, call the application façade,
//! map errors to stable codes, forward domain events to the webview.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::application::store::CatalogSnapshot;
use crate::application::{EventSink, Studio, StudioError};
use crate::domain::device::{DeviceEvent, DiscoverySnapshot};
use crate::domain::installed::InstalledSnapshot;
use crate::domain::log::LogEntry;
use crate::domain::operation::{OperationEvent, OperationHandle};
use crate::domain::preparation::{ConsentRecord, PreparationPlan};
use crate::domain::readiness::ReadinessReport;

pub const DEVICE_EVENT: &str = "studio://device";
pub const OPERATION_EVENT: &str = "studio://operation";
pub const LOG_EVENT: &str = "studio://log";

pub struct AppState {
    pub studio: Arc<Studio>,
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
pub async fn list_devices(state: State<'_, AppState>) -> CommandResult<DiscoverySnapshot> {
    Ok(state.studio.list_devices().await)
}

#[tauri::command]
pub async fn check_readiness(
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<ReadinessReport> {
    Ok(state.studio.check_readiness(&device_id).await?)
}

#[tauri::command]
pub async fn plan_preparation(
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<PreparationPlan> {
    Ok(state.studio.plan_preparation(&device_id).await?)
}

#[tauri::command]
pub async fn start_preparation(
    state: State<'_, AppState>,
    consent: ConsentRecord,
) -> CommandResult<OperationHandle> {
    state
        .studio
        .preparation
        .start(consent)
        .await
        .map_err(|error| StudioError::from(error).into())
}

#[tauri::command]
pub async fn list_catalog(
    state: State<'_, AppState>,
    device_id: Option<String>,
    refresh: Option<bool>,
) -> CommandResult<CatalogSnapshot> {
    state
        .studio
        .store
        .catalog(device_id.as_deref(), refresh.unwrap_or(true))
        .await
        .map_err(|error| StudioError::from(error).into())
}

#[tauri::command]
pub async fn store_media(state: State<'_, AppState>, sha256: String) -> CommandResult<String> {
    state
        .studio
        .store
        .repository
        .media(&sha256)
        .await
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| StudioError::from(error).into())
}

#[tauri::command]
pub async fn list_installed(
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<InstalledSnapshot> {
    state
        .studio
        .installed
        .snapshot(&device_id)
        .await
        .map_err(|error| StudioError::from(error).into())
}

#[tauri::command]
pub async fn install_package(
    state: State<'_, AppState>,
    device_id: String,
    package_id: String,
) -> CommandResult<OperationHandle> {
    let _ = (device_id, package_id);
    state.studio.unavailable_operation().map_err(Into::into)
}

#[tauri::command]
pub async fn cancel_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> CommandResult<()> {
    state
        .studio
        .preparation
        .cancel(&operation_id)
        .map_err(|error| StudioError::from(error).into())
}

#[tauri::command]
pub async fn list_logs(state: State<'_, AppState>) -> CommandResult<Vec<LogEntry>> {
    Ok(state.studio.logs())
}

#[tauri::command]
pub async fn export_logs(state: State<'_, AppState>) -> CommandResult<String> {
    Ok(state.studio.export_logs())
}
