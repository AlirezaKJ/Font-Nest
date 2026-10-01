//! Importing fonts the person already has, starting with what would happen if they did.
//!
//! Installing a font changes the computer, so nothing here is decided from a file name or from
//! anything the interface says about a file. Every candidate is opened, parsed and measured, and
//! the verdict comes from the bytes. The same work is done again when an import actually runs, so
//! a file that changed between being reviewed and being imported is refused rather than installed
//! on the strength of an older reading.
//!
//! Paths arrive from the interface because the person chose them in a file dialog, which is the
//! same way a preview already reaches `FontNest`. They are treated as a question, never as an
//! instruction: a path that names something protected, unreadable, or not a font this platform
//! installs comes back as a refusal with the reason, and the only thing an import ever writes is
//! a copy inside the per-user font directory under a name `FontNest` derived itself.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ttf_parser::{Face, PlatformId, name_id};

use crate::font_platform::{self, FontPlatformError, InstallableFormat};
use crate::managed_installations::{
    ManagedInstallationRecord, ManagedInstallationRepository, OperationKind, OperationStep,
};
// The same digest the ownership proof verifies against, so a font FontNest imports is one it can
// still prove is its own afterwards.
use crate::managed_ownership::content_hash;

/// Largest single file `FontNest` will read for an import. Desktop fonts are far smaller; this is
/// a bound on what a chosen file can make the application allocate.
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;

/// Most files one import may review at once.
const MAX_CANDIDATES: usize = 500;

/// How far into a chosen folder the search goes. Font folders are shallow, and an unbounded walk
/// is a way to make one click read an entire drive.
const MAX_FOLDER_DEPTH: usize = 4;

/// Extensions worth opening at all. Anything else in a chosen folder is simply not a font and is
/// passed over rather than reported as a failure.
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc", "otc", "woff", "woff2", "pfb", "dfont"];

/// What `FontNest` would do with one chosen file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum ImportVerdict {
    /// A font this platform installs, not already installed, ready to go.
    Installable,
    /// `FontNest` already installed exactly these bytes.
    AlreadyInstalled,
    /// The same bytes appear more than once in this selection; the first one stands.
    RepeatedSelection,
    /// A real font, but not a format this platform installs. It can still be previewed.
    PreviewOnly,
    /// A font file that will not parse, or one whose naming metadata is unusable.
    Unreadable,
    /// Larger than `FontNest` will read.
    TooLarge,
    /// The file is one the operating system owns. Fonts are never imported out of a system
    /// directory: they are already installed, and copying them would claim them as ours.
    SystemOwned,
    /// The file could not be opened at all.
    Missing,
}

impl ImportVerdict {
    #[must_use]
    pub const fn is_installable(self) -> bool {
        matches!(self, Self::Installable)
    }
}

/// What a font says about who may use it and how. Read from the font, never from a file name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontLicenceSummary {
    /// What the font's own licence field says, trimmed and capped for display.
    pub description: Option<String>,
    pub url: Option<String>,
    /// What the embedding permissions in the font allow, in the font's own terms.
    pub embedding: EmbeddingPermission,
}

/// The OS/2 embedding permission, which is the only licence statement a font makes in a form a
/// program can act on. `FontNest` reports it and never decides anything from it: installing a font
/// on the computer it came to is not embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum EmbeddingPermission {
    Installable,
    Restricted,
    PreviewAndPrint,
    Editable,
    Unknown,
}

/// One face inside a chosen file, as the parser read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ImportFaceSummary {
    pub family_name: String,
    pub subfamily_name: String,
    pub full_name: String,
    pub post_script_name: String,
    pub version: Option<String>,
    pub weight: u16,
    pub italic: bool,
    pub glyph_count: u16,
}

/// One chosen file and everything the review found in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ImportCandidate {
    /// The path the person chose, for them to recognize the file by. It is display only: an
    /// import re-reads and re-decides everything from the file itself.
    pub source_path: String,
    pub file_name: String,
    /// For display. Saturated rather than widened: this crosses to an interface that shows it as
    /// "about 2 MB", and a number that large is already a refusal for being too large.
    pub size_bytes: u32,
    pub verdict: ImportVerdict,
    /// The name the file would be installed under, when it would be installed at all.
    pub installed_file_name: Option<String>,
    pub faces: Vec<ImportFaceSummary>,
    pub licence: Option<FontLicenceSummary>,
}

