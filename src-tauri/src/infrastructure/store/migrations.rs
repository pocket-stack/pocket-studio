//! One-time migration of the private, unsigned operation journal. Signed
//! catalog bytes and public v1 parsing are deliberately outside this module.
use crate::{application::store::StoreError, domain::packages::StoredPackageJob};
use rusqlite::{Connection, TransactionBehavior, params};
use serde_json::{Map, Value, json};

const VERSION: u32 = 1;
const MAX_RECORD_BYTES: usize = 1_000_000;

pub(super) fn migrate(db: &mut Connection) -> Result<(), StoreError> {
    let transaction = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| StoreError::Storage)?;
    let version: u32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| StoreError::Storage)?;
    if version == VERSION {
        return Ok(());
    }
    if version > VERSION {
        return Err(StoreError::Storage);
    }
    let records = {
        let mut query = transaction
            .prepare(
                "SELECT operation_id,substr(record_json,1,1000001),updated_at FROM package_jobs",
            )
            .map_err(|_| StoreError::Storage)?;
        query
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| StoreError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Storage)?
    };
    let mut migrated = 0;
    for (operation_id, original, updated_at) in records {
        if original.len() > MAX_RECORD_BYTES {
            return Err(StoreError::Storage);
        }
        let mut record: Value = serde_json::from_str(&original).map_err(|_| StoreError::Storage)?;
        let converted = migrate_job(&mut record).and_then(|changed| {
            serde_json::from_value::<StoredPackageJob>(record.clone())
                .map(|parsed| (changed, parsed))
                .map_err(|_| StoreError::Storage)
        });
        let (changed, parsed) = match converted {
            Ok(value) => value,
            Err(error) => {
                // Identifiers only: the record may embed device paths.
                tracing::error!(
                    %operation_id,
                    "a local package journal record cannot be migrated; the store cache must be reset"
                );
                return Err(error);
            }
        };
        if parsed.job.handle.operation_id != operation_id {
            return Err(StoreError::Storage);
        }
        if !changed {
            continue;
        }
        // Keep the original JSON byte-for-byte, including submission evidence.
        // The archive and the rewrite commit together; a bad later row rolls
        // back every earlier rewrite and leaves the schema version unchanged.
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS package_job_migration_backups(
            migration_version INTEGER NOT NULL,operation_id TEXT NOT NULL,
            record_json TEXT NOT NULL,updated_at INTEGER NOT NULL,
            PRIMARY KEY(migration_version,operation_id));",
            )
            .map_err(|_| StoreError::Storage)?;
        transaction
            .execute(
                "INSERT INTO package_job_migration_backups VALUES(?,?,?,?)",
                params![VERSION, operation_id, original, updated_at],
            )
            .map_err(|_| StoreError::Storage)?;
        let updated = serde_json::to_string(&record).map_err(|_| StoreError::Storage)?;
        transaction
            .execute(
                "UPDATE package_jobs SET record_json=? WHERE operation_id=?",
                params![updated, operation_id],
            )
            .map_err(|_| StoreError::Storage)?;
        migrated += 1;
    }
    transaction
        .pragma_update(None, "user_version", VERSION)
        .map_err(|_| StoreError::Storage)?;
    transaction.commit().map_err(|_| StoreError::Storage)?;
    if migrated > 0 {
        tracing::info!(count = migrated, "migrated local package journal records");
    }
    Ok(())
}

