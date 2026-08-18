//! Undoes managed font operations that a previous run started and never finished.
//!
//! Installing a font changes the computer before `FontNest` can prove it owns what it changed: the
//! file lands in the user font directory and the registry points at it, and only then can the
//! ledger record the installation. A crash, a power cut, or a killed process in between would
//! leave fonts nothing accounts for, which is exactly the state that makes a later uninstall
//! unsafe.
//!
//! The operation journal closes that window. Every install writes down where each file will go and
//! what its registry value will be called before it touches either, and the ledger rows and the
//! removal of that intent record commit together. So at startup the rule is simple: an intent
//! record that outlived its process describes work `FontNest` cannot prove it finished, and the
//! only safe direction is backwards. Recovery undoes it and leaves the computer as it was.

use std::path::PathBuf;

use crate::font_platform::{FontPlatformError, PlatformInstallation, rollback_user_font};
use crate::managed_installations::{
    InterruptedOperation, ManagedInstallationRepository, PlannedInstallStep,
};

/// How many launches may try to undo the same operation before `FontNest` stops retrying it. An
/// operation that fails this often is not going to succeed on the next launch either, and retrying
/// it forever would make every start slower and every log noisier.
const MAX_RECOVERY_ATTEMPTS: i64 = 3;

/// What startup recovery did, so the interface can say it plainly instead of the user finding
/// fonts they did not install.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Interrupted operations undone during this launch.
    pub recovered: u32,
    /// Operations recovery has given up on, including ones from earlier launches. They are still
    /// recorded, and their files are still on the computer.
    pub quarantined: u32,
}

/// Undoes every operation the journal still holds open.
///
/// Never fails: a ledger that cannot be read leaves the computer exactly as it was, which is the
/// same outcome as having nothing to recover. The caller keeps running either way, because the
/// commands that would mutate managed fonts already refuse when the ledger is untrustworthy.
pub fn recover_interrupted_operations(
    repository: &ManagedInstallationRepository,
) -> RecoveryReport {
    recover_with(repository, |step| {
        rollback_user_font(&PlatformInstallation {
            installed_path: PathBuf::from(&step.installed_path),
            registry_value_name: step.registry_value_name.clone(),
            display_name: step.display_name.clone(),
        })
    })
}

fn recover_with<U>(repository: &ManagedInstallationRepository, undo: U) -> RecoveryReport
where
    U: Fn(&PlannedInstallStep) -> Result<(), FontPlatformError>,
{
    let operations = match repository.interrupted_operations() {
        Ok(operations) => operations,
        Err(error) => {
            log::error!("The font operation journal could not be read: {error}");
            return RecoveryReport::default();
        }
    };

    let mut recovered = 0_u32;
    for operation in &operations {
        log::warn!(
            "Undoing an interrupted {kind:?} of {count} font(s) from {provider}, attempt {attempt} of {MAX_RECOVERY_ATTEMPTS}: {id}",
            kind = operation.kind,
            count = operation.steps.len(),
            provider = operation.provider,
            attempt = operation.attempts + 1,
            id = operation.id,
        );
        if undo_operation(repository, operation, &undo) {
            recovered = recovered.saturating_add(1);
        } else {
            note_failure(repository, operation);
        }
    }

    RecoveryReport {
        recovered,
        quarantined: repository
            .quarantined_operation_count()
            .map_or(0, |count| u32::try_from(count).unwrap_or(u32::MAX)),
    }
}

/// Returns whether the whole operation was undone. A partly undone operation stays in the journal:
/// the next launch retries the steps that are left rather than forgetting them.
fn undo_operation<U>(
    repository: &ManagedInstallationRepository,
    operation: &InterruptedOperation,
    undo: &U,
) -> bool
where
    U: Fn(&PlannedInstallStep) -> Result<(), FontPlatformError>,
{
    for step in &operation.steps {
        match repository.is_recorded_installation(&step.installed_path) {
            // The ledger already proves this font was installed and accounted for, so the intent
            // record is stale rather than describing something to take away. Leaving the file is
            // the whole point of asking: a stale entry must never delete an owned font.
            Ok(true) => {
                log::warn!(
                    "An interrupted operation named an installed font, so it was left in place: {}",
                    step.artifact_id
                );
            }
            Ok(false) => {
                if let Err(error) = undo(step) {
                    log::error!(
                        "An interrupted installation of {artifact} could not be undone: {error}",
                        artifact = step.artifact_id,
                    );
                    return false;
                }
            }
            Err(error) => {
                log::error!("The managed-installation ledger could not be read: {error}");
                return false;
            }
        }
    }

    if let Err(error) = repository.discard_operation(&operation.id) {
        log::error!("A recovered operation could not be closed: {error}");
        return false;
    }
    true
}