/// Everything one review found, in the order the files were chosen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ImportPlan {
    pub candidates: Vec<ImportCandidate>,
    /// Total bytes of the files that would be installed, for display.
    pub installable_bytes: u32,
    /// True when the selection was cut off at the limit rather than read whole.
    pub truncated: bool,
}

/// What one file's import actually did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ImportOutcome {
    pub source_path: String,
    pub file_name: String,
    pub installed: bool,
    /// The family and style now installed, when it was.
    pub display_name: Option<String>,
    /// Why it was not installed, in the same words the review would have used.
    pub refusal: Option<ImportVerdict>,
    /// Set when the import failed for a reason the review could not have seen, such as the
    /// file changing underneath it or the operating system refusing the registration.
    pub failure: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("the per-user font directory is unavailable")]
    Platform(#[from] FontPlatformError),
    #[error("the managed installation ledger is unavailable")]
    Database,
}

/// Expands what the person chose into the files worth reviewing.
///
/// A chosen file is taken as chosen even when its extension is unfamiliar, because they picked it
/// deliberately. A chosen folder is searched instead, shallowly and only for names that look like
/// fonts, so pointing at a downloads folder does not turn into reading everything in it.
#[must_use]
pub fn collect_candidates(chosen: &[PathBuf]) -> (Vec<PathBuf>, bool) {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut truncated = false;

    for path in chosen {
        if found.len() >= MAX_CANDIDATES {
            truncated = true;
            break;
        }
        if path.is_dir() {
            collect_from_folder(path, 0, &mut found, &mut seen, &mut truncated);
        } else if seen.insert(path.clone()) {
            found.push(path.clone());
        }
    }

    (found, truncated)
}

fn collect_from_folder(
    folder: &Path,
    depth: usize,
    found: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    truncated: &mut bool,
) {
    if depth > MAX_FOLDER_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    let mut folders = Vec::new();
    for entry in entries.flatten() {
        if found.len() >= MAX_CANDIDATES {
            *truncated = true;
            return;
        }
        let path = entry.path();
        // Never walk into a link: a folder that points elsewhere is a way to leave the folder
        // the person actually chose.
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            folders.push(path);
        } else if looks_like_a_font(&path) && seen.insert(path.clone()) {
            found.push(path);
        }
    }

    for folder in folders {
        if found.len() >= MAX_CANDIDATES {
            *truncated = true;
            return;
        }
        collect_from_folder(&folder, depth + 1, found, seen, truncated);
    }
}

fn looks_like_a_font(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            let lowered = extension.to_ascii_lowercase();
            FONT_EXTENSIONS.contains(&lowered.as_str())
        })
}

/// Reviews one file's bytes and decides what would happen to it.
///
/// Separated from the filesystem so every verdict can be tested against bytes rather than against
/// a machine in a particular state.
#[must_use]
pub fn review_bytes(
    file_name: &str,
    bytes: &[u8],
    already_installed: &HashSet<String>,
    seen_hashes: &mut HashSet<String>,
) -> (
    ImportVerdict,
    Vec<ImportFaceSummary>,
    Option<FontLicenceSummary>,
    Option<String>,
) {
    let faces = read_faces(bytes);
    let licence = read_licence(bytes);

    // Unreadable comes first: a file that will not parse has nothing else worth reporting, and
    // whether this platform installs its extension is beside the point.
    if faces.is_empty() {
        return (ImportVerdict::Unreadable, faces, licence, None);
    }

    let Some(format) = InstallableFormat::of_file_name(file_name) else {
        return (ImportVerdict::PreviewOnly, faces, licence, None);
    };
    // A collection carries several faces that the operating system registers together under names
    // from inside the file. That is a different operation, so it is previewed rather than guessed.
    if faces.len() > 1 {
        return (ImportVerdict::PreviewOnly, faces, licence, None);
    }
    let _ = format;

    let hash = content_hash(bytes);
    if already_installed.contains(&hash) {
        return (ImportVerdict::AlreadyInstalled, faces, licence, None);
    }
    if !seen_hashes.insert(hash.clone()) {
        return (ImportVerdict::RepeatedSelection, faces, licence, None);
    }

    match font_platform::managed_file_name(file_name, &hash) {
        Ok(installed_file_name) => (
            ImportVerdict::Installable,
            faces,
            licence,
            Some(installed_file_name),
        ),
        // A name with nothing safe left in it cannot be installed under any name FontNest would
        // recognize again, which is the same problem as unusable metadata.
        Err(_) => (ImportVerdict::Unreadable, faces, licence, None),
    }
}

