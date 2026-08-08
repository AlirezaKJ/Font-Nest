//! Guards every mutation of managed font state behind two conditions: this process holds the
//! interprocess writer lock, and the managed-installation ledger is at a schema this build
//! understands. When either fails `FontNest` keeps running as a read-only catalogue and inspector,
//! and the commands that install, update, uninstall, repair, or restore fonts refuse to run
//! rather than working from a ledger they cannot trust.

use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::dto::{ManagedStorageRecovery, ManagedStorageStatus};
use crate::managed_installations::{LedgerError, ManagedInstallationRepository};

/// Lock file next to the ledger. Its contents are never read; holding the handle open is the
/// whole signal.
const WRITER_LOCK_FILE_NAME: &str = "fontnest.lock";

/// Windows reports a second exclusive open of the same file with one of these two codes.
#[cfg(windows)]
const SHARING_VIOLATION: i32 = 32;
#[cfg(windows)]
const LOCK_VIOLATION: i32 = 33;

/// Whether this process may mutate managed font state, and why not when it may not.
pub struct ManagedStorage {
    recovery: Option<ManagedStorageRecovery>,
    /// Held open for the lifetime of the process. Dropping it releases the writer lock, so it
    /// lives here rather than in the function that acquired it.
    _writer_lock: Option<File>,
}

impl ManagedStorage {
    /// Acquires the writer lock and migrates the ledger, falling back to read-only recovery mode
    /// when either step fails.
    pub fn initialize(app_data_dir: &Path) -> Self {
        if let Err(error) = std::fs::create_dir_all(app_data_dir) {
            log::error!("The managed font directory could not be created: {error}");
            return Self::recovery(ManagedStorageRecovery::LedgerUnavailable);
        }

        let writer_lock = match acquire_writer_lock(&app_data_dir.join(WRITER_LOCK_FILE_NAME)) {
            Ok(lock) => lock,
            Err(reason) => return Self::recovery(reason),
        };

        if let Err(error) =
            ManagedInstallationRepository::in_app_data_dir(app_data_dir).initialize()
        {
            log::error!("The managed-installation ledger could not be prepared: {error}");
            return Self::recovery(match error {
                LedgerError::SchemaTooNew { .. } => ManagedStorageRecovery::SchemaTooNew,
                LedgerError::Storage(_) | LedgerError::Unavailable(_) => {
                    ManagedStorageRecovery::LedgerUnavailable
                }
            });
        }

        Self {
            recovery: None,
            _writer_lock: Some(writer_lock),
        }
    }

    fn recovery(reason: ManagedStorageRecovery) -> Self {
        Self {
            recovery: Some(reason),
            _writer_lock: None,
        }
    }

    pub const fn status(&self) -> ManagedStorageStatus {
        ManagedStorageStatus {
            writable: self.recovery.is_none(),
            reason: self.recovery,
        }
    }

    pub const fn recovery_reason(&self) -> Option<ManagedStorageRecovery> {
        self.recovery
    }

    /// # Errors
    ///
    /// Returns the recovery reason when managed font state must not be mutated.
    pub const fn ensure_writable(&self) -> Result<(), ManagedStorageRecovery> {
        match self.recovery {
            Some(reason) => Err(reason),
            None => Ok(()),
        }
    }
}

/// Takes the interprocess writer lock by opening the lock file with no sharing allowed. Windows
/// releases the handle when the process ends, including after a crash, so a stale lock file never
/// keeps the next launch out.
#[cfg(windows)]
fn acquire_writer_lock(path: &Path) -> Result<File, ManagedStorageRecovery> {
    use std::os::windows::fs::OpenOptionsExt;

    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .share_mode(0)
        .open(path)
        .map_err(|error| {
            let taken = matches!(
                error.raw_os_error(),
                Some(SHARING_VIOLATION | LOCK_VIOLATION)
            );
            if taken {
                log::error!("Another FontNest process holds the managed font writer lock.");
                ManagedStorageRecovery::Locked
            } else {
                log::error!("The managed font writer lock could not be taken: {error}");
                ManagedStorageRecovery::LedgerUnavailable
            }
        })
}

/// Placeholder for the platforms that cannot manage fonts yet. Creating the file proves the
/// directory is writable but claims nothing about other processes; a real advisory lock lands with
/// the macOS and Linux platform adapters, which are the first builds able to mutate font state
/// there.
#[cfg(not(windows))]
fn acquire_writer_lock(path: &Path) -> Result<File, ManagedStorageRecovery> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|error| {
            log::error!("The managed font writer lock could not be taken: {error}");
            ManagedStorageRecovery::LedgerUnavailable
        })
}

#[cfg(test)]
mod tests {
    use super::ManagedStorage;
    use crate::dto::ManagedStorageRecovery;
    use crate::managed_installations::{ManagedInstallationRepository, SCHEMA_VERSION};

    #[test]
    fn a_fresh_application_directory_is_writable() {
        let temp = tempfile::tempdir().expect("a temporary directory");

        let storage = ManagedStorage::initialize(temp.path());

        assert_eq!(storage.recovery_reason(), None);
        assert!(storage.status().writable);
        assert!(storage.ensure_writable().is_ok());
    }

    #[test]
    fn a_forward_schema_ledger_falls_back_to_read_only_recovery() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the first migration");
        {
            let connection = rusqlite::Connection::open(temp.path().join("fontnest.sqlite3"))
                .expect("the ledger opens");
            connection
                .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
                .expect("a forward schema stamp");
        }

        let storage = ManagedStorage::initialize(temp.path());

        assert_eq!(
            storage.ensure_writable(),
            Err(ManagedStorageRecovery::SchemaTooNew)
        );
        assert!(!storage.status().writable);
    }

    #[cfg(windows)]
    #[test]
    fn a_second_holder_of_the_writer_lock_falls_back_to_read_only_recovery() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let first = ManagedStorage::initialize(temp.path());
        assert!(first.status().writable);

        let second = ManagedStorage::initialize(temp.path());

        assert_eq!(
            second.ensure_writable(),
            Err(ManagedStorageRecovery::Locked)
        );

        // Releasing the first holder lets the next process back in.
        drop(first);
        assert!(ManagedStorage::initialize(temp.path()).status().writable);
    }
}
