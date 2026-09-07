//! Last confirmed package records, never a substitute for a live observation.
use crate::domain::{device::AppSyncObservation, now_millis, store::sha256_hex};
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, sync::Mutex};

pub struct AppSyncObservations(Mutex<Connection>);
impl AppSyncObservations {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        Self::new(Connection::open(path)?)
    }
    pub fn memory() -> Self {
        Self::new(Connection::open_in_memory().expect("in-memory observation database"))
            .expect("observation schema")
    }
    fn new(connection: Connection) -> rusqlite::Result<Self> {
        connection.execute_batch("CREATE TABLE IF NOT EXISTS appsync_observations (device_key TEXT PRIMARY KEY, installed INTEGER NOT NULL CHECK(installed IN (0,1)), observed_at INTEGER NOT NULL)")?;
        Ok(Self(Mutex::new(connection)))
    }
    pub fn observe(&self, key: &str, current: Option<bool>) -> Option<AppSyncObservation> {
        match self.update(key, current) {
            Ok(observation) => observation,
            Err(_) => {
                tracing::warn!("AppSync observation storage unavailable");
                None
            }
        }
    }
    fn update(
        &self,
        key: &str,
        current: Option<bool>,
    ) -> rusqlite::Result<Option<AppSyncObservation>> {
        let db = self
            .0
            .lock()
            .expect("AppSync observation database poisoned");
        let key = sha256_hex(key.as_bytes());
        let previous = db
            .query_row(
                "SELECT installed, observed_at FROM appsync_observations WHERE device_key=?",
                [&key],
                |row| {
                    Ok(AppSyncObservation {
                        installed: row.get(0)?,
                        observed_at: row.get(1)?,
                    })
                },
            )
            .optional()?;
        let now = now_millis();
        if let Some(installed) = current {
            if previous.is_some_and(|old| {
                old.installed == installed
                    && old.observed_at <= now
                    && now - old.observed_at < 60_000
            }) {
                return Ok(previous);
            }
            db.execute("INSERT INTO appsync_observations VALUES(?,?,?) ON CONFLICT(device_key) DO UPDATE SET installed=excluded.installed, observed_at=excluded.observed_at", params![key, installed, now])?;
            return Ok(Some(AppSyncObservation {
                installed,
                observed_at: now,
            }));
        }
        Ok(previous.filter(|old| old.observed_at <= now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_and_failed_reads_preserve_history_without_proving_current_presence() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("observations.sqlite");
        let history = AppSyncObservations::open(&path).unwrap();
        let observed = history.observe("device-a", Some(true)).unwrap();
        drop(history);
        let history = AppSyncObservations::open(&path).unwrap();
        assert_eq!(history.observe("device-a", None), Some(observed));
        assert_eq!(history.observe("device-b", None), None);
        assert!(!history.observe("device-a", Some(false)).unwrap().installed);
        assert!(!history.observe("device-a", None).unwrap().installed);
        let db = history.0.lock().unwrap();
        let key: String = db
            .query_row("SELECT device_key FROM appsync_observations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(key, sha256_hex(b"device-a"));
    }
}