/// Reads every face in the file, or nothing when it is not a font this build can parse.
fn read_faces(bytes: &[u8]) -> Vec<ImportFaceSummary> {
    if bytes.len() < 4 {
        return Vec::new();
    }
    let face_count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    let mut faces = Vec::new();
    for index in 0..face_count {
        let Ok(face) = Face::parse(bytes, index) else {
            return Vec::new();
        };
        if face.number_of_glyphs() == 0 {
            return Vec::new();
        }
        let Some(family_name) = unicode_name(&face, name_id::TYPOGRAPHIC_FAMILY)
            .or_else(|| unicode_name(&face, name_id::FAMILY))
        else {
            return Vec::new();
        };
        let Some(post_script_name) = unicode_name(&face, name_id::POST_SCRIPT_NAME) else {
            return Vec::new();
        };
        let subfamily_name = unicode_name(&face, name_id::TYPOGRAPHIC_SUBFAMILY)
            .or_else(|| unicode_name(&face, name_id::SUBFAMILY))
            .unwrap_or_else(|| "Regular".to_owned());
        faces.push(ImportFaceSummary {
            full_name: unicode_name(&face, name_id::FULL_NAME)
                .unwrap_or_else(|| format!("{family_name} {subfamily_name}")),
            version: unicode_name(&face, name_id::VERSION),
            weight: face.weight().to_number(),
            italic: face.is_italic(),
            glyph_count: face.number_of_glyphs(),
            family_name,
            subfamily_name,
            post_script_name,
        });
    }
    faces
}

fn read_licence(bytes: &[u8]) -> Option<FontLicenceSummary> {
    let face = Face::parse(bytes, 0).ok()?;
    let embedding = match face.permissions() {
        Some(ttf_parser::Permissions::Installable) => EmbeddingPermission::Installable,
        Some(ttf_parser::Permissions::Restricted) => EmbeddingPermission::Restricted,
        Some(ttf_parser::Permissions::PreviewAndPrint) => EmbeddingPermission::PreviewAndPrint,
        Some(ttf_parser::Permissions::Editable) => EmbeddingPermission::Editable,
        None => EmbeddingPermission::Unknown,
    };
    Some(FontLicenceSummary {
        description: unicode_name(&face, name_id::LICENSE).map(|value| capped(&value)),
        url: unicode_name(&face, name_id::LICENSE_URL).map(|value| capped(&value)),
        embedding,
    })
}

/// Names are read from the font, so their length is the font's choice rather than ours.
fn capped(value: &str) -> String {
    const MAX_CHARS: usize = 600;
    if value.chars().count() <= MAX_CHARS {
        return value.to_owned();
    }
    value.chars().take(MAX_CHARS).collect()
}

fn unicode_name(face: &Face<'_>, name_id: u16) -> Option<String> {
    face.names()
        .into_iter()
        .filter(|name| {
            name.name_id == name_id
                && (name.is_unicode() || name.platform_id == PlatformId::Windows)
        })
        .find_map(|name| name.to_string())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// Whether a chosen file is one the operating system owns.
///
/// Importing out of a system font directory would copy a font the computer already has and record
/// it as one `FontNest` placed, which is a claim it has no business making.
fn is_system_owned(path: &Path) -> bool {
    let Ok(canonical) = fs::canonicalize(path) else {
        return false;
    };
    system_font_directories()
        .iter()
        .filter_map(|directory| fs::canonicalize(directory).ok())
        .any(|directory| canonical.starts_with(&directory))
}

#[cfg(windows)]
fn system_font_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(windows) = std::env::var_os("SystemRoot") {
        directories.push(PathBuf::from(windows).join("Fonts"));
    }
    directories
}

