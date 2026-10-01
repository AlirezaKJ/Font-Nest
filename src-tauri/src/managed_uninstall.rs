//! Takes fonts back off this computer, one proof at a time.
//!
//! Uninstall is the operation with the most to lose. Installing badly leaves a font somewhere it
//! should not be; uninstalling badly takes away a file that was never `FontNest`'s, and there is
//! no undo for that from the user's side. So the order here is deliberate, and every step is
//! either reversible or has already been proven safe:
//!
//! 1. Every artifact the ledger claims is put to [`crate::managed_ownership`], which verifies it
//!    against the computer rather than against the ledger. Anything that cannot be proven is
//!    reported and left completely alone; it never reaches the rest of this module.
//! 2. What survives is written to the operation journal before anything changes, including where
//!    each file is going to be set aside, so a run that dies halfway can be put back from the
//!    record instead of from a guess.
//! 3. The font is taken out of service first: the registration goes, the file stays. A run that
//!    stops here has stopped a font being used without having taken anything away.
//! 4. The file is moved into quarantine rather than deleted. Permanent deletion is never the first
//!    action, so a removal that turns out to be wrong is a move away and a move back.
//! 5. Only then do the ledger rows and the journal entry go, together, in one transaction. Until
//!    that moment the ledger still claims the font, and the next launch restores it.

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::font_platform::{FontPlatformError, is_managed_file_name};
use crate::managed_installations::{
    ManagedInstallationRecord, ManagedInstallationRepository, OperationKind, OperationStep,
    QuarantinedFont,
};
use crate::managed_ownership::{
    ManagedEnvironment, OwnershipRefusal, ProvenOwnership, UninstallClaim, authorize_uninstall,
    content_hash, still_the_proven_file,
};

/// Where uninstalled fonts are kept, inside the application data directory.
///
/// They stay there. Reclaiming the space is a decision for the user to make once restoring is
/// something the interface offers, not something an uninstall quietly does on their behalf.
pub const QUARANTINE_DIRECTORY: &str = "quarantine";

/// A font this run took out of service and set aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedFont {
    pub artifact_id: String,
    pub display_name: String,
}

/// A font this run would not touch, and the check that stopped it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedFont {
    pub artifact_id: String,
    pub display_name: String,
    pub reason: OwnershipRefusal,
}

/// What a whole uninstall did. A run can succeed and still refuse fonts: the ones it could prove
/// are gone, and the ones it could not are reported untouched rather than silently skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UninstallOutcome {
    pub removed: Vec<RemovedFont>,
    pub refused: Vec<RefusedFont>,
}

#[derive(Debug, thiserror::Error)]
pub enum UninstallError {
    #[error("the managed-installation ledger is unavailable")]
    Ledger,
    #[error("the font could not be taken out of service")]
    Platform,
    #[error("the quarantine directory is unavailable")]
    Quarantine,
    #[error("the font changed while it was being removed")]
    Changed,
}

