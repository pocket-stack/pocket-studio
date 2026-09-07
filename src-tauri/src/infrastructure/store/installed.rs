use super::StoreCache;
use crate::{
    application::installed::{InstalledError, InstalledPersistence},
    domain::installed::InstallationObservation,
};
use rusqlite::{OptionalExtension, params};
impl InstalledPersistence for StoreCache {
    fn load_observation(
        &self,
        binding: &str,
        repository: &str,
    ) -> Result<Option<InstallationObservation>, InstalledError> {
        let db = self.db.lock().map_err(|_| InstalledError::Storage)?;
        let json:Option<String>=db.query_row("SELECT substr(observation_json,1,1000001) FROM installed_observations WHERE device_key=? AND repository_id=?",params![binding,repository],|row|row.get(0)).optional().map_err(|_|InstalledError::Storage)?;
        json.map(|json| serde_json::from_str(&json).map_err(|_| InstalledError::Storage))
            .transpose()
    }
    fn save_observation(
        &self,
        binding: &str,
        repository: &str,
        observation: &InstallationObservation,
    ) -> Result<(), InstalledError> {
        let json = serde_json::to_string(observation).map_err(|_| InstalledError::Storage)?;
        if json.len() > 1_000_000 {
            return Err(InstalledError::Storage);
        }
        self.db.lock().map_err(|_|InstalledError::Storage)?.execute("INSERT INTO installed_observations(device_key,repository_id,observation_json) VALUES(?,?,?) ON CONFLICT(device_key,repository_id) DO UPDATE SET observation_json=excluded.observation_json",params![binding,repository,json]).map_err(|_|InstalledError::Storage)?;
        Ok(())
    }
}
