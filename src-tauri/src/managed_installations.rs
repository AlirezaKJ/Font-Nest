use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

/// File name of the managed-installation ledger inside the application data directory.
pub const LEDGER_FILE_NAME: &str = "fontnest.sqlite3";

/// Highest schema version this build understands. It must equal `MIGRATIONS.len()`.
pub const SCHEMA_VERSION: i64 = 1;

/// How long a statement waits for another connection to release the write lock before it
/// reports a busy database.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Numbered, forward-only migrations. Entry `n` upgrades a ledger from version `n - 1` to `n`,
/// runs exactly once, and is committed together with the new `user_version`, so an interrupted
/// upgrade leaves the ledger on its previous version instead of half migrated.
///
/// Migration 1 uses `IF NOT EXISTS` because ledgers written before migrations existed already
/// carry these objects at `user_version = 0`; it adopts them without touching their rows. Every
/// later migration must be written strictly, because it can only ever meet a ledger this build
/// created.
const MIGRATIONS: &[&str] = &["
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
"];

/// Why the ledger could not be brought up to the schema this build expects.
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("the managed-installation ledger directory could not be created")]
    Storage(#[from] std::io::Error),
    #[error("the managed-installation ledger could not be opened or migrated")]
    Unavailable(#[from] rusqlite::Error),
    /// A newer `FontNest` wrote this ledger. `FontNest` never downgrades a ledger and never
    /// guesses at a schema it does not know: the file is left exactly as it was found and every
    /// managed operation stays disabled until a build that understands the schema runs again.
    #[error("the ledger is at schema version {found}, but this build supports {supported}")]
    SchemaTooNew { found: i64, supported: i64 },
}

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

    /// The ledger that belongs to an application data directory.
    pub fn in_app_data_dir(app_data_dir: &Path) -> Self {
        Self::new(app_data_dir.join(LEDGER_FILE_NAME))
    }

    /// Brings the ledger up to [`SCHEMA_VERSION`], creating it when it does not exist yet.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::SchemaTooNew`] when the file was written by a newer `FontNest`, and
    /// the underlying I/O or `SQLite` error when the ledger cannot be opened or migrated. In
    /// every failure case the caller must treat managed font state as read-only.
    pub fn initialize(&self) -> Result<(), LedgerError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut connection = self.open()?;
        let version = schema_version(&connection)?;
        if version > SCHEMA_VERSION {
            return Err(LedgerError::SchemaTooNew {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }

        for (target, migration) in (1_i64..).zip(MIGRATIONS.iter()) {
            if target <= version {
                continue;
            }
            let transaction = connection.transaction()?;
            transaction.execute_batch(migration)?;
            transaction.pragma_update(None, "user_version", target)?;
            transaction.commit()?;
        }

        Ok(())
    }

    pub fn installed_artifact_ids(
        &self,
        provider: &str,
        family_id: &str,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let connection = self.open()?;
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
        let connection = self.open()?;
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

        let mut connection = self.open()?;
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

    /// Opens a connection with the pragmas every ledger connection needs. Both are set outside a
    /// transaction on purpose: `SQLite` ignores a `foreign_keys` change inside one, and the
    /// journal mode is a property of the file rather than of a statement.
    fn open(&self) -> Result<Connection, rusqlite::Error> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            ",
        )?;
        Ok(connection)
    }
}

fn schema_version(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.pragma_query_value(None, "user_version", |row| row.get(0))
}

#[cfg(test)]
mod tests {
    use super::{
        LedgerError, MIGRATIONS, ManagedInstallationRecord, ManagedInstallationRepository,
        SCHEMA_VERSION, schema_version,
    };
    use rusqlite::Connection;
    use std::path::Path;

    fn sample_record() -> ManagedInstallationRecord {
        ManagedInstallationRecord {
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
        }
    }

    fn recorded_version(path: &Path) -> i64 {
        let connection = Connection::open(path).expect("the ledger opens");
        schema_version(&connection).expect("a schema version")
    }

    #[test]
    fn schema_version_matches_the_migration_count() {
        assert_eq!(SCHEMA_VERSION, i64::try_from(MIGRATIONS.len()).unwrap());
    }

    #[test]
    fn managed_installations_are_recorded_transactionally() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the first migration");

        repository
            .record_batch(&[sample_record()])
            .expect("the ledger write");

        assert_eq!(
            repository
                .installed_artifact_ids("google-fonts", "gf:inter")
                .expect("installed IDs"),
            vec!["gf:inter:regular"]
        );
    }

    #[test]
    fn a_new_ledger_is_stamped_with_the_current_schema_version() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let path = temp.path().join("fontnest.sqlite3");
        let repository = ManagedInstallationRepository::new(path.clone());

        repository.initialize().expect("the first migration");

        assert_eq!(recorded_version(&path), SCHEMA_VERSION);
    }

    #[test]
    fn initializing_twice_keeps_the_version_and_the_rows() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let path = temp.path().join("fontnest.sqlite3");
        let repository = ManagedInstallationRepository::new(path.clone());
        repository.initialize().expect("the first migration");
        repository
            .record_batch(&[sample_record()])
            .expect("the ledger write");

        repository.initialize().expect("a repeated migration run");

        assert_eq!(recorded_version(&path), SCHEMA_VERSION);
        assert_eq!(
            repository
                .installed_family_ids("google-fonts")
                .expect("installed families")
                .len(),
            1
        );
    }

    #[test]
    fn a_pre_migration_ledger_is_adopted_without_losing_rows() {
        // Exactly what FontNest 0.1.4 and earlier left behind: the tables exist, but nothing ever
        // stamped `user_version`, so the file still reports schema 0.
        let temp = tempfile::tempdir().expect("a temporary directory");
        let path = temp.path().join("fontnest.sqlite3");
        {
            let connection = Connection::open(&path).expect("a legacy ledger");
            connection
                .execute_batch(MIGRATIONS[0])
                .expect("the legacy tables");
        }
        let repository = ManagedInstallationRepository::new(path.clone());
        repository
            .record_batch(&[sample_record()])
            .expect("a legacy row");
        assert_eq!(recorded_version(&path), 0);

        repository.initialize().expect("the adoption run");

        assert_eq!(recorded_version(&path), SCHEMA_VERSION);
        assert_eq!(
            repository
                .installed_artifact_ids("google-fonts", "gf:inter")
                .expect("installed IDs"),
            vec!["gf:inter:regular"]
        );
    }

    #[test]
    fn a_forward_schema_is_refused_and_left_untouched() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let path = temp.path().join("fontnest.sqlite3");
        let repository = ManagedInstallationRepository::new(path.clone());
        repository.initialize().expect("the first migration");
        repository
            .record_batch(&[sample_record()])
            .expect("the ledger write");
        let future = SCHEMA_VERSION + 1;
        {
            let connection = Connection::open(&path).expect("the ledger opens");
            connection
                .pragma_update(None, "user_version", future)
                .expect("a forward schema stamp");
        }

        let error = repository
            .initialize()
            .expect_err("a forward schema is refused");

        assert!(matches!(
            error,
            LedgerError::SchemaTooNew { found, supported }
                if found == future && supported == SCHEMA_VERSION
        ));
        // Refusing must not downgrade the file or drop what the newer build recorded.
        assert_eq!(recorded_version(&path), future);
        assert_eq!(
            repository
                .installed_artifact_ids("google-fonts", "gf:inter")
                .expect("installed IDs"),
            vec!["gf:inter:regular"]
        );
    }
}