/// Takes back the installations a provider recorded for one family.
///
/// Passing no artifact IDs means the whole family. Anything named that the ledger does not have a
/// row for is ignored: this only ever acts on what was recorded, and then only on what can be
/// proven.
///
/// # Errors
///
/// Returns [`UninstallError::Ledger`] when the ledger cannot be read or written, and the platform
/// or quarantine error when a font that was proven could not be taken out of service or set aside.
/// In every failure case the journal entry stays open, so the next launch puts back whatever this
/// run had already taken.
pub fn uninstall_family(
    repository: &ManagedInstallationRepository,
    app_data_dir: &Path,
    provider: &str,
    family_id: &str,
    artifact_ids: &[String],
    environment: &impl ManagedEnvironment,
) -> Result<UninstallOutcome, UninstallError> {
    let recorded = repository
        .installations(provider, family_id)
        .map_err(|_| UninstallError::Ledger)?;
    let requested = recorded
        .into_iter()
        .filter(|record| artifact_ids.is_empty() || artifact_ids.contains(&record.artifact_id));

    let mut outcome = UninstallOutcome::default();
    let mut proven: Vec<(ManagedInstallationRecord, ProvenOwnership)> = Vec::new();
    for record in requested {
        let claim = UninstallClaim {
            provider: &record.provider,
            artifact_id: &record.artifact_id,
            installed_path: &record.installed_path,
            registry_value_name: &record.registry_value_name,
            source_hash: &record.source_hash,
        };
        match authorize_uninstall(&claim, environment) {
            Ok(proof) => proven.push((record, proof)),
            Err(reason) => {
                log::warn!(
                    "FontNest will not remove {artifact}: {reason}",
                    artifact = record.artifact_id,
                );
                outcome.refused.push(RefusedFont {
                    artifact_id: record.artifact_id,
                    display_name: record.display_name,
                    reason,
                });
            }
        }
    }

    if proven.is_empty() {
        return Ok(outcome);
    }

    let operation_id = new_operation_id(family_id);
    let quarantine = app_data_dir
        .join(QUARANTINE_DIRECTORY)
        .join(directory_name(&operation_id));
    std::fs::create_dir_all(&quarantine).map_err(|_| UninstallError::Quarantine)?;

    // Everything this run is about to do, written down before it does any of it.
    let steps = proven
        .iter()
        .map(|(record, proof)| OperationStep {
            artifact_id: record.artifact_id.clone(),
            display_name: proof.display_name.clone(),
            installed_path: proof.path.to_string_lossy().into_owned(),
            registry_value_name: proof.registry_value_name.clone(),
            quarantine_path: quarantine_path(&quarantine, proof)
                .to_string_lossy()
                .into_owned(),
        })
        .collect::<Vec<_>>();
    repository
        .begin_operation(&operation_id, OperationKind::Uninstall, provider, &steps)
        .map_err(|_| UninstallError::Ledger)?;

    let removed_at = seconds_since_epoch();
    let mut installation_ids = Vec::with_capacity(proven.len());
    let mut quarantined = Vec::with_capacity(proven.len());
    for ((record, proof), step) in proven.iter().zip(&steps) {
        if let Err(error) =
            take_out_of_service(environment, proof, Path::new(&step.quarantine_path))
        {
            log::error!(
                "FontNest stopped removing {artifact} and will put back what it had taken: {error}",
                artifact = record.artifact_id,
            );
            return Err(error);
        }
        installation_ids.push(record.id.clone());
        // Where the file went, and everything needed to put it back. A quarantined file nothing
        // remembers is one nobody can offer back, which would make setting it aside a kindness
        // only in principle.
        quarantined.push(QuarantinedFont {
            id: record.id.clone(),
            provider: record.provider.clone(),
            family_id: record.family_id.clone(),
            artifact_id: record.artifact_id.clone(),
            family_name: record.family_name.clone(),
            display_name: proof.display_name.clone(),
            source_hash: record.source_hash.clone(),
            installed_path: proof.path.to_string_lossy().into_owned(),
            registry_value_name: proof.registry_value_name.clone(),
            quarantine_path: step.quarantine_path.clone(),
            license: record.license.clone(),
            license_path: record.license_path.clone(),
            removed_at,
        });
        outcome.removed.push(RemovedFont {
            artifact_id: record.artifact_id.clone(),
            display_name: proof.display_name.clone(),
        });
    }

    // The ledger stops claiming these fonts, starts offering them back, and the journal entry
    // closes, all together. Until this commits, the fonts are still recorded as installed and the
    // next launch restores them.
    repository
        .commit_uninstall(&operation_id, &installation_ids, &quarantined)
        .map_err(|_| UninstallError::Ledger)?;

    Ok(outcome)
}

/// Why a font a removal set aside could not be put back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RestoreRefusal {
    #[error("the quarantined file is no longer there")]
    Missing,
    #[error("the quarantined file no longer holds the bytes FontNest set aside")]
    ContentMismatch,
    #[error("the font is installed again, so there is nothing to put back")]
    AlreadyInstalled,
    #[error("the font could not be put back where it was")]
    Platform,
    #[error("the managed installation ledger is unavailable")]
    Ledger,
}

