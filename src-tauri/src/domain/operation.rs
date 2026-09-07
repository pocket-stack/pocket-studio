use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    Preparation,
    Install,
    Uninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepId {
    // preparation
    EnterDfu,
    ExploitBootrom,
    FetchResources,
    BuildRamdisk,
    BootRamdisk,
    MountFilesystem,
    InstallUntether,
    RebootDevice,
    VerifyJailbreak,
    ConnectAppSync,
    InstallAppSync,
    ActivateAppSync,
    VerifyAppSync,
    // install
    Resolve,
    Download,
    Verify,
    Transfer,
    Install,
    VerifyInstall,
    Uninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequiredAction {
    EnterDfu,
    Reconnect,
    TrustComputer,
}

/// One unit of a workflow. `cancellable` and `point_of_no_return` drive what
/// the UI lets the user do while the step runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub id: StepId,
    pub cancellable: bool,
    pub point_of_no_return: bool,
    pub estimated_seconds: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_action: Option<RequiredAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationErrorCode {
    DeviceChanged,
    AlreadyJailbroken,
    RebootTimeout,
    DfuTimeout,
    ExploitFailed,
    DownloadFailed,
    BuildFailed,
    RamdiskBootFailed,
    SshUnavailable,
    WriteFailed,
    DeviceDisconnected,
    VerificationFailed,
    VerificationUnavailable,
    ChecksumMismatch,
    TransferFailed,
    InstallRejected,
    AppSyncInstallFailed,
    AppSyncRestartRequired,
    AppSyncDependencies,
    AppSyncVerificationFailed,
    SshAuthenticationFailed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationError {
    pub code: OperationErrorCode,
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_from_step_id: Option<StepId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<OperationDiagnostic>,
}

/// Fixed diagnostic codes only; never include USB identities or payload data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDiagnostic {
    pub stage: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationHandle {
    pub operation_id: String,
    pub kind: OperationKind,
    pub steps: Vec<PlanStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OperationEvent {
    Started {
        operation_id: String,
        kind: OperationKind,
    },
    StepChanged {
        operation_id: String,
        step_id: StepId,
        status: StepStatus,
    },
    Progress {
        operation_id: String,
        step_id: StepId,
        percent: u8,
    },
    ActionRequired {
        operation_id: String,
        step_id: StepId,
        action: RequiredAction,
    },
    ActionResolved {
        operation_id: String,
        step_id: StepId,
    },
    Finished {
        operation_id: String,
    },
    Failed {
        operation_id: String,
        step_id: StepId,
        error: OperationError,
    },
    Cancelled {
        operation_id: String,
        step_id: StepId,
    },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CancelError {
    #[error("operation not found")]
    UnknownOperation,
    #[error("operation already finished")]
    AlreadyFinished,
    #[error("current step cannot be cancelled safely")]
    NotCancellable,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operation_event_fields_match_the_webview_contract() {
        let value = serde_json::to_value(OperationEvent::StepChanged {
            operation_id: "test".into(),
            step_id: StepId::EnterDfu,
            status: StepStatus::Running,
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({"type":"stepChanged", "operationId":"test", "stepId":"enterDfu", "status":"running"})
        );
    }
}
