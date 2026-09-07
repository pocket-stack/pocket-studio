//! Device, workflow, package and log models plus the pure rules that govern
//! them. Nothing in this module touches USB, the network or the filesystem.

pub mod catalog;
pub mod device;
pub mod installed;
pub mod log;
pub mod operation;
pub mod packages;
pub mod preparation;
pub mod readiness;
pub mod store;

/// Milliseconds since the Unix epoch, the timestamp representation shared
/// with the webview.
pub fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}