/// Puts a font a removal set aside back into service.
///
/// A quarantined file lives in the application's own data directory, which is a place the person
/// can edit, so the bytes are checked before any of this: they have to be the bytes that were set
/// aside. Otherwise a restore would register whatever happens to be sitting at that path under a
/// name `FontNest` chose, which is exactly the thing the uninstall proof exists to prevent in the
/// other direction. Where the file goes is checked too, by the same containment rule a recovery
/// uses: a managed name, directly inside the per-user font directory.
///
/// # Errors
///
/// Returns the [`RestoreRefusal`] naming the check that stopped. A refusal leaves the quarantined
/// file exactly where it is.
pub fn restore_quarantined_font(
    repository: &ManagedInstallationRepository,
    font: &QuarantinedFont,
    environment: &impl ManagedEnvironment,
) -> Result<(), RestoreRefusal> {
    let quarantined = Path::new(&font.quarantine_path);
    let bytes = std::fs::read(quarantined).map_err(|_| RestoreRefusal::Missing)?;
    if !content_hash(&bytes).eq_ignore_ascii_case(&font.source_hash) {
        return Err(RestoreRefusal::ContentMismatch);
    }
    if Path::new(&font.installed_path).exists() {
        return Err(RestoreRefusal::AlreadyInstalled);
    }

    let installed_path =
        return_quarantined_file(environment, &font.installed_path, &font.quarantine_path)
            .map_err(|_| RestoreRefusal::Platform)?;
    if !installed_path.is_file() {
        return Err(RestoreRefusal::Missing);
    }
    environment
        .register_font(&font.registry_value_name, &installed_path)
        .map_err(|_| RestoreRefusal::Platform)?;

    repository
        .commit_restore(&ManagedInstallationRecord {
            id: font.id.clone(),
            provider: font.provider.clone(),
            family_id: font.family_id.clone(),
            artifact_id: font.artifact_id.clone(),
            family_name: font.family_name.clone(),
            display_name: font.display_name.clone(),
            // A restored font was not placed by a provider run, and saying otherwise would invent
            // provenance it does not have.
            source_commit: String::new(),
            source_hash: font.source_hash.clone(),
            installed_path: installed_path.to_string_lossy().into_owned(),
            registry_value_name: font.registry_value_name.clone(),
            license: font.license.clone(),
            license_path: font.license_path.clone(),
            operation_id: String::new(),
        })
        .map_err(|_| RestoreRefusal::Ledger)
}

/// When a removal happened, for an interface that lists what can still be put back.
fn seconds_since_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or_default()
}

/// Puts back a font an interrupted uninstall had already taken.
///
/// Both halves are written to be safe to repeat, because recovery cannot know how far the run that
/// died had got: the file only moves back when the quarantined copy is there and nothing is in its
/// place, and the registration is simply set again.
///
/// # Errors
///
/// Returns [`FontPlatformError::TargetConflict`] when the journal names a path that is not a
/// managed font file in the per-user font directory, and the platform error when the file or the
/// registration cannot be restored.
pub fn restore_uninstalled_font(
    environment: &impl ManagedEnvironment,
    step: &OperationStep,
) -> Result<(), FontPlatformError> {
    let installed_path =
        return_quarantined_file(environment, &step.installed_path, &step.quarantine_path)?;
    if !installed_path.is_file() {
        // Nothing to put back and nothing to register: the run never got as far as moving the
        // file, or something else has removed it since.
        return Ok(());
    }

    environment.register_font(&step.registry_value_name, &installed_path)
}

/// Moves a quarantined font back to where it was installed, and reports that path.
///
/// The journal is a record this process did not write, so the destination is checked before
/// anything moves: it has to be a managed file name directly inside the per-user font directory,
/// which is the same containment an uninstall proved before it took the file away. A file already
/// sitting there is left alone rather than overwritten, because the run may simply have died after
/// putting it back.
fn return_quarantined_file(
    environment: &impl ManagedEnvironment,
    installed_path: &str,
    quarantine_path: &str,
) -> Result<PathBuf, FontPlatformError> {
    let installed_path = PathBuf::from(installed_path);
    if !is_managed_destination(environment, &installed_path) {
        return Err(FontPlatformError::TargetConflict);
    }

    let quarantined = Path::new(quarantine_path);
    if !quarantine_path.is_empty() && quarantined.is_file() && !installed_path.exists() {
        std::fs::rename(quarantined, &installed_path).or_else(|_| {
            std::fs::copy(quarantined, &installed_path)
                .and_then(|_| std::fs::remove_file(quarantined))
        })?;
    }
    Ok(installed_path)
}

