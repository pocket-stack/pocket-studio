use super::StoreCache;
use crate::{
    application::packages::{PackageError, PackagePersistence},
    domain::packages::StoredPackageJob,
};
use rusqlite::params;
impl PackagePersistence for StoreCache {
    fn jobs(&self) -> Result<Vec<StoredPackageJob>, PackageError> {
        let db = self.db.lock().map_err(|_| PackageError::Storage)?;
        let mut query = db
            .prepare(
                "SELECT substr(record_json,1,1000001) FROM package_jobs ORDER BY updated_at DESC",
            )
            .map_err(|_| PackageError::Storage)?;
        query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| PackageError::Storage)?
            .map(|row| {
                serde_json::from_str(&row.map_err(|_| PackageError::Storage)?)
                    .map_err(|_| PackageError::Storage)
            })
            .collect()
    }
    fn save_job(&self, job: &StoredPackageJob) -> Result<(), PackageError> {
        let json = serde_json::to_string(job).map_err(|_| PackageError::Storage)?;
        if json.len() > 1_000_000 {
            return Err(PackageError::Storage);
        }
        self.db.lock().map_err(|_|PackageError::Storage)?.execute("INSERT INTO package_jobs(operation_id,record_json,updated_at) VALUES(?,?,?) ON CONFLICT(operation_id) DO UPDATE SET record_json=excluded.record_json,updated_at=excluded.updated_at",params![job.job.handle.operation_id,json,job.job.updated_at]).map_err(|_|PackageError::Storage)?;
        Ok(())
    }
}
