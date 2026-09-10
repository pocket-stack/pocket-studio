use super::super::provisioning::SetupError;
use std::{os::unix::fs::MetadataExt, path::Path};
pub fn fingerprint(path: &Path) -> Result<String, SetupError> {
    let m = std::fs::metadata(path).map_err(|_| SetupError::InvalidCard)?;
    Ok(format!("{}:{}", m.dev(), m.ino()))
}