#[cfg(not(windows))]
fn system_font_directories() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts",
        "/Library/Fonts",
        "/usr/share/fonts",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

/// Reviews everything the person chose, without changing anything.
#[must_use]
pub fn review(chosen: &[PathBuf], already_installed: &HashSet<String>) -> ImportPlan {
    let (paths, truncated) = collect_candidates(chosen);
    let mut seen_hashes = HashSet::new();
    let mut candidates = Vec::with_capacity(paths.len());

    for path in paths {
        candidates.push(review_path(&path, already_installed, &mut seen_hashes));
    }

    let installable_bytes = candidates
        .iter()
        .filter(|candidate| candidate.verdict.is_installable())
        .fold(0_u32, |total, candidate| {
            total.saturating_add(candidate.size_bytes)
        });

    ImportPlan {
        candidates,
        installable_bytes,
        truncated,
    }
}

fn review_path(
    path: &Path,
    already_installed: &HashSet<String>,
    seen_hashes: &mut HashSet<String>,
) -> ImportCandidate {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let source_path = path.to_string_lossy().into_owned();

    let refusal = |verdict: ImportVerdict, size_bytes: u32| ImportCandidate {
        source_path: source_path.clone(),
        file_name: file_name.clone(),
        size_bytes,
        verdict,
        installed_file_name: None,
        faces: Vec::new(),
        licence: None,
    };

    let Ok(metadata) = fs::metadata(path) else {
        return refusal(ImportVerdict::Missing, 0);
    };
    let size_bytes = u32::try_from(metadata.len()).unwrap_or(u32::MAX);
    if metadata.len() > MAX_IMPORT_BYTES {
        return refusal(ImportVerdict::TooLarge, size_bytes);
    }
    if is_system_owned(path) {
        return refusal(ImportVerdict::SystemOwned, size_bytes);
    }
    let Ok(bytes) = fs::read(path) else {
        return refusal(ImportVerdict::Missing, size_bytes);
    };

    let (verdict, faces, licence, installed_file_name) =
        review_bytes(&file_name, &bytes, already_installed, seen_hashes);

    ImportCandidate {
        source_path,
        file_name,
        size_bytes,
        verdict,
        installed_file_name,
        faces,
        licence,
    }
}

/// Everything one file needs to be installed, resolved from its bytes rather than from the review.
pub struct PreparedImport {
    pub path: PathBuf,
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub hash: String,
    pub family_name: String,
    pub display_name: String,
    pub installation: font_platform::PlatformInstallation,
}

/// Re-reads and re-decides one file at import time.
///
/// The review is a report, not a permission: a file can change, be replaced, or be installed by
/// something else between being looked at and being imported, so everything is derived again here
/// and a file that no longer qualifies is refused rather than installed on an old reading.
///
/// # Errors
///
/// Returns the platform error when the per-user font directory or the target name cannot be
/// resolved.
pub fn prepare(
    path: &Path,
    already_installed: &HashSet<String>,
    seen_hashes: &mut HashSet<String>,
) -> Result<PreparedImport, ImportVerdict> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ImportVerdict::Missing)?
        .to_owned();

    let metadata = fs::metadata(path).map_err(|_| ImportVerdict::Missing)?;
    if metadata.len() > MAX_IMPORT_BYTES {
        return Err(ImportVerdict::TooLarge);
    }
    if is_system_owned(path) {
        return Err(ImportVerdict::SystemOwned);
    }
    let bytes = fs::read(path).map_err(|_| ImportVerdict::Missing)?;

    let (verdict, faces, _, _) = review_bytes(&file_name, &bytes, already_installed, seen_hashes);
    if !verdict.is_installable() {
        return Err(verdict);
    }

    let hash = content_hash(&bytes);
    let metadata = font_platform::validate_font(&bytes).map_err(|_| ImportVerdict::Unreadable)?;
    let installation = font_platform::plan_user_font_installation(&file_name, &hash, &metadata)
        .map_err(|_| ImportVerdict::Unreadable)?;
    let display_name = faces
        .first()
        .map_or_else(|| metadata.full_name.clone(), |face| face.full_name.clone());
    let family_name = faces.first().map_or_else(
        || metadata.family_name.clone(),
        |face| face.family_name.clone(),
    );

    Ok(PreparedImport {
        path: path.to_path_buf(),
        file_name,
        bytes,
        hash,
        family_name,
        display_name,
        installation,
    })
}