/// Whether a path is somewhere `FontNest` may put a font back: a managed name, directly inside the
/// per-user font directory, with the directory resolved rather than taken as spelled.
fn is_managed_destination(environment: &impl ManagedEnvironment, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !is_managed_file_name(name) {
        return false;
    }
    let (Some(directory), Some(parent)) = (environment.user_font_directory(), path.parent()) else {
        return false;
    };
    match (
        std::fs::canonicalize(parent),
        std::fs::canonicalize(&directory),
    ) {
        (Ok(parent), Ok(directory)) => parent == directory,
        // Without both real directories there is nothing safe to compare, and nowhere to write.
        _ => false,
    }
}

/// Unregisters a proven font and moves it into quarantine.
fn take_out_of_service(
    environment: &impl ManagedEnvironment,
    proof: &ProvenOwnership,
    quarantine_path: &Path,
) -> Result<(), UninstallError> {
    environment
        .unregister_font(&proof.registry_value_name, &proof.path)
        .map_err(|_| UninstallError::Platform)?;
    set_aside(proof, quarantine_path)
}

/// Moves the proven file to `destination`, checking on both sides of the move that it is still the
/// file the proof was issued for.
///
/// Verification and removal are separate moments, and a path can come to mean a different file in
/// between. The filesystem's own record for the file settles it: renaming keeps that record, so a
/// file that arrives at the destination with a different one is not the file that was verified,
/// and it goes straight back where it came from.
fn set_aside(proof: &ProvenOwnership, destination: &Path) -> Result<(), UninstallError> {
    let source = File::open(&proof.path).map_err(|_| UninstallError::Changed)?;
    if !still_the_proven_file(proof, &source) {
        return Err(UninstallError::Changed);
    }

    if std::fs::rename(&proof.path, destination).is_ok() {
        let moved = File::open(destination).map_err(|_| UninstallError::Quarantine)?;
        if !still_the_proven_file(proof, &moved) {
            drop(moved);
            let _ = std::fs::rename(destination, &proof.path);
            return Err(UninstallError::Changed);
        }
        return Ok(());
    }

    // Renaming only fails across volumes, which a redirected application data directory can
    // produce. Copy first, confirm the original is still the proven file, and only then remove it.
    std::fs::copy(&proof.path, destination).map_err(|_| UninstallError::Quarantine)?;
    let still_there = File::open(&proof.path).map_err(|_| UninstallError::Changed)?;
    if !still_the_proven_file(proof, &still_there) {
        let _ = std::fs::remove_file(destination);
        return Err(UninstallError::Changed);
    }
    drop(still_there);
    drop(source);
    std::fs::remove_file(&proof.path).map_err(|_| UninstallError::Platform)
}

