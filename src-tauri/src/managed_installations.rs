use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::font_identity::IdentityKey;

/// File name of the managed-installation ledger inside the application data directory.
pub const LEDGER_FILE_NAME: &str = "fontnest.sqlite3";

/// Highest schema version this build understands. It must equal `MIGRATIONS.len()`.
pub const SCHEMA_VERSION: i64 = 3;

/// The journal state of an operation that may still be undone at the next launch.
const OPERATION_OPEN: &str = "open";

/// The journal state of an operation recovery gave up on. It is kept for diagnostics and is
/// never retried, so a path `FontNest` refuses to touch cannot make every launch slower.
const OPERATION_QUARANTINED: &str = "quarantined";

/// How many times a first-sighted identity is moved off a taken ID before the ledger gives up and
/// lets the session fall back to its derived ID. Reaching even the second attempt means two
/// 128-bit digests collided, so this is a backstop rather than a path anything takes.
const IDENTITY_COLLISION_ATTEMPTS: u32 = 4;

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
const MIGRATIONS: &[&str] = &[
    "
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
    "
    CREATE TABLE managed_operations (
        id TEXT PRIMARY KEY NOT NULL,
        kind TEXT NOT NULL,
        provider TEXT NOT NULL,
        state TEXT NOT NULL,
        attempts INTEGER NOT NULL DEFAULT 0,
        started_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE managed_operation_steps (
        operation_id TEXT NOT NULL REFERENCES managed_operations(id) ON DELETE CASCADE,
        ordinal INTEGER NOT NULL,
        artifact_id TEXT NOT NULL,
        display_name TEXT NOT NULL,
        installed_path TEXT NOT NULL,
        registry_value_name TEXT NOT NULL,
        PRIMARY KEY (operation_id, ordinal)
    );

    CREATE INDEX idx_managed_operations_state ON managed_operations(state);

    ALTER TABLE managed_installations ADD COLUMN operation_id TEXT NOT NULL DEFAULT '';
",
    "
    CREATE TABLE font_identities (
        id TEXT PRIMARY KEY NOT NULL,
        kind TEXT NOT NULL,
        identity_key TEXT NOT NULL,
        first_seen_at INTEGER NOT NULL,
        last_seen_at INTEGER NOT NULL,
        UNIQUE(kind, identity_key)
    );

    CREATE INDEX idx_font_identities_kind_seen ON font_identities(kind, last_seen_at);
",
];

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
    /// The journal operation that placed this font, so a row can always be traced back to the
    /// run that produced it.
    pub operation_id: String,
}

/// What an interrupted operation was in the middle of doing. Only installation exists today;
/// update, uninstall, and repair join it when those operations land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Install,
}

impl OperationKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "install" => Some(Self::Install),
            _ => None,
        }
    }
}

/// One font an operation intends to place on this computer. Every field is resolved before the
/// filesystem or the registry is touched and written to the journal first, so an interrupted run
/// can be undone from the record alone rather than from a guess about how far it got.
#[derive(Debug, Clone)]
pub struct PlannedInstallStep {
    pub artifact_id: String,
    pub display_name: String,
    pub installed_path: String,
    pub registry_value_name: String,
}

/// An operation the journal still holds open: whatever started it neither committed nor discarded
/// it, so the computer may be carrying files and registry values `FontNest` cannot prove it owns.
#[derive(Debug, Clone)]
pub struct InterruptedOperation {
    pub id: String,
    pub kind: OperationKind,
    pub provider: String,
    pub attempts: i64,
    pub steps: Vec<PlannedInstallStep>,
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

    /// Writes down everything an operation is about to do, before it does any of it.
    ///
    /// The rows this commits are the only reason an interrupted run can be cleaned up: the paths
    /// and registry values are resolved up front rather than discovered as the work proceeds, so
    /// recovery never has to guess where a half-finished install left its files.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the journal cannot be written. The caller must abandon the
    /// operation rather than proceed, because nothing would be able to undo it.
    pub fn begin_operation(
        &self,
        id: &str,
        kind: OperationKind,
        provider: &str,
        steps: &[PlannedInstallStep],
    ) -> Result<(), rusqlite::Error> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let now = unix_seconds();

        transaction.execute(
            "INSERT INTO managed_operations (
                id, kind, provider, state, attempts, started_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
            params![id, kind.as_str(), provider, OPERATION_OPEN, now],
        )?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO managed_operation_steps (
                    operation_id, ordinal, artifact_id, display_name,
                    installed_path, registry_value_name
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (ordinal, step) in (0_i64..).zip(steps) {
                statement.execute(params![
                    id,
                    ordinal,
                    step.artifact_id,
                    step.display_name,
                    step.installed_path,
                    step.registry_value_name,
                ])?;
            }
        }