fn migrate_job(record: &mut Value) -> Result<bool, StoreError> {
    let plan = record
        .pointer_mut("/job/plan")
        .and_then(Value::as_object_mut)
        .ok_or(StoreError::Storage)?;
    if plan.contains_key("installation") {
        return Ok(false);
    }
    let bundle = plan
        .remove("bundleId")
        .filter(Value::is_string)
        .ok_or(StoreError::Storage)?;
    let previous = plan.remove("previous").ok_or(StoreError::Storage)?;
    let appsync = plan.remove("appsync").ok_or(StoreError::Storage)?;
    let jailbreak = plan.remove("jailbreak").ok_or(StoreError::Storage)?;
    plan.insert("installation".into(), json!({"platform":"ios","bundleId":bundle,"previous":previous,"appsync":appsync,"jailbreak":jailbreak}));
    // The old iOS uninstall contract always included data removal consent.
    let deletes_data = plan.get("action").and_then(Value::as_str) == Some("uninstall");
    plan.insert("deleteData".into(), json!(deletes_data));
    if let Some(artifact) = plan.get_mut("artifact").filter(|value| !value.is_null()) {
        let artifact = artifact.as_object_mut().ok_or(StoreError::Storage)?;
        if artifact.get("format").and_then(Value::as_str) != Some("ipa") {
            return Err(StoreError::Storage);
        }
        let native = artifact
            .get_mut("native_identity")
            .and_then(Value::as_object_mut)
            .ok_or(StoreError::Storage)?;
        native.entry("kind").or_insert(json!("ios_bundle"));
        for target in artifact
            .get_mut("targets")
            .and_then(Value::as_array_mut)
            .ok_or(StoreError::Storage)?
        {
            migrate_target(target)?;
        }
    }
    if let Some(target) = plan.get_mut("target").filter(|value| !value.is_null()) {
        migrate_target(target)?;
    }
    Ok(true)
}
fn migrate_target(target: &mut Value) -> Result<(), StoreError> {
    let target: &mut Map<String, Value> = target.as_object_mut().ok_or(StoreError::Storage)?;
    if target.get("platform").and_then(Value::as_str) != Some("ios") {
        return Err(StoreError::Storage);
    }
    if let Some(delivery) = target.remove("runtime_delivery") {
        if target.contains_key("runtime_deliveries") {
            return Err(StoreError::Storage);
        }
        target.insert("runtime_deliveries".into(), json!([delivery]));
    }
    target
        .get_mut("requires")
        .and_then(Value::as_object_mut)
        .ok_or(StoreError::Storage)?
        .entry("kind")
        .or_insert(json!("ios"));
    target.entry("runtime_requirement").or_insert(Value::Null);
    target.entry("runtime_provides").or_insert(Value::Null);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const LEGACY: &str = include_str!("fixtures/legacy-package-job.json");
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE package_jobs(operation_id TEXT PRIMARY KEY,record_json TEXT NOT NULL,updated_at INTEGER NOT NULL);
            CREATE TABLE catalog_cache(catalog_bytes BLOB,pointer_json TEXT);
            INSERT INTO catalog_cache VALUES(x'00ff01','signed pointer stays unchanged');").unwrap();
        db.execute(
            "INSERT INTO package_jobs VALUES('old-operation',?,123)",
            [LEGACY],
        )
        .unwrap();
        db
    }
    #[test]
    fn legacy_journal_is_migrated_once_without_changing_authorization_or_signed_bytes() {
        let mut db = database();
        assert!(serde_json::from_str::<StoredPackageJob>(LEGACY).is_err());
        migrate(&mut db).unwrap();
        let updated: String = db
            .query_row("SELECT record_json FROM package_jobs", [], |r| r.get(0))
            .unwrap();
        let parsed: StoredPackageJob = serde_json::from_str(&updated).unwrap();
        assert_eq!(parsed.job.plan.bundle_id(), Some("dev.example.notes.ios"));
        assert!(parsed.job.submitted);
        assert_eq!(
            parsed.job.phase,
            crate::domain::packages::PackagePhase::Running
        );
        assert_eq!(parsed.job.plan.id, "old-plan");
        assert_eq!(parsed.binding, "private-test-binding");
        assert_eq!(parsed.job.updated_at, 123);
        assert!(!parsed.job.plan.delete_data);
        let artifact = parsed.job.plan.artifact.unwrap();
        assert_eq!(artifact.ios_identity().unwrap().build_number, "42");
        assert_eq!(artifact.targets[0], parsed.job.plan.target.unwrap());
        assert_eq!(artifact.blob.sha256, "1".repeat(64));
        let original: String = db
            .query_row(
                "SELECT record_json FROM package_job_migration_backups",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(original, LEGACY);
        migrate(&mut db).unwrap();
        assert_eq!(
            db.query_row::<String, _, _>("SELECT record_json FROM package_jobs", [], |r| r.get(0))
                .unwrap(),
            updated
        );
        assert_eq!(
            db.query_row::<i64, _, _>(
                "SELECT count(*) FROM package_job_migration_backups",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            db.query_row::<Vec<u8>, _, _>("SELECT catalog_bytes FROM catalog_cache", [], |r| r
                .get(0))
                .unwrap(),
            vec![0, 255, 1]
        );
        assert_eq!(
            db.query_row::<String, _, _>("SELECT pointer_json FROM catalog_cache", [], |r| r
                .get(0))
                .unwrap(),
            "signed pointer stays unchanged"
        );
    }
    #[test]
    fn an_invalid_later_record_rolls_back_the_whole_migration() {
        let mut db = database();
        db.execute(
            "INSERT INTO package_jobs VALUES('broken','{broken',124)",
            [],
        )
        .unwrap();
        assert!(migrate(&mut db).is_err());
        assert_eq!(
            db.query_row::<String, _, _>(
                "SELECT record_json FROM package_jobs WHERE operation_id='old-operation'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            LEGACY
        );
        assert_eq!(
            db.pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row::<i64, _, _>("SELECT count(*) FROM package_jobs", [], |r| r.get(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn uninstall_keeps_its_data_consent_and_current_platform_records_are_not_rewritten() {
        let mut value: Value = serde_json::from_str(LEGACY).unwrap();
        value["job"]["plan"]["action"] = json!("uninstall");
        value["job"]["plan"]["artifact"] = Value::Null;
        value["job"]["plan"]["target"] = Value::Null;
        assert!(migrate_job(&mut value).unwrap());
        let job: StoredPackageJob = serde_json::from_value(value.clone()).unwrap();
        assert!(job.job.plan.delete_data && job.job.submitted);
        let saved = value.clone();
        assert!(!migrate_job(&mut value).unwrap());
        assert_eq!(value, saved);
    }
}