/// Where one font is set aside. The managed file name is already unique per artifact, and the
/// directory is unique per operation, so nothing an uninstall quarantines can overwrite anything
/// an earlier one did.
fn quarantine_path(directory: &Path, proof: &ProvenOwnership) -> PathBuf {
    let name = proof.path.file_name().map_or_else(
        || proof.content_hash.clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    directory.join(name)
}

/// An operation ID as a directory name. The ID reads well in a log and separates its parts with
/// colons, which Windows will not have in a file name, so the name keeps only what is safe.
fn directory_name(operation_id: &str) -> String {
    operation_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

/// An identifier no other operation can be holding: the family, the moment, and this process.
fn new_operation_id(family_id: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("uninstall:{family_id}:{now}:{}", std::process::id())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use super::{
        QUARANTINE_DIRECTORY, RestoreRefusal, UninstallOutcome, restore_quarantined_font,
        restore_uninstalled_font, uninstall_family,
    };
    use crate::font_platform::{FontPlatformError, managed_file_name, validate_font};
    use crate::managed_installations::{
        ManagedInstallationRecord, ManagedInstallationRepository, OperationKind, OperationStep,
    };
    use crate::managed_ownership::{
        ManagedEnvironment, OwnershipRefusal, ProviderArtifact, content_hash,
    };

    /// The same stand-in the authorization tests use: a font Windows ships, so the checks run on
    /// real bytes without the repository having to carry a font it would need to license.
    const SYSTEM_FONT: &str = r"C:\Windows\Fonts\arial.ttf";
    const PROVIDER: &str = "google-fonts";
    const FAMILY: &str = "gf:test";
    const ARTIFACT: &str = "gf:test:regular";

    /// A computer a test controls, which also records every registration change asked of it.
    struct FakeEnvironment {
        font_directory: PathBuf,
        artifacts: HashMap<String, ProviderArtifact>,
        registry: RefCell<HashMap<String, String>>,
        unregistered: RefCell<Vec<String>>,
        registered: RefCell<Vec<String>>,
    }

    impl ManagedEnvironment for FakeEnvironment {
        fn user_font_directory(&self) -> Option<PathBuf> {
            Some(self.font_directory.clone())
        }

        fn provider_artifact(&self, provider: &str, artifact_id: &str) -> Option<ProviderArtifact> {
            if provider != PROVIDER {
                return None;
            }
            self.artifacts.get(artifact_id).cloned()
        }

        fn registered_font_path(&self, value_name: &str) -> Option<String> {
            self.registry.borrow().get(value_name).cloned()
        }

        fn unregister_font(
            &self,
            registry_value_name: &str,
            _path: &Path,
        ) -> Result<(), FontPlatformError> {
            self.registry.borrow_mut().remove(registry_value_name);
            self.unregistered
                .borrow_mut()
                .push(registry_value_name.to_owned());
            Ok(())
        }

        fn register_font(
            &self,
            registry_value_name: &str,
            path: &Path,
        ) -> Result<(), FontPlatformError> {
            self.registry.borrow_mut().insert(
                registry_value_name.to_owned(),
                path.to_string_lossy().into_owned(),
            );
            self.registered
                .borrow_mut()
                .push(registry_value_name.to_owned());
            Ok(())
        }
    }

    struct Installation {
        environment: FakeEnvironment,
        repository: ManagedInstallationRepository,
        app_data_dir: PathBuf,
        path: PathBuf,
        registry_value_name: String,
        bytes: Vec<u8>,
    }

    /// One recorded, installed font: the file in a directory shaped like the per-user font
    /// directory, the registration pointing at it, and a committed ledger row claiming it.
    fn install(root: &Path) -> Option<Installation> {
        let bytes = std::fs::read(SYSTEM_FONT).ok()?; // Not every machine has this font.
        let font = validate_font(&bytes).expect("a system font parses");
        let font_directory = root
            .join("AppData")
            .join("Local")
            .join("Microsoft")
            .join("Windows")
            .join("Fonts");
        std::fs::create_dir_all(&font_directory).expect("the user font directory");
        let app_data_dir = root.join("AppData").join("Roaming").join("FontNest");
        std::fs::create_dir_all(&app_data_dir).expect("the application data directory");

        let hash = content_hash(&bytes);
        let path = font_directory
            .join(managed_file_name("Test-Regular.ttf", &hash).expect("a managed file name"));
        std::fs::write(&path, &bytes).expect("the installed font");
        let registry_value_name = format!("{} (TrueType)", font.full_name);

        let repository = ManagedInstallationRepository::in_app_data_dir(&app_data_dir);
        repository.initialize().expect("the migrations");
        repository
            .commit_operation(
                "install:test",
                &[ManagedInstallationRecord {
                    id: format!("{PROVIDER}:{ARTIFACT}"),
                    provider: PROVIDER.to_owned(),
                    family_id: FAMILY.to_owned(),
                    artifact_id: ARTIFACT.to_owned(),
                    family_name: "Test".to_owned(),
                    display_name: font.full_name.clone(),
                    source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
                    source_hash: hash.clone(),
                    installed_path: path.to_string_lossy().into_owned(),
                    registry_value_name: registry_value_name.clone(),
                    license: "OFL-1.1".to_owned(),
                    license_path: app_data_dir
                        .join("licenses")
                        .join("test.txt")
                        .to_string_lossy()
                        .into_owned(),
                    operation_id: "install:test".to_owned(),
                }],
            )
            .expect("the ledger row");

        let mut artifacts = HashMap::new();
        artifacts.insert(
            ARTIFACT.to_owned(),
            ProviderArtifact {
                file_name: "Test-Regular.ttf".to_owned(),
                content_hash: hash,
                size_bytes: u64::try_from(bytes.len()).expect("a font that fits"),
            },
        );
        let mut registry = HashMap::new();
        registry.insert(
            registry_value_name.clone(),
            path.to_string_lossy().into_owned(),
        );

        Some(Installation {
            environment: FakeEnvironment {
                font_directory,
                artifacts,
                registry: RefCell::new(registry),
                unregistered: RefCell::new(Vec::new()),
                registered: RefCell::new(Vec::new()),
            },
            repository,
            app_data_dir,
            path,
            registry_value_name,
            bytes,
        })
    }

    /// Sets up one installed font, or leaves the test unrun on a machine without the system font
    /// that stands in for an artifact.
    macro_rules! installed {
        ($temp:ident) => {{
            let $temp = tempfile::tempdir().expect("a temporary directory");
            let Some(installed) = install($temp.path()) else {
                return;
            };
            ($temp, installed)
        }};
    }

    /// The one file now sitting in quarantine.
    fn quarantined(app_data_dir: &Path) -> Option<PathBuf> {
        let mut found = Vec::new();
        for operation in std::fs::read_dir(app_data_dir.join(QUARANTINE_DIRECTORY)).ok()? {
            let operation = operation.ok()?;
            for file in std::fs::read_dir(operation.path()).ok()? {
                found.push(file.ok()?.path());
            }
        }
        found.pop()
    }

    /// Edits the ledger the way somebody with the file open could.
    fn rewrite_installed_path(installed: &Installation, path: &str) {
        let connection =
            rusqlite::Connection::open(installed.app_data_dir.join("fontnest.sqlite3"))
                .expect("the ledger opens");
        connection
            .execute(
                "UPDATE managed_installations SET installed_path = ?1",
                rusqlite::params![path],
            )
            .expect("the tampered row");
    }

    #[test]
    fn a_removed_font_is_set_aside_rather_than_deleted() {
        let (_temp, installed) = installed!(temp);

        let outcome = uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &[],
            &installed.environment,
        )
        .expect("a proven font is FontNest's to remove");

        assert_eq!(outcome.removed.len(), 1);
        assert!(outcome.refused.is_empty());
        assert!(!installed.path.exists(), "the font leaves the font folder");
        let set_aside = quarantined(&installed.app_data_dir).expect("the quarantined file");
        assert_eq!(
            std::fs::read(&set_aside).expect("the quarantined bytes"),
            installed.bytes,
            "removal keeps the font, so a removal that was wrong can be undone"
        );
        assert_eq!(
            installed.environment.unregistered.borrow().as_slice(),
            std::slice::from_ref(&installed.registry_value_name)
        );
        assert!(
            installed
                .repository
                .installations(PROVIDER, FAMILY)
                .expect("the ledger")
                .is_empty(),
            "the ledger stops claiming a font it no longer has"
        );
        assert!(
            installed
                .repository
                .interrupted_operations()
                .expect("the journal")
                .is_empty(),
            "a committed removal leaves nothing for the next launch to undo"
        );
    }

    #[test]
    fn an_edited_ledger_cannot_take_away_a_file_fontnest_never_installed() {
        let (temp, installed) = installed!(temp);
        let victim = temp.path().join("someone-elses-document.ttf");
        std::fs::write(&victim, &installed.bytes).expect("a file that is not ours");
        // Exactly what somebody with access to the SQLite file would write.
        rewrite_installed_path(&installed, &victim.to_string_lossy());

        let outcome = uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &[],
            &installed.environment,
        )
        .expect("a refusal is an outcome, not a failure");

        assert!(outcome.removed.is_empty());
        assert_eq!(
            outcome
                .refused
                .iter()
                .map(|font| font.reason)
                .collect::<Vec<_>>(),
            vec![OwnershipRefusal::LedgerPathMismatch]
        );
        assert!(victim.exists(), "the named file is untouched");
        assert!(installed.path.exists(), "and so is the real installation");
        assert!(
            installed.environment.unregistered.borrow().is_empty(),
            "nothing was unregistered either"
        );
        assert!(
            quarantined(&installed.app_data_dir).is_none(),
            "a refused removal never even opens an operation"
        );
    }

    #[test]
    fn a_font_someone_replaced_is_reported_and_left_alone() {
        let (_temp, installed) = installed!(temp);
        let mut replaced = installed.bytes.clone();
        let last = replaced.len() - 1;
        replaced[last] ^= 0xff;
        std::fs::write(&installed.path, &replaced).expect("the replaced font");

        let outcome = uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &[],
            &installed.environment,
        )
        .expect("a refusal is an outcome, not a failure");

        assert_eq!(
            outcome
                .refused
                .iter()
                .map(|font| font.reason)
                .collect::<Vec<_>>(),
            vec![OwnershipRefusal::ContentMismatch]
        );
        assert!(installed.path.exists());
        assert_eq!(
            installed
                .repository
                .installations(PROVIDER, FAMILY)
                .expect("the ledger")
                .len(),
            1,
            "a font FontNest would not remove is still recorded as installed"
        );
    }

    #[test]
    fn an_artifact_the_request_did_not_name_is_left_installed() {
        let (_temp, installed) = installed!(temp);

        let outcome = uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &["gf:test:italic".to_owned()],
            &installed.environment,
        )
        .expect("naming another artifact removes nothing");

        assert_eq!(outcome, UninstallOutcome::default());
        assert!(installed.path.exists());
    }

    #[test]
    fn a_removal_interrupted_before_it_committed_is_put_back() {
        let (_temp, installed) = installed!(temp);
        let quarantine = installed.app_data_dir.join(QUARANTINE_DIRECTORY);
        std::fs::create_dir_all(&quarantine).expect("the quarantine directory");
        let set_aside = quarantine.join("FontNest-set-aside.ttf");
        std::fs::rename(&installed.path, &set_aside).expect("the font is taken away");
        installed
            .environment
            .registry
            .borrow_mut()
            .remove(&installed.registry_value_name);
        let step = OperationStep {
            artifact_id: ARTIFACT.to_owned(),
            display_name: "Test Regular".to_owned(),
            installed_path: installed.path.to_string_lossy().into_owned(),
            registry_value_name: installed.registry_value_name.clone(),
            quarantine_path: set_aside.to_string_lossy().into_owned(),
        };

        restore_uninstalled_font(&installed.environment, &step).expect("the font goes back");

        assert!(installed.path.is_file(), "the file is where it was");
        assert_eq!(
            std::fs::read(&installed.path).expect("the restored bytes"),
            installed.bytes
        );
        assert!(!set_aside.exists(), "and it is no longer in quarantine");
        assert_eq!(
            installed.environment.registered.borrow().as_slice(),
            std::slice::from_ref(&installed.registry_value_name),
            "a restored font is registered again"
        );
    }

    #[test]
    fn a_restore_will_not_write_outside_the_font_folder() {
        let (temp, installed) = installed!(temp);
        let elsewhere = temp.path().join("FontNest-elsewhere.ttf");
        let set_aside = installed.app_data_dir.join("FontNest-set-aside.ttf");
        std::fs::write(&set_aside, &installed.bytes).expect("the quarantined font");
        let step = OperationStep {
            artifact_id: ARTIFACT.to_owned(),
            display_name: "Test Regular".to_owned(),
            installed_path: elsewhere.to_string_lossy().into_owned(),
            registry_value_name: installed.registry_value_name.clone(),
            quarantine_path: set_aside.to_string_lossy().into_owned(),
        };

        let error = restore_uninstalled_font(&installed.environment, &step)
            .expect_err("a journal is a record, not permission to write anywhere");

        assert!(matches!(error, FontPlatformError::TargetConflict));
        assert!(!elsewhere.exists());
        assert!(set_aside.exists());
    }

    #[test]
    fn a_restore_does_not_overwrite_a_font_that_is_already_back() {
        let (_temp, installed) = installed!(temp);
        let set_aside = installed.app_data_dir.join("FontNest-set-aside.ttf");
        std::fs::write(&set_aside, b"not the installed font").expect("a stale quarantined file");
        let step = OperationStep {
            artifact_id: ARTIFACT.to_owned(),
            display_name: "Test Regular".to_owned(),
            installed_path: installed.path.to_string_lossy().into_owned(),
            registry_value_name: installed.registry_value_name.clone(),
            quarantine_path: set_aside.to_string_lossy().into_owned(),
        };

        restore_uninstalled_font(&installed.environment, &step)
            .expect("repeating a restore is safe");

        assert_eq!(
            std::fs::read(&installed.path).expect("the font on disk"),
            installed.bytes,
            "the font already in place is the one that stays"
        );
    }

    #[test]
    fn an_interrupted_removal_knows_where_it_put_the_font() {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let repository = ManagedInstallationRepository::in_app_data_dir(temp.path());
        repository.initialize().expect("the migrations");
        let step = OperationStep {
            artifact_id: ARTIFACT.to_owned(),
            display_name: "Test Regular".to_owned(),
            installed_path: r"C:\Fonts\FontNest-abc-Test.ttf".to_owned(),
            registry_value_name: "Test Regular (TrueType)".to_owned(),
            quarantine_path: r"C:\Quarantine\FontNest-abc-Test.ttf".to_owned(),
        };
        repository
            .begin_operation(
                "uninstall:test",
                OperationKind::Uninstall,
                PROVIDER,
                &[step],
            )
            .expect("the journal write");

        let open = repository.interrupted_operations().expect("the journal");

        assert_eq!(open.len(), 1);
        assert_eq!(open[0].kind, OperationKind::Uninstall);
        assert_eq!(
            open[0].steps[0].quarantine_path,
            r"C:\Quarantine\FontNest-abc-Test.ttf"
        );
    }

    // Removal sets a font aside rather than deleting it, which is only a kindness if it can
    // actually be undone. This is the whole round trip: out of service, then back into it.
    #[test]
    fn a_font_set_aside_can_be_put_back() {
        let (_temp, installed) = installed!(temp);
        uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &[],
            &installed.environment,
        )
        .expect("a proven font is FontNest's to remove");
        assert!(!installed.path.exists(), "the font left the font folder");

        let set_aside = installed
            .repository
            .quarantined_fonts()
            .expect("the quarantine")
            .into_iter()
            .next()
            .expect("the removal recorded what it set aside");

        restore_quarantined_font(&installed.repository, &set_aside, &installed.environment)
            .expect("a font FontNest set aside is one it can put back");

        assert!(installed.path.exists(), "the file is back where it was");
        assert_eq!(
            std::fs::read(&installed.path).expect("the restored bytes"),
            installed.bytes,
            "and holds what it held before"
        );
        assert_eq!(
            installed.environment.registered.borrow().as_slice(),
            std::slice::from_ref(&installed.registry_value_name),
            "and is registered again under the same name"
        );
        assert_eq!(
            installed
                .repository
                .all_installations()
                .expect("the ledger")
                .len(),
            1,
            "the ledger claims it again"
        );
        assert!(
            installed
                .repository
                .quarantined_fonts()
                .expect("the quarantine")
                .is_empty(),
            "and stops offering it back"
        );
    }

    // The quarantine lives in the application's own data directory, which the person can edit. A
    // restore that did not check would register whatever happens to be sitting there.
    #[test]
    fn a_quarantined_file_somebody_swapped_is_not_put_back() {
        let (_temp, installed) = installed!(temp);
        uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &[],
            &installed.environment,
        )
        .expect("the removal");

        let set_aside = installed
            .repository
            .quarantined_fonts()
            .expect("the quarantine")
            .into_iter()
            .next()
            .expect("one set aside");
        std::fs::write(
            &set_aside.quarantine_path,
            b"not the font that was taken away",
        )
        .expect("the swap");

        let refusal =
            restore_quarantined_font(&installed.repository, &set_aside, &installed.environment)
                .expect_err("bytes that are not the ones set aside are not put back");

        assert_eq!(refusal, RestoreRefusal::ContentMismatch);
        assert!(
            !installed.path.exists(),
            "and nothing reaches the font folder"
        );
        assert_eq!(
            installed
                .repository
                .quarantined_fonts()
                .expect("the quarantine")
                .len(),
            1,
            "the record stays, so the mistake can still be looked at"
        );
    }

    #[test]
    fn a_font_already_installed_again_is_not_put_back_over_itself() {
        let (_temp, installed) = installed!(temp);
        uninstall_family(
            &installed.repository,
            &installed.app_data_dir,
            PROVIDER,
            FAMILY,
            &[],
            &installed.environment,
        )
        .expect("the removal");
        let set_aside = installed
            .repository
            .quarantined_fonts()
            .expect("the quarantine")
            .into_iter()
            .next()
            .expect("one set aside");
        std::fs::write(&installed.path, &installed.bytes).expect("the font is back by other means");

        let refusal =
            restore_quarantined_font(&installed.repository, &set_aside, &installed.environment)
                .expect_err("there is nothing to put back");

        assert_eq!(refusal, RestoreRefusal::AlreadyInstalled);
    }
}