        transaction.commit()
    }

    /// Records what an operation installed and closes its journal entry in the same transaction.
    ///
    /// The two must not be separable. `FontNest` may only claim ownership of a font it can prove
    /// it placed, so the ledger rows and the disappearance of the intent record either both
    /// survive a crash or neither does.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the ledger cannot be written. The journal entry stays open,
    /// so the next launch undoes the installation instead of leaving fonts nothing owns.
    pub fn commit_operation(
        &self,
        id: &str,
        records: &[ManagedInstallationRecord],
    ) -> Result<(), rusqlite::Error> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        insert_records(&transaction, records)?;
        transaction.execute("DELETE FROM managed_operations WHERE id = ?1", params![id])?;
        transaction.commit()
    }

    /// Drops an operation that was undone while the process was still alive, so the next launch
    /// does not try to undo it again.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the journal entry cannot be removed.
    pub fn discard_operation(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.open()?
            .execute("DELETE FROM managed_operations WHERE id = ?1", params![id])
            .map(|_| ())
    }

    /// Every operation still held open, oldest first, with the steps needed to undo it.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the journal cannot be read.
    pub fn interrupted_operations(&self) -> Result<Vec<InterruptedOperation>, rusqlite::Error> {
        let connection = self.open()?;
        let mut operations = Vec::new();
        {
            let mut statement = connection.prepare(
                "SELECT id, kind, provider, attempts
                 FROM managed_operations
                 WHERE state = ?1
                 ORDER BY started_at, id",
            )?;
            let rows = statement.query_map(params![OPERATION_OPEN], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            for row in rows {
                let (id, kind, provider, attempts) = row?;
                // A kind this build does not understand is left alone rather than undone by
                // guesswork; it is counted as unrecovered so it stays visible.
                let Some(kind) = OperationKind::parse(&kind) else {
                    log::error!("The font operation journal holds an unknown operation: {kind}");
                    continue;
                };
                operations.push(InterruptedOperation {
                    id,
                    kind,
                    provider,
                    attempts,
                    steps: Vec::new(),
                });
            }
        }

        let mut statement = connection.prepare(
            "SELECT artifact_id, display_name, installed_path, registry_value_name
             FROM managed_operation_steps
             WHERE operation_id = ?1
             ORDER BY ordinal",
        )?;
        for operation in &mut operations {
            let rows = statement.query_map(params![operation.id], |row| {
                Ok(PlannedInstallStep {
                    artifact_id: row.get(0)?,
                    display_name: row.get(1)?,
                    installed_path: row.get(2)?,
                    registry_value_name: row.get(3)?,
                })
            })?;
            operation.steps = rows.collect::<Result<Vec<_>, _>>()?;
        }

        Ok(operations)
    }

    /// Counts one failed recovery pass and reports how many have now failed.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the attempt cannot be recorded.
    pub fn record_failed_recovery(&self, id: &str) -> Result<i64, rusqlite::Error> {
        let connection = self.open()?;
        connection.execute(
            "UPDATE managed_operations
             SET attempts = attempts + 1, updated_at = ?2
             WHERE id = ?1",
            params![id, unix_seconds()],
        )?;
        connection.query_row(
            "SELECT attempts FROM managed_operations WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
    }

    /// Stops retrying an operation recovery cannot finish, keeping the record for diagnostics.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the operation cannot be marked.
    pub fn quarantine_operation(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.open()?
            .execute(
                "UPDATE managed_operations SET state = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, OPERATION_QUARANTINED, unix_seconds()],
            )
            .map(|_| ())
    }

    /// How many operations recovery has given up on.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the journal cannot be read.
    pub fn quarantined_operation_count(&self) -> Result<i64, rusqlite::Error> {
        self.open()?.query_row(
            "SELECT COUNT(*) FROM managed_operations WHERE state = ?1",
            params![OPERATION_QUARANTINED],
            |row| row.get(0),
        )
    }

    /// Whether the ledger already proves `FontNest` owns the font at this path.
    ///
    /// Recovery asks before it deletes anything: a committed installation is never collateral
    /// damage of an unrelated journal entry.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the ledger cannot be read. The caller must treat that as
    /// "possibly owned" and leave the file alone.
    pub fn is_recorded_installation(&self, installed_path: &str) -> Result<bool, rusqlite::Error> {
        self.open()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM managed_installations WHERE installed_path = ?1)",
            params![installed_path],
            |row| row.get(0),
        )
    }

    /// Gives every scanned family and face the ID the ledger already knows it by, and records the
    /// ones it is seeing for the first time.
    ///
    /// A first sighting is stored under the ID its identity key derives, so the ledger and a
    /// session that cannot reach the ledger agree. What the ledger adds is durability: once a row
    /// exists, that ID stays with the identity key even if the derivation later changes, which is
    /// what a rename or an explicit family merge will need.
    ///
    /// A derived ID that is already taken by a different key is a real digest collision. It is
    /// logged and the newcomer is moved onto a disambiguated ID rather than allowed to shadow the
    /// row that got there first.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the ledger cannot be read or written. The caller falls back
    /// to derived IDs for the session, which are the same IDs in every case except a rename the
    /// ledger had already recorded.
    pub fn resolve_font_identities(
        &self,
        keys: &[IdentityKey],
    ) -> Result<HashMap<String, String>, rusqlite::Error> {
        let mut resolved = HashMap::with_capacity(keys.len());
        if keys.is_empty() {
            return Ok(resolved);
        }

        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let now = unix_seconds();
        {
            let mut lookup = transaction
                .prepare("SELECT id FROM font_identities WHERE kind = ?1 AND identity_key = ?2")?;
            let mut touch = transaction
                .prepare("UPDATE font_identities SET last_seen_at = ?2 WHERE id = ?1")?;
            let mut claim = transaction.prepare(
                "INSERT INTO font_identities (id, kind, identity_key, first_seen_at, last_seen_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(id) DO NOTHING",
            )?;

            for key in keys {
                if resolved.contains_key(key.key()) {
                    continue;
                }
                let kind = key.kind().as_str();
                let existing: Option<String> = lookup
                    .query_row(params![kind, key.key()], |row| row.get(0))
                    .optional()?;
                if let Some(id) = existing {
                    touch.execute(params![id, now])?;
                    resolved.insert(key.key().to_owned(), id);
                    continue;
                }

                let mut candidate = key.clone();
                for attempt in 1..=IDENTITY_COLLISION_ATTEMPTS {
                    let inserted =
                        claim.execute(params![candidate.derived_id(), kind, key.key(), now])?;
                    if inserted == 1 {
                        resolved.insert(key.key().to_owned(), candidate.into_derived_id());
                        break;
                    }
                    log::error!(
                        "Two different {kind} identities derive the ID {}. The newer one is being moved off it.",
                        candidate.derived_id()
                    );
                    candidate = key.disambiguated(attempt);
                }
            }
        }
        transaction.commit()?;

        Ok(resolved)
    }

    /// Opens a connection with the pragmas every ledger connection needs. They are set outside a
    /// transaction on purpose: `SQLite` ignores a `foreign_keys` change inside one, and the
    /// journal mode is a property of the file rather than of a statement. `synchronous = FULL`
    /// costs an fsync per commit and buys the thing the journal exists for: an intent record that
    /// is really on the disk before the filesystem and the registry are touched.
    fn open(&self) -> Result<Connection, rusqlite::Error> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            ",
        )?;
        Ok(connection)
    }
}