/// A journal identifier for one import run, distinct per process and per moment so two runs can
/// never be mistaken for one another during recovery.
fn new_operation_id() -> String {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{LOCAL_PROVIDER}:{started}:{}", std::process::id())
}

/// The provider name a font imported from this computer is recorded under. It is not a provider
/// in the sense Google Fonts is: there is no manifest and no upstream, which is exactly why the
/// ownership proof for these fonts is derived differently. See `managed_ownership`.
pub const LOCAL_PROVIDER: &str = "local";

/// Imports the fonts the person chose, and reports what happened to every one of them.
///
/// The whole intent is written to the operation journal before anything is copied or registered,
/// so a process that dies partway through leaves a record the next launch undoes. Files are
/// independent of each other on purpose: one font that will not install does not undo the ones
/// that did, because importing twenty fonts and losing nineteen to the twentieth is not a
/// service to anybody. What did and did not happen comes back per file.
///
/// # Errors
///
/// Returns [`ImportError::Database`] when the ledger cannot be read or the journal cannot be
/// written, and [`ImportError::Platform`] when the per-user font directory is unavailable.
pub fn import(
    paths: &[PathBuf],
    repository: &ManagedInstallationRepository,
) -> Result<Vec<ImportOutcome>, ImportError> {
    let operation_id = new_operation_id();
    let operation_id = operation_id.as_str();
    let already_installed = repository
        .installed_source_hashes(LOCAL_PROVIDER)
        .map_err(|_| ImportError::Database)?;

    let (paths, _) = collect_candidates(paths);
    let mut seen_hashes = HashSet::new();
    let mut prepared = Vec::new();
    let mut outcomes = Vec::new();

    // Everything is resolved before the computer changes, so the journal can describe the whole
    // operation up front rather than learn it as the work proceeds.
    for path in &paths {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        match prepare(path, &already_installed, &mut seen_hashes) {
            Ok(import) => prepared.push(import),
            Err(refusal) => outcomes.push(ImportOutcome {
                source_path: path.to_string_lossy().into_owned(),
                file_name,
                installed: false,
                display_name: None,
                refusal: Some(refusal),
                failure: None,
            }),
        }
    }

    if prepared.is_empty() {
        return Ok(outcomes);
    }

    let steps = prepared
        .iter()
        .map(|import| OperationStep {
            artifact_id: import.hash.clone(),
            display_name: import.display_name.clone(),
            installed_path: import
                .installation
                .installed_path
                .to_string_lossy()
                .into_owned(),
            registry_value_name: import.installation.registry_value_name.clone(),
            // An installation has nothing to set aside: undoing it removes what it wrote.
            quarantine_path: String::new(),
        })
        .collect::<Vec<_>>();

    repository
        .begin_operation(operation_id, OperationKind::Install, LOCAL_PROVIDER, &steps)
        .map_err(|_| ImportError::Database)?;

    let mut records = Vec::new();
    for import in &prepared {
        match font_platform::install_planned_user_font(&import.bytes, &import.installation) {
            Ok(()) => {
                records.push(ManagedInstallationRecord {
                    id: format!("{LOCAL_PROVIDER}:{}", import.hash),
                    provider: LOCAL_PROVIDER.to_owned(),
                    family_id: import.family_name.clone(),
                    artifact_id: import.hash.clone(),
                    family_name: import.family_name.clone(),
                    display_name: import.display_name.clone(),
                    // There is no upstream revision behind a file from this computer, and
                    // recording the path it came from would only invite trusting it later.
                    source_commit: String::new(),
                    source_hash: import.hash.clone(),
                    installed_path: import
                        .installation
                        .installed_path
                        .to_string_lossy()
                        .into_owned(),
                    registry_value_name: import.installation.registry_value_name.clone(),
                    // A font from this computer carries whatever licence it states inside itself;
                    // FontNest has no separate licence file to preserve beside it.
                    license: String::new(),
                    license_path: String::new(),
                    operation_id: operation_id.to_owned(),
                });
                outcomes.push(ImportOutcome {
                    source_path: import.path.to_string_lossy().into_owned(),
                    file_name: import.file_name.clone(),
                    installed: true,
                    display_name: Some(import.display_name.clone()),
                    refusal: None,
                    failure: None,
                });
            }
            Err(error) => outcomes.push(ImportOutcome {
                source_path: import.path.to_string_lossy().into_owned(),
                file_name: import.file_name.clone(),
                installed: false,
                display_name: None,
                refusal: None,
                failure: Some(error.to_string()),
            }),
        }
    }

    // Closing the journal records what succeeded and drops the intent in one transaction, so the
    // ledger never claims a font that is not there and never forgets one that is.
    repository
        .commit_operation(operation_id, &records)
        .map_err(|_| ImportError::Database)?;

    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::{EmbeddingPermission, ImportVerdict, collect_candidates, review, review_bytes};
    use crate::managed_ownership::content_hash;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// A real font from this computer, so the parser is tested against a font somebody shipped
    /// rather than against bytes a test invented.
    fn a_real_font() -> Vec<u8> {
        let candidates = [
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\segoeui.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ];
        for candidate in candidates {
            if let Ok(bytes) = fs::read(candidate) {
                return bytes;
            }
        }
        panic!("no system font available to test against");
    }

    fn nothing_installed() -> HashSet<String> {
        HashSet::new()
    }

    #[test]
    fn a_font_this_platform_installs_is_installable() {
        let bytes = a_real_font();
        let mut seen = HashSet::new();

        let (verdict, faces, licence, installed_file_name) =
            review_bytes("Imported.ttf", &bytes, &nothing_installed(), &mut seen);

        assert_eq!(verdict, ImportVerdict::Installable);
        assert!(!faces.is_empty());
        assert!(!faces[0].family_name.is_empty());
        assert!(!faces[0].post_script_name.is_empty());
        assert!(licence.is_some());
        assert!(
            installed_file_name.is_some_and(|name| name.starts_with("FontNest-")),
            "an installable font knows the name it would take"
        );
    }

    #[test]
    fn bytes_that_are_not_a_font_are_unreadable() {
        let mut seen = HashSet::new();

        let (verdict, faces, _, _) = review_bytes(
            "Imported.ttf",
            b"this is not a font at all",
            &nothing_installed(),
            &mut seen,
        );

        assert_eq!(verdict, ImportVerdict::Unreadable);
        assert!(faces.is_empty());
    }

    #[test]
    fn an_empty_file_is_unreadable() {
        let mut seen = HashSet::new();

        let (verdict, _, _, _) = review_bytes("Empty.ttf", b"", &nothing_installed(), &mut seen);

        assert_eq!(verdict, ImportVerdict::Unreadable);
    }

    // A real font in a format Windows does not install: previewing it is honest, installing it
    // would not work.
    #[test]
    fn a_format_this_platform_does_not_install_is_preview_only() {
        let bytes = a_real_font();
        let mut seen = HashSet::new();

        let (verdict, faces, _, installed) =
            review_bytes("Imported.woff2", &bytes, &nothing_installed(), &mut seen);

        assert_eq!(verdict, ImportVerdict::PreviewOnly);
        assert!(
            !faces.is_empty(),
            "a preview-only font still reports what is in it"
        );
        assert_eq!(installed, None);
    }

    #[test]
    fn bytes_already_installed_are_reported_rather_than_installed_twice() {
        let bytes = a_real_font();
        let installed = HashSet::from([content_hash(&bytes)]);
        let mut seen = HashSet::new();

        let (verdict, _, _, _) = review_bytes("Imported.ttf", &bytes, &installed, &mut seen);

        assert_eq!(verdict, ImportVerdict::AlreadyInstalled);
    }

    #[test]
    fn the_same_font_chosen_twice_is_only_installed_once() {
        let bytes = a_real_font();
        let mut seen = HashSet::new();

        let (first, _, _, _) =
            review_bytes("Imported.ttf", &bytes, &nothing_installed(), &mut seen);
        let (second, _, _, _) = review_bytes("Copy.ttf", &bytes, &nothing_installed(), &mut seen);

        assert_eq!(first, ImportVerdict::Installable);
        assert_eq!(second, ImportVerdict::RepeatedSelection);
    }

    #[test]
    fn a_licence_is_read_from_the_font_rather_than_guessed() {
        let bytes = a_real_font();
        let mut seen = HashSet::new();

        let (_, _, licence, _) =
            review_bytes("Imported.ttf", &bytes, &nothing_installed(), &mut seen);
        let licence = licence.expect("a font states its embedding permission");

        assert_ne!(licence.embedding, EmbeddingPermission::Unknown);
    }

    #[test]
    fn a_chosen_file_is_reviewed_whatever_its_extension() {
        let directory = tempdir().expect("a temporary directory");
        let chosen = directory.path().join("no-extension");
        fs::write(&chosen, a_real_font()).expect("the fixture writes");

        let (found, truncated) = collect_candidates(std::slice::from_ref(&chosen));

        assert_eq!(found, vec![chosen]);
        assert!(!truncated);
    }

    #[test]
    fn a_chosen_folder_gives_up_the_fonts_in_it_and_nothing_else() {
        let directory = tempdir().expect("a temporary directory");
        let fonts = directory.path().join("fonts");
        fs::create_dir_all(fonts.join("weights")).expect("the fixture directories");
        fs::write(fonts.join("One.ttf"), a_real_font()).expect("a font");
        fs::write(fonts.join("readme.txt"), "not a font").expect("a decoy");
        fs::write(fonts.join("weights").join("Two.otf"), a_real_font()).expect("a nested font");

        let (found, _) = collect_candidates(std::slice::from_ref(&fonts));
        let names = found
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect::<HashSet<_>>();

        assert_eq!(names, HashSet::from(["One.ttf", "Two.otf"]));
    }

    #[test]
    fn a_file_chosen_twice_is_only_reviewed_once() {
        let directory = tempdir().expect("a temporary directory");
        let chosen = directory.path().join("One.ttf");
        fs::write(&chosen, a_real_font()).expect("the fixture writes");

        let (found, _) = collect_candidates(&[chosen.clone(), chosen.clone()]);

        assert_eq!(found, vec![chosen]);
    }

    #[test]
    fn a_missing_file_is_reported_rather_than_skipped() {
        let plan = review(
            &[PathBuf::from("nowhere-at-all/Imagined.ttf")],
            &nothing_installed(),
        );

        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].verdict, ImportVerdict::Missing);
        assert_eq!(plan.installable_bytes, 0);
    }

    // Fonts the operating system owns are already installed; copying one out of the system folder
    // and recording it as ours would be a claim FontNest has no business making.
    #[cfg(windows)]
    #[test]
    fn a_font_the_operating_system_owns_is_refused() {
        let system_font = PathBuf::from(r"C:\Windows\Fonts\arial.ttf");
        if !system_font.exists() {
            return;
        }

        let plan = review(&[system_font], &nothing_installed());

        assert_eq!(plan.candidates[0].verdict, ImportVerdict::SystemOwned);
    }

    #[test]
    fn a_review_counts_only_what_it_would_install() {
        let directory = tempdir().expect("a temporary directory");
        let font = a_real_font();
        let good = directory.path().join("Good.ttf");
        let bad = directory.path().join("Bad.ttf");
        fs::write(&good, &font).expect("a font");
        fs::write(&bad, b"not a font").expect("a decoy");

        let plan = review(&[good, bad], &nothing_installed());
        let installable = plan
            .candidates
            .iter()
            .filter(|candidate| candidate.verdict.is_installable())
            .count();

        assert_eq!(installable, 1);
        assert_eq!(
            plan.installable_bytes,
            u32::try_from(font.len()).expect("a font that fits")
        );
    }
}
