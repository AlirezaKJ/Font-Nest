use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

#[derive(Debug, Clone)]
pub struct ManagedInstallationRecord {
    pub id: String,
    pub provider: String,
    pub family_id: String,
    pub artifact_id: String,
    pub family_name: String,
    pub display_name: String,
    pub source_commit: String,
    pub source_hash: String,
    pub installed_path: String,
    pub registry_value_name: String,
    pub license: String,
    pub license_path: String,
}

#[derive(Debug, Clone)]
pub struct ManagedInstallationRepository {
    path: PathBuf,
}

impl ManagedInstallationRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn initialize(&self) -> Result<(), rusqlite::Error> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(&self.path)?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS managed_installations (
                id TEXT PRIMARY KEY NOT NULL,
                provider TEXT NOT NULL,
                family_id TEXT NOT NULL,
                artifact_id TEXT NOT NULL,
                family_name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                source_commit TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                installed_path TEXT NOT NULL,
                registry_value_name TEXT NOT NULL,
                license TEXT NOT NULL,
                license_path TEXT NOT NULL,
                installed_at INTEGER NOT NULL,
                UNIQUE(provider, artifact_id)
            );

            CREATE INDEX IF NOT EXISTS idx_managed_installations_provider_family
                ON managed_installations(provider, family_id);
            ",
        )
    }

    pub fn installed_artifact_ids(
        &self,
        provider: &str,
        family_id: &str,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let connection = Connection::open(&self.path)?;
        let mut statement = connection.prepare(
            "SELECT artifact_id
             FROM managed_installations
             WHERE provider = ?1 AND family_id = ?2
             ORDER BY artifact_id",
        )?;
        let rows = statement.query_map(params![provider, family_id], |row| row.get(0))?;
        rows.collect()
    }

    pub fn installed_family_ids(&self, provider: &str) -> Result<HashSet<String>, rusqlite::Error> {
        let connection = Connection::open(&self.path)?;
        let mut statement = connection.prepare(
            "SELECT DISTINCT family_id
             FROM managed_installations
             WHERE provider = ?1",
        )?;
        let rows = statement.query_map(params![provider], |row| row.get(0))?;
        rows.collect()
    }

    pub fn record_batch(
        &self,
        records: &[ManagedInstallationRecord],
    ) -> Result<(), rusqlite::Error> {
        if records.is_empty() {
            return Ok(());
        }

        let mut connection = Connection::open(&self.path)?;
        let transaction = connection.transaction()?;
        let installed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .try_into()
            .unwrap_or(i64::MAX);

        {
            let mut statement = transaction.prepare(
                "INSERT INTO managed_installations (
                    id, provider, family_id, artifact_id, family_name, display_name,
                    source_commit, source_hash, installed_path, registry_value_name,
                    license, license_path, installed_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
                 )",
            )?;
            for record in records {
                statement.execute(params![
                    record.id,
                    record.provider,
                    record.family_id,
                    record.artifact_id,
                    record.family_name,
                    record.display_name,
                    record.source_commit,
                    record.source_hash,
                    record.installed_path,
                    record.registry_value_name,
                    record.license,
                    record.license_path,
                    installed_at,
                ])?;
            }
        }

        transaction.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::{ManagedInstallationRecord, ManagedInstallationRepository};

    #[test]
    fn managed_installations_are_recorded_transactionally() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::new(temp.path().join("fontnest.sqlite3"));
        repository.initialize().expect("the first migration");
        let record = ManagedInstallationRecord {
            id: "google-fonts:gf:inter:regular".to_owned(),
            provider: "google-fonts".to_owned(),
            family_id: "gf:inter".to_owned(),
            artifact_id: "gf:inter:regular".to_owned(),
            family_name: "Inter".to_owned(),
            display_name: "Inter Regular".to_owned(),
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            source_hash: "30d74d258442c7c65512eafab474568dd706c430".to_owned(),
            installed_path:
                "C:\\Users\\Akari\\AppData\\Local\\Microsoft\\Windows\\Fonts\\FontNest-Inter.ttf"
                    .to_owned(),
            registry_value_name: "Inter Regular (TrueType)".to_owned(),
            license: "OFL-1.1".to_owned(),
            license_path: "C:\\FontNest\\licenses\\inter-OFL.txt".to_owned(),
        };

        repository
            .record_batch(&[record])
            .expect("the ledger write");

        assert_eq!(
            repository
                .installed_artifact_ids("google-fonts", "gf:inter")
                .expect("installed IDs"),
            vec!["gf:inter:regular"]
        );
    }
}