fn note_failure(repository: &ManagedInstallationRepository, operation: &InterruptedOperation) {
    let attempts = match repository.record_failed_recovery(&operation.id) {
        Ok(attempts) => attempts,
        Err(error) => {
            log::error!("A failed recovery attempt could not be recorded: {error}");
            return;
        }
    };
    if attempts < MAX_RECOVERY_ATTEMPTS {
        return;
    }
    if let Err(error) = repository.quarantine_operation(&operation.id) {
        log::error!("An unrecoverable operation could not be quarantined: {error}");
        return;
    }
    log::error!(
        "FontNest stopped trying to undo {id} after {attempts} attempts. Its files are still on this computer.",
        id = operation.id,
    );
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{MAX_RECOVERY_ATTEMPTS, RecoveryReport, recover_with};
    use crate::font_platform::FontPlatformError;
    use crate::managed_installations::{
        ManagedInstallationRecord, ManagedInstallationRepository, OperationKind, PlannedInstallStep,
    };

    const OPERATION: &str = "google-fonts:gf:inter:1723996800000000000:4242";
    const INSTALLED_PATH: &str = "C:\\Fonts\\FontNest-30d74d258442-Inter-Regular.ttf";

    fn step() -> PlannedInstallStep {
        PlannedInstallStep {
            artifact_id: "gf:inter:regular".to_owned(),
            display_name: "Inter Regular".to_owned(),
            installed_path: INSTALLED_PATH.to_owned(),
            registry_value_name: "Inter Regular (TrueType)".to_owned(),
        }
    }

    fn record() -> ManagedInstallationRecord {
        let step = step();
        ManagedInstallationRecord {
            id: "google-fonts:gf:inter:regular".to_owned(),
            provider: "google-fonts".to_owned(),
            family_id: "gf:inter".to_owned(),
            artifact_id: step.artifact_id.clone(),
            family_name: "Inter".to_owned(),
            display_name: step.display_name.clone(),
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            source_hash: "30d74d258442c7c65512eafab474568dd706c430".to_owned(),
            installed_path: step.installed_path.clone(),
            registry_value_name: step.registry_value_name.clone(),
            license: "OFL-1.1".to_owned(),
            license_path: "C:\\FontNest\\licenses\\inter-OFL.txt".to_owned(),
            operation_id: OPERATION.to_owned(),
        }
    }

    fn interrupted(directory: &std::path::Path) -> ManagedInstallationRepository {
        let repository = ManagedInstallationRepository::in_app_data_dir(directory);
        repository.initialize().expect("the first migration");
        repository
            .begin_operation(OPERATION, OperationKind::Install, "google-fonts", &[step()])
            .expect("the journal write");
        repository
    }

    #[test]
    fn an_interrupted_installation_is_undone_and_closed() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = interrupted(temp.path());
        let undone = RefCell::new(Vec::new());

        let report = recover_with(&repository, |step| {
            undone.borrow_mut().push(step.installed_path.clone());
            Ok(())
        });

        assert_eq!(
            report,
            RecoveryReport {
                recovered: 1,
                quarantined: 0
            }
        );
        assert_eq!(undone.into_inner(), vec![INSTALLED_PATH.to_owned()]);
        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty(),
            "a recovered operation is not retried at the next launch"
        );
    }

    #[test]
    fn a_committed_installation_is_never_deleted_by_a_stale_journal_entry() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = interrupted(temp.path());
        // The same path is already recorded as owned, which is the one case where an open
        // operation must not take a file away.
        repository
            .commit_operation("google-fonts:earlier", &[record()])
            .expect("the ledger write");

        let report = recover_with(&repository, |_| {
            panic!("an installed font must not be rolled back")
        });

        assert_eq!(report.recovered, 1);
        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty()
        );
    }

    #[test]
    fn an_operation_that_cannot_be_undone_is_retried_then_quarantined() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = interrupted(temp.path());
        let refuse = |_: &PlannedInstallStep| Err(FontPlatformError::TargetConflict);

        for attempt in 1..MAX_RECOVERY_ATTEMPTS {
            let report = recover_with(&repository, refuse);
            assert_eq!(
                report,
                RecoveryReport {
                    recovered: 0,
                    quarantined: 0
                },
                "attempt {attempt} keeps the operation open for the next launch"
            );
            assert_eq!(
                repository
                    .interrupted_operations()
                    .expect("the journal")
                    .len(),
                1
            );
        }

        let report = recover_with(&repository, refuse);

        assert_eq!(
            report,
            RecoveryReport {
                recovered: 0,
                quarantined: 1
            }
        );
        assert!(
            repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty(),
            "a quarantined operation is not retried"
        );
        // Later launches keep reporting it, because its files are still on the computer.
        assert_eq!(recover_with(&repository, refuse).quarantined, 1);
    }

    #[test]
    fn nothing_to_recover_reports_nothing() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the first migration");

        let report = recover_with(&repository, |_| panic!("there is nothing to undo"));

        assert_eq!(report, RecoveryReport::default());
    }
}
