use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogSource {
    Device,
    Preparation,
    Store,
    System,
}

/// A user-visible, exportable log line. `code` is a stable key the webview
/// translates; `message` is the English fallback that also goes to exports.
/// Must never carry full identifiers, credentials or device contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: u64,
    pub level: LogLevel,
    pub source: LogSource,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}