fn insert_records(
    transaction: &rusqlite::Transaction<'_>,
    records: &[ManagedInstallationRecord],
) -> Result<(), rusqlite::Error> {
    if records.is_empty() {
        return Ok(());
    }

    let installed_at = unix_seconds();
    let mut statement = transaction.prepare(
        "INSERT INTO managed_installations (
            id, provider, family_id, artifact_id, family_name, display_name,
            source_commit, source_hash, installed_path, registry_value_name,
            license, license_path, installed_at, operation_id
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
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
            record.operation_id,
        ])?;
    }

    Ok(())
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn schema_version(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.pragma_query_value(None, "user_version", |row| row.get(0))
}

#[cfg(test)]
mod tests {
    use super::{
        InterruptedOperation, LedgerError, MIGRATIONS, ManagedInstallationRecord,
        ManagedInstallationRepository, OperationKind, PlannedInstallStep, SCHEMA_VERSION,
        schema_version,
    };
    use crate::font_identity::{FileIdentity, face_identity_key, family_identity_key};
    use rusqlite::Connection;
    use std::path::Path;

    const OPERATION: &str = "google-fonts:gf:inter:1723996800000000000:4242";

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
            operation_id: OPERATION.to_owned(),
        }
    }

    fn sample_step() -> PlannedInstallStep {
        let record = sample_record();
        PlannedInstallStep {
            artifact_id: record.artifact_id,
            display_name: record.display_name,
            installed_path: record.installed_path,
            registry_value_name: record.registry_value_name,
        }
    }

    fn begin(repository: &ManagedInstallationRepository) {
        repository
            .begin_operation(
                OPERATION,
                OperationKind::Install,
                "google-fonts",
                &[sample_step()],
            )
            .expect("the journal write");
    }

    /// The whole install path in one call: journal the intent, then commit the ledger rows.
    fn install(repository: &ManagedInstallationRepository) {
        begin(repository);
        repository
            .commit_operation(OPERATION, &[sample_record()])
            .expect("the ledger write");
    }

    fn ready_repository(directory: &Path) -> ManagedInstallationRepository {
        let repository = ManagedInstallationRepository::in_app_data_dir(directory);
        repository.initialize().expect("the first migration");
        repository
    }

    fn recorded_version(path: &Path) -> i64 {
        let connection = Connection::open(path).expect("the ledger opens");
        schema_version(&connection).expect("a schema version")
    }

    fn only(operations: &[InterruptedOperation]) -> &InterruptedOperation {
        assert_eq!(operations.len(), 1, "exactly one open operation");
        &operations[0]
    }

    /// A font meets the ledger under the ID its own identity derives, so a session that cannot
    /// open the ledger names it exactly the same way.
    #[test]
    fn a_first_sighting_is_recorded_under_the_id_it_derives() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the migrations");
        let key = family_identity_key("Source Serif 4");

        let resolved = repository
            .resolve_font_identities(std::slice::from_ref(&key))
            .expect("the identities resolve");

        assert_eq!(
            resolved.get(key.key()).map(String::as_str),
            Some(key.derived_id())
        );
    }

    /// The point of writing identities down: the second scan gets the same answer as the first,
    /// even for a font whose file has moved since.
    #[test]
    fn a_second_scan_gets_the_same_ids_as_the_first() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the migrations");
        let keys = vec![
            family_identity_key("Inter"),
            face_identity_key(
                &FileIdentity::Platform {
                    volume: 3,
                    index: 91,
                },
                0,
                "Inter-Regular",
            ),
        ];

        let first = repository
            .resolve_font_identities(&keys)
            .expect("the first scan");
        let second = repository
            .resolve_font_identities(&keys)
            .expect("the second scan");

        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
    }

    /// An ID the ledger has already handed out stays with the identity it was given to. A newer
    /// identity that derives the same ID is moved off it instead of shadowing the first.
    #[test]
    fn a_taken_id_is_never_handed_to_a_second_identity() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the migrations");
        let first = family_identity_key("Inter");
        let colliding = family_identity_key("Geist");
        {
            // Stand in for a digest collision by claiming the second family's derived ID under an
            // unrelated key, which is the only way this can happen in practice.
            let connection =
                Connection::open(temp.path().join(super::LEDGER_FILE_NAME)).expect("the ledger");
            connection
                .execute(
                    "INSERT INTO font_identities (id, kind, identity_key, first_seen_at, last_seen_at)
                     VALUES (?1, 'family', 'a-key-from-an-older-build', 0, 0)",
                    [colliding.derived_id()],
                )
                .expect("the claim is written");
        }

        let resolved = repository
            .resolve_font_identities(&[first.clone(), colliding.clone()])
            .expect("the identities resolve");

        assert_eq!(
            resolved.get(first.key()).map(String::as_str),
            Some(first.derived_id())
        );
        let moved = resolved.get(colliding.key()).expect("the second family");
        assert_ne!(moved, colliding.derived_id());
        assert_eq!(moved, colliding.disambiguated(1).derived_id());
    }

    /// A family and a face are different kinds of thing, so one key never answers for the other.
    #[test]
    fn identities_of_different_kinds_are_stored_apart() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the migrations");
        let family = family_identity_key("Inter");
        let face = face_identity_key(&FileIdentity::Embedded, 0, "Inter-Regular");

        let resolved = repository
            .resolve_font_identities(&[family.clone(), face.clone()])
            .expect("the identities resolve");

        assert_ne!(resolved[family.key()], resolved[face.key()]);
    }

    #[test]
    fn schema_version_matches_the_migration_count() {
        assert_eq!(SCHEMA_VERSION, i64::try_from(MIGRATIONS.len()).unwrap());
    }

    #[test]
    fn managed_installations_are_recorded_transactionally() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ready_repository(temp.path());

        install(&repository);

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
        install(&repository);

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
        // stamped `user_version`, so the file still reports schema 0 and carries neither a
        // journal nor the column that names the operation a row came from.
        let temp = tempfile::tempdir().expect("a temporary directory");
        let path = temp.path().join("fontnest.sqlite3");
        {
            let connection = Connection::open(&path).expect("a legacy ledger");
            connection
                .execute_batch(MIGRATIONS[0])
                .expect("the legacy tables");
            connection
                .execute(
                    "INSERT INTO managed_installations (
                        id, provider, family_id, artifact_id, family_name, display_name,
                        source_commit, source_hash, installed_path, registry_value_name,
                        license, license_path, installed_at
                     ) VALUES (
                        'google-fonts:gf:inter:regular', 'google-fonts', 'gf:inter',
                        'gf:inter:regular', 'Inter', 'Inter Regular', '0123', 'abcd',
                        'C:\\Fonts\\FontNest-Inter.ttf', 'Inter Regular (TrueType)',
                        'OFL-1.1', 'C:\\licenses\\inter.txt', 0
                     )",
                    [],
                )
                .expect("a legacy row");
        }
        let repository = ManagedInstallationRepository::new(path.clone());
        assert_eq!(recorded_version(&path), 0);

        repository.initialize().expect("the adoption run");

        assert_eq!(recorded_version(&path), SCHEMA_VERSION);
        assert_eq!(
            repository
                .installed_artifact_ids("google-fonts", "gf:inter")
                .expect("installed IDs"),
            vec!["gf:inter:regular"]
        );
        // Adopting a ledger must not invent work for recovery to undo.
        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty()
        );
    }

    #[test]
    fn a_forward_schema_is_refused_and_left_untouched() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let path = temp.path().join("fontnest.sqlite3");
        let repository = ManagedInstallationRepository::new(path.clone());
        repository.initialize().expect("the first migration");
        install(&repository);
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

    #[test]
    fn an_open_operation_carries_everything_needed_to_undo_it() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ready_repository(temp.path());

        begin(&repository);

        // This is what the next launch sees after a crash part way through an install.
        let open = repository.interrupted_operations().expect("the journal");
        let operation = only(&open);
        assert_eq!(operation.id, OPERATION);
        assert_eq!(operation.kind, OperationKind::Install);
        assert_eq!(operation.provider, "google-fonts");
        assert_eq!(operation.attempts, 0);
        assert_eq!(operation.steps.len(), 1);
        assert_eq!(
            operation.steps[0].installed_path,
            sample_step().installed_path
        );
        assert_eq!(
            operation.steps[0].registry_value_name,
            sample_step().registry_value_name
        );
    }

    #[test]
    fn committing_records_the_installation_and_closes_the_journal_together() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ready_repository(temp.path());

        install(&repository);

        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty()
        );
        assert!(
            repository
                .is_recorded_installation(&sample_record().installed_path)
                .expect("the ledger lookup")
        );
    }

    #[test]
    fn a_failed_ledger_write_leaves_the_operation_open_for_recovery() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ready_repository(temp.path());
        begin(&repository);
        // Two rows with the same primary key, so the insert fails part way through the batch.
        let records = vec![sample_record(), sample_record()];

        repository
            .commit_operation(OPERATION, &records)
            .expect_err("a duplicate installation is refused");

        // Neither half landed, so the next launch still knows to undo the fonts on disk.
        assert!(
            !repository
                .is_recorded_installation(&sample_record().installed_path)
                .expect("the ledger lookup")
        );
        assert_eq!(
            only(&repository.interrupted_operations().expect("the journal")).id,
            OPERATION
        );
    }

    #[test]
    fn discarding_an_operation_removes_it_and_its_steps() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ready_repository(temp.path());
        begin(&repository);

        repository
            .discard_operation(OPERATION)
            .expect("the journal delete");

        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty()
        );
        let connection =
            Connection::open(temp.path().join("fontnest.sqlite3")).expect("the ledger opens");
        let steps: i64 = connection
            .query_row("SELECT COUNT(*) FROM managed_operation_steps", [], |row| {
                row.get(0)
            })
            .expect("a step count");
        assert_eq!(steps, 0, "the cascade removed the steps");
    }

    #[test]
    fn a_quarantined_operation_is_counted_but_never_retried() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ready_repository(temp.path());
        begin(&repository);

        assert_eq!(
            repository
                .record_failed_recovery(OPERATION)
                .expect("an attempt count"),
            1
        );
        assert_eq!(
            repository
                .record_failed_recovery(OPERATION)
                .expect("an attempt count"),
            2
        );
        repository
            .quarantine_operation(OPERATION)
            .expect("the quarantine");

        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty()
        );
        assert_eq!(
            repository
                .quarantined_operation_count()
                .expect("the quarantine count"),
            1
        );
    }
}
