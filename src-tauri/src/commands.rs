use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fontdb::Source;
use tauri::{Manager, ipc::Channel};
use tauri_plugin_updater::UpdaterExt;

use crate::catalogue::{self, CatalogueInspectionError, FontCatalogueStore};
use crate::dto::{
    AppUpdateEvent, AppUpdateInfo, CommandError, FontCatalogue, FontFaceInspection,
    FontGlyphOutline, FontGlyphOutlineRequest, FontParserJsonEvent, FontParserJsonRequest,
    GoogleFontFamilyDetails, GoogleFontInstallResult, GoogleFontPage, GoogleFontPageRequest,
    GoogleFontPreview, GoogleFontUninstallResult, InstallGoogleFontRequest, ManagedStorageStatus,
    UninstallGoogleFontRequest, ValidatedLocalFont,
};
use crate::font_identity::{IdentityKind, is_well_formed};
use crate::font_inspection::{self, CancelToken, FontInspectionError, ParserJsonSnapshot};
use crate::font_platform;
use crate::google_fonts::{self, GoogleFontsError};
use crate::local_fonts::{self, LocalFontError};
use crate::local_import::{self, ImportOutcome, ImportPlan};
use crate::managed_installations::ManagedInstallationRepository;
use crate::managed_storage::ManagedStorage;
use crate::preferences::{self, LoadedPreferences, Preferences};
use crate::release_notes::{self, ReleaseNotesError};

const MAX_GLYPH_VARIATIONS: usize = 64;
/// Longest caller-generated export ID accepted for a parser snapshot.
const MAX_EXPORT_ID_LENGTH: usize = 64;
/// Bytes per streamed parser-snapshot chunk. A snapshot crosses IPC in pieces this size
/// rather than as one string, so a large document never becomes one oversized payload.
const PARSER_JSON_CHUNK_BYTES: usize = 256 * 1024;
/// Most parser exports allowed to run at once.
const MAX_ACTIVE_PARSER_EXPORTS: usize = 8;

#[derive(Default)]
pub struct CatalogueState {
    store: Mutex<Option<FontCatalogueStore>>,
}

impl CatalogueState {
    fn replace(&self, store: FontCatalogueStore) -> Result<(), CommandError> {
        let mut current = self
            .store
            .lock()
            .map_err(|_| CommandError::catalogue_unavailable())?;
        *current = Some(store);
        Ok(())
    }

    fn inspect_face(&self, face_id: &str) -> Result<FontFaceInspection, CommandError> {
        let current = self
            .store
            .lock()
            .map_err(|_| CommandError::catalogue_unavailable())?;
        let store = current
            .as_ref()
            .ok_or_else(CommandError::catalogue_unavailable)?;
        store
            .inspect_face(face_id)
            .map_err(|error| map_catalogue_inspection_error(&error))
    }

    /// Resolves a face to its source without keeping the catalogue locked for the parse.
    ///
    /// Cloning a `Source` is a path clone or a reference-count bump, so this guard is held
    /// for a lookup rather than for the length of an export.
    fn face_source(&self, face_id: &str) -> Result<(Source, u32), CommandError> {
        let current = self
            .store
            .lock()
            .map_err(|_| CommandError::catalogue_unavailable())?;
        let store = current
            .as_ref()
            .ok_or_else(CommandError::catalogue_unavailable)?;
        store
            .face_source(face_id)
            .map_err(|error| map_catalogue_inspection_error(&error))
    }

    fn face_file_path(&self, face_id: &str) -> Result<std::path::PathBuf, CommandError> {
        let current = self
            .store
            .lock()
            .map_err(|_| CommandError::catalogue_unavailable())?;
        let store = current
            .as_ref()
            .ok_or_else(CommandError::catalogue_unavailable)?;
        store
            .face_file_path(face_id)
            .map_err(|error| map_catalogue_inspection_error(&error))
    }

    fn inspect_glyph_outline(
        &self,
        request: &FontGlyphOutlineRequest,
    ) -> Result<FontGlyphOutline, CommandError> {
        let current = self
            .store
            .lock()
            .map_err(|_| CommandError::catalogue_unavailable())?;
        let store = current
            .as_ref()
            .ok_or_else(CommandError::catalogue_unavailable)?;
        store
            .inspect_glyph_outline(&request.face_id, request.codepoint, &request.variations)
            .map_err(|error| map_catalogue_inspection_error(&error))
    }
}

#[tauri::command]
pub async fn scan_installed_fonts(app: tauri::AppHandle) -> Result<FontCatalogue, CommandError> {
    // The ledger is what makes a font's ID outlive the scan that found it. A session in read-only
    // recovery must not write to it, and scans without it: the IDs are the same ones the ledger
    // would have stored, they just are not recorded for the next launch.
    let ledger = app
        .state::<ManagedStorage>()
        .ensure_writable()
        .ok()
        .and_then(|()| app.path().app_data_dir().ok())
        .map(|app_data_dir| ManagedInstallationRepository::in_app_data_dir(&app_data_dir));

    let scanned = tauri::async_runtime::spawn_blocking(move || {
        catalogue::scan_installed_fonts(ledger.as_ref())
    })
    .await
    .map_err(|_| CommandError::catalogue_unavailable())?;
    app.state::<CatalogueState>().replace(scanned.store)?;
    Ok(scanned.catalogue)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn inspect_font_face(
    face_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<FontFaceInspection, CommandError> {
    ensure_trusted_window(&window)?;
    validate_face_id(&face_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<CatalogueState>().inspect_face(&face_id)
    })
    .await
    .map_err(|_| CommandError::font_parser_unavailable())?
}

/// Cancellation tokens for the parser exports that are still running.
///
/// An export can walk tens of thousands of glyphs, so the user has to be able to abandon
/// one: the frontend names each export and calls `cancel_font_face_parser_export` when it
/// closes the panel or moves to another face.
#[derive(Default, Clone)]
pub struct ParserExports {
    active: Arc<Mutex<HashMap<String, CancelToken>>>,
}

impl ParserExports {
    fn begin(&self, export_id: &str) -> Result<CancelToken, CommandError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| CommandError::font_parser_unavailable())?;
        if active.contains_key(export_id) {
            return Err(CommandError::invalid_parser_export_request());
        }
        if active.len() >= MAX_ACTIVE_PARSER_EXPORTS {
            return Err(CommandError::too_many_parser_exports());
        }
        let token = CancelToken::default();
        active.insert(export_id.to_owned(), token.clone());
        Ok(token)
    }

    fn finish(&self, export_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(export_id);
        }
    }

    fn cancel(&self, export_id: &str) {
        if let Ok(active) = self.active.lock() {
            if let Some(token) = active.get(export_id) {
                token.cancel();
            }
        }
    }
}

/// Keeps an export cancellable until its last chunk has been sent, then deregisters it.
struct ActiveExport {
    exports: ParserExports,
    export_id: String,
    token: CancelToken,
}

impl Drop for ActiveExport {
    fn drop(&mut self) {
        self.exports.finish(&self.export_id);
    }
}

enum ParserExportFailure {
    Cancelled,
    Failed(CommandError),
}

/// Streams a bounded parser snapshot for one face.
///
/// The snapshot is built off the command thread with the catalogue lock released, capped
/// section by section, and delivered as ordered chunks: `started`, then `chunk` messages,
/// then `finished`. A cancelled export ends with `cancelled` and no further chunks.
#[tauri::command]
pub async fn export_font_face_parser_json(
    request: FontParserJsonRequest,
    on_event: Channel<FontParserJsonEvent>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), CommandError> {
    ensure_trusted_window(&window)?;
    validate_face_id(&request.face_id)?;
    validate_export_id(&request.export_id)?;

    let exports = (*app.state::<ParserExports>()).clone();
    let token = exports.begin(&request.export_id)?;
    let active = ActiveExport {
        exports,
        export_id: request.export_id.clone(),
        token,
    };

    let face_id = request.face_id.clone();
    let snapshot = {
        let app = app.clone();
        let token = active.token.clone();
        tauri::async_runtime::spawn_blocking(move || build_parser_snapshot(&app, &face_id, &token))
            .await
            .map_err(|_| CommandError::font_parser_unavailable())?
    };

    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(ParserExportFailure::Cancelled) => {
            let _ = on_event.send(FontParserJsonEvent::Cancelled);
            return Ok(());
        }
        Err(ParserExportFailure::Failed(error)) => return Err(error),
    };

    let chunks = split_on_char_boundaries(&snapshot.json, PARSER_JSON_CHUNK_BYTES);
    on_event
        .send(FontParserJsonEvent::Started {
            face_id: request.face_id,
            parser_name: font_inspection::PARSER_NAME,
            parser_version: font_inspection::PARSER_VERSION,
            total_bytes: u32::try_from(snapshot.json.len()).unwrap_or(u32::MAX),
            chunk_count: u32::try_from(chunks.len()).unwrap_or(u32::MAX),
            truncated: snapshot.truncated,
            unicode_mappings: snapshot.unicode_mappings,
            glyphs: snapshot.glyphs,
        })
        .map_err(|_| CommandError::font_parser_unavailable())?;

    for (index, chunk) in chunks.into_iter().enumerate() {
        if active.token.is_cancelled() {
            let _ = on_event.send(FontParserJsonEvent::Cancelled);
            return Ok(());
        }
        on_event
            .send(FontParserJsonEvent::Chunk {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                text: chunk.to_owned(),
            })
            .map_err(|_| CommandError::font_parser_unavailable())?;
    }

    on_event
        .send(FontParserJsonEvent::Finished)
        .map_err(|_| CommandError::font_parser_unavailable())
}

/// Abandons a running parser export. Unknown IDs are ignored: an export that already
/// finished has nothing left to cancel.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub fn cancel_font_face_parser_export(
    export_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), CommandError> {
    ensure_trusted_window(&window)?;
    validate_export_id(&export_id)?;
    app.state::<ParserExports>().cancel(&export_id);
    Ok(())
}

fn build_parser_snapshot(
    app: &tauri::AppHandle,
    face_id: &str,
    cancel: &CancelToken,
) -> Result<ParserJsonSnapshot, ParserExportFailure> {
    let (source, face_index) = app
        .state::<CatalogueState>()
        .face_source(face_id)
        .map_err(ParserExportFailure::Failed)?;
    let bytes = catalogue::read_face_bytes(&source)
        .map_err(|error| ParserExportFailure::Failed(map_catalogue_inspection_error(&error)))?;
    font_inspection::export_face_json(bytes.as_slice(), face_index, cancel).map_err(|error| {
        match error {
            FontInspectionError::Cancelled => ParserExportFailure::Cancelled,
            other => ParserExportFailure::Failed(map_catalogue_inspection_error(
                &CatalogueInspectionError::Parser(other),
            )),
        }
    })
}

/// Splits `text` into pieces of at most `chunk_bytes`, never inside a character. Snapshot
/// text carries the characters a font maps, so a naive byte split would cut them apart.
fn split_on_char_boundaries(text: &str, chunk_bytes: usize) -> Vec<&str> {
    debug_assert!(chunk_bytes >= 4, "a chunk must fit the longest character");
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = start.saturating_add(chunk_bytes).min(text.len());
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        chunks.push(&text[start..end]);
        start = end;
    }
    chunks
}

#[tauri::command]
pub async fn inspect_font_glyph_outline(
    request: FontGlyphOutlineRequest,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<FontGlyphOutline, CommandError> {
    ensure_trusted_window(&window)?;
    validate_glyph_outline_request(&request)?;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<CatalogueState>()
            .inspect_glyph_outline(&request)
    })
    .await
    .map_err(|_| CommandError::font_parser_unavailable())?
}

/// Returns the on-disk path a scanned face came from.
///
/// Catalogue summaries deliberately carry only a sanitized file name, so this is the one
/// place a real path crosses into the web view. It is reached from an explicit "Copy file
/// path" action, never as part of a routine scan.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn font_face_file_path(
    face_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<String, CommandError> {
    ensure_trusted_window(&window)?;
    validate_face_id(&face_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<CatalogueState>()
            .face_file_path(&face_id)
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| CommandError::font_file_unavailable())?
}

/// Opens the platform file manager with the scanned face's file selected.
///
/// The path is resolved from the opaque face ID inside the backend and never leaves it.
/// Returns `true` when the file itself could be highlighted; system fonts live in a shell
/// namespace folder whose files are not selectable, so those open the folder and return
/// `false` for the caller to explain.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn reveal_font_face_file(
    face_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<bool, CommandError> {
    ensure_trusted_window(&window)?;
    validate_face_id(&face_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let path = app.state::<CatalogueState>().face_file_path(&face_id)?;
        font_platform::reveal_in_file_manager(&path)
            .map(|outcome| outcome == font_platform::RevealOutcome::Selected)
            .map_err(|error| {
                log::error!("Revealing a font file in the file manager failed: {error}");
                CommandError::font_file_reveal_failed()
            })
    })
    .await
    .map_err(|_| CommandError::font_file_reveal_failed())?
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn validate_font_file(
    path: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<ValidatedLocalFont, CommandError> {
    ensure_trusted_window(&window)?;

    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("font")
        .to_owned();
    // Clone an owned handle to the shared registry so the read + parse can run on a
    // blocking worker without borrowing app state across the await.
    let store = (*app.state::<local_fonts::PreviewStore>()).clone();

    tauri::async_runtime::spawn_blocking(move || {
        let bytes = read_capped_font(&path)?;
        local_fonts::validate_and_register(&store, bytes, &file_name)
            .map_err(|error| map_local_font_error(&error))
    })
    .await
    .map_err(|_| CommandError::local_font_unreadable())?
}

/// Registers the exact bytes of one scanned face for preview and returns its handle.
///
/// This is the preview authority for everything in the installed catalogue. The web view
/// asks by opaque face ID and never sees a path; the face is lifted out of its collection
/// so the web view cannot quietly render face zero in its place, and the bytes are
/// revalidated before they are registered. A specimen drawn through the returned family
/// name is then that face and nothing else: no family-name lookup for the OS to resolve
/// against a duplicate, and no system fallback standing in for a character the face does
/// not actually have.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn preview_font_face(
    face_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<ValidatedLocalFont, CommandError> {
    ensure_trusted_window(&window)?;
    validate_face_id(&face_id)?;
    // Clone an owned handle to the shared registry so the read, extract, and parse can run
    // on a blocking worker without borrowing app state across the await.
    let store = (*app.state::<local_fonts::PreviewStore>()).clone();

    tauri::async_runtime::spawn_blocking(move || {
        let (source, face_index) = app.state::<CatalogueState>().face_source(&face_id)?;
        // Display only. The frontend already shows this name beside the face; it is never
        // handed back as something to open.
        let file_name = match &source {
            Source::File(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("font")
                .to_owned(),
            _ => "font".to_owned(),
        };
        let bytes = catalogue::read_face_bytes(&source)
            .map_err(|error| map_catalogue_inspection_error(&error))?;
        let face = local_fonts::extract_face_sfnt(bytes.as_slice(), face_index)
            .map_err(|error| map_local_font_error(&error))?;
        drop(bytes);
        local_fonts::validate_and_register(&store, face, &file_name)
            .map_err(|error| map_local_font_error(&error))
    })
    .await
    .map_err(|_| CommandError::local_font_unreadable())?
}

#[tauri::command]
pub async fn list_google_fonts(
    request: GoogleFontPageRequest,
    app: tauri::AppHandle,
) -> Result<GoogleFontPage, CommandError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::managed_storage_unavailable())?;
    tauri::async_runtime::spawn_blocking(move || google_fonts::list_fonts(&request, &app_data_dir))
        .await
        .map_err(|_| CommandError::online_catalogue_unavailable())?
        .map_err(map_google_fonts_error)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn get_google_font_details(
    family_id: String,
    app: tauri::AppHandle,
) -> Result<GoogleFontFamilyDetails, CommandError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::managed_storage_unavailable())?;
    tauri::async_runtime::spawn_blocking(move || {
        google_fonts::font_details(&family_id, &app_data_dir)
    })
    .await
    .map_err(|_| CommandError::online_catalogue_unavailable())?
    .map_err(map_google_fonts_error)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn prepare_google_font_preview(
    artifact_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<GoogleFontPreview, CommandError> {
    ensure_trusted_window(&window)?;
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|_| CommandError::font_download_failed())?;
    let store = (*app.state::<local_fonts::PreviewStore>()).clone();
    google_fonts::prepare_preview(&artifact_id, &cache_dir, &store)
        .await
        .map_err(map_google_fonts_error)
}

/// Reads the settings this application owns.
///
/// Never fails: unreadable settings are recovered from rather than reported as an error, because
/// an interface that cannot start because of a stored preference is worse than one that starts
/// with the defaults and says so.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub fn load_preferences(app: tauri::AppHandle) -> LoadedPreferences {
    let Ok(app_data_dir) = app.path().app_data_dir() else {
        return LoadedPreferences {
            preferences: Preferences::default(),
            recovery: None,
        };
    };
    preferences::load(&app_data_dir)
}

/// Writes the settings this application owns.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub fn save_preferences(
    preferences: Preferences,
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::preferences_unavailable())?;
    preferences::save(&app_data_dir, &preferences).map_err(|error| {
        log::warn!("FontNest could not write its settings: {error}");
        CommandError::preferences_unavailable()
    })
}

/// Reviews font files the person chose, without changing anything.
///
/// Reviewing is deliberately separate from importing. Installing a font changes the computer, and
/// nobody should have to find out what a selection contained by letting it happen: this answers
/// the question first, and every verdict in it is derived from the files themselves.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn preflight_font_import(
    paths: Vec<String>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<ImportPlan, CommandError> {
    ensure_trusted_window(&window)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::managed_storage_unavailable())?;

    tauri::async_runtime::spawn_blocking(move || {
        let already_installed = ManagedInstallationRepository::in_app_data_dir(&app_data_dir)
            .installed_source_hashes(local_import::LOCAL_PROVIDER)
            .unwrap_or_default();
        let chosen = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
        local_import::review(&chosen, &already_installed)
    })
    .await
    .map_err(|_| CommandError::import_failed())
}

/// Imports the font files the person chose.
///
/// Everything the review decided is decided again here from the files themselves, because a file
/// can change between being looked at and being imported. Each file stands on its own: the result
/// says what happened to every one of them.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn import_font_files(
    paths: Vec<String>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Vec<ImportOutcome>, CommandError> {
    ensure_trusted_window(&window)?;
    // Fail closed: a ledger FontNest cannot read or trust means it cannot prove what it owns, so
    // it must not put anything new on the computer.
    app.state::<ManagedStorage>()
        .ensure_writable()
        .map_err(CommandError::managed_storage_recovery)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::managed_storage_unavailable())?;

    tauri::async_runtime::spawn_blocking(move || {
        let repository = ManagedInstallationRepository::in_app_data_dir(&app_data_dir);
        let chosen = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
        local_import::import(&chosen, &repository)
    })
    .await
    .map_err(|_| CommandError::import_failed())?
    .map_err(|error| {
        log::warn!("FontNest could not import the chosen fonts: {error}");
        CommandError::import_failed()
    })
}

/// Reports whether managed font operations are available in this session, so the interface can
/// explain a read-only recovery mode instead of offering actions that will be refused.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub fn managed_storage_status(app: tauri::AppHandle) -> ManagedStorageStatus {
    app.state::<ManagedStorage>().status()
}

#[tauri::command]
pub async fn install_google_font(
    request: InstallGoogleFontRequest,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<GoogleFontInstallResult, CommandError> {
    ensure_trusted_window(&window)?;
    // Fail closed: a ledger FontNest cannot read or trust means it cannot prove what it owns, so
    // it must not register anything new with the operating system.
    app.state::<ManagedStorage>()
        .ensure_writable()
        .map_err(CommandError::managed_storage_recovery)?;
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|_| CommandError::font_download_failed())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::managed_storage_unavailable())?;
    google_fonts::install_fonts(&request, &cache_dir, &app_data_dir)
        .await
        .map_err(map_google_fonts_error)
}

/// Takes back fonts `FontNest` installed from the online catalogue.
///
/// Every font is proven against the computer before anything is removed, and a font that cannot be
/// proven is reported in the result rather than removed anyway, so the interface can say which
/// ones were left alone and why.
#[tauri::command]
pub async fn uninstall_google_font(
    request: UninstallGoogleFontRequest,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<GoogleFontUninstallResult, CommandError> {
    ensure_trusted_window(&window)?;
    // Fail closed: a ledger FontNest cannot read or trust cannot prove what it owns, so it must
    // not take anything off this computer.
    app.state::<ManagedStorage>()
        .ensure_writable()
        .map_err(CommandError::managed_storage_recovery)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| CommandError::managed_storage_unavailable())?;
    tauri::async_runtime::spawn_blocking(move || {
        google_fonts::uninstall_fonts(&request, &app_data_dir)
    })
    .await
    .map_err(|_| CommandError::font_uninstall_failed())?
    .map_err(map_google_fonts_error)
}

#[tauri::command]
pub async fn fetch_remote_changelog(window: tauri::WebviewWindow) -> Result<String, CommandError> {
    ensure_trusted_window(&window)?;
    release_notes::fetch_changelog()
        .await
        .map_err(|error| map_release_notes_error(&error))
}

#[tauri::command]
pub async fn check_for_app_update(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<AppUpdateInfo>, CommandError> {
    ensure_trusted_window(&window)?;
    let updater = app.updater().map_err(|error| {
        log::error!("Application updater could not be initialized: {error}");
        CommandError::update_check_failed()
    })?;
    let update = updater.check().await.map_err(|error| {
        log::error!("Application update check failed: {error}");
        CommandError::update_check_failed()
    })?;

    Ok(update.map(|update| AppUpdateInfo {
        current_version: update.current_version,
        version: update.version,
        notes: update.body.unwrap_or_default(),
        published_at: update.date.map(|date| date.to_string()),
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn install_app_update(
    expected_version: String,
    on_event: Channel<AppUpdateEvent>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), CommandError> {
    ensure_trusted_window(&window)?;
    let updater = app.updater().map_err(|error| {
        log::error!("Application updater could not be initialized: {error}");
        CommandError::update_install_failed()
    })?;
    let update = updater
        .check()
        .await
        .map_err(|error| {
            log::error!("Application update re-check failed: {error}");
            CommandError::update_install_failed()
        })?
        .ok_or_else(CommandError::update_unavailable)?;

    if !update_version_matches(&expected_version, &update.version) {
        return Err(CommandError::update_changed());
    }

    let progress_events = on_event.clone();
    let installing_events = on_event;
    let mut downloaded = 0_u64;
    let mut started = false;

    update
        .download_and_install(
            move |chunk_length, content_length| {
                if !started {
                    started = true;
                    let _ = progress_events.send(AppUpdateEvent::DownloadStarted {
                        total: content_length.map(saturating_u32),
                    });
                }
                downloaded =
                    downloaded.saturating_add(u64::try_from(chunk_length).unwrap_or(u64::MAX));
                let _ = progress_events.send(AppUpdateEvent::DownloadProgress {
                    downloaded: saturating_u32(downloaded),
                    total: content_length.map(saturating_u32),
                });
            },
            move || {
                let _ = installing_events.send(AppUpdateEvent::Installing);
            },
        )
        .await
        .map_err(|error| {
            log::error!("Application update installation failed: {error}");
            CommandError::update_install_failed()
        })
}

fn update_version_matches(expected: &str, announced: &str) -> bool {
    !expected.trim().is_empty() && expected == announced
}

fn saturating_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn ensure_trusted_window(window: &tauri::WebviewWindow) -> Result<(), CommandError> {
    let url = window.url().map_err(|_| CommandError::untrusted_origin())?;
    if is_trusted_app_origin(&url) {
        Ok(())
    } else {
        Err(CommandError::untrusted_origin())
    }
}

/// True when an `Origin` header names the app's own web view. The internal preview
/// protocol grants cross-origin read access to that origin and nothing else.
pub(crate) fn is_trusted_origin_header(value: &str) -> bool {
    tauri::Url::parse(value).is_ok_and(|url| is_trusted_app_origin(&url))
}

pub(crate) fn is_trusted_app_origin(url: &tauri::Url) -> bool {
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or_default();
    if scheme == "tauri" && host == "localhost" {
        return true;
    }
    if matches!(scheme, "http" | "https") && host == "tauri.localhost" {
        return true;
    }
    cfg!(debug_assertions) && scheme == "http" && host == "localhost" && url.port() == Some(5173)
}

fn validate_face_id(face_id: &str) -> Result<(), CommandError> {
    if is_well_formed(IdentityKind::Face, face_id) {
        Ok(())
    } else {
        Err(CommandError::font_face_unavailable())
    }
}

/// Export IDs come from the frontend, so they are kept to a short opaque shape: they are
/// map keys and nothing else, and never reach the filesystem or a query.
fn validate_export_id(export_id: &str) -> Result<(), CommandError> {
    if export_id.is_empty()
        || export_id.len() > MAX_EXPORT_ID_LENGTH
        || !export_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(CommandError::invalid_parser_export_request());
    }
    Ok(())
}

fn validate_glyph_outline_request(request: &FontGlyphOutlineRequest) -> Result<(), CommandError> {
    validate_face_id(&request.face_id)?;
    if char::from_u32(request.codepoint).is_none()
        || request.variations.len() > MAX_GLYPH_VARIATIONS
        || request.variations.iter().any(|variation| {
            variation.tag.len() != 4
                || !variation
                    .tag
                    .bytes()
                    .all(|byte| byte.is_ascii_graphic() || byte == b' ')
                || !variation.value.is_finite()
        })
    {
        return Err(CommandError::invalid_glyph_request());
    }
    Ok(())
}

fn map_catalogue_inspection_error(error: &CatalogueInspectionError) -> CommandError {
    log::error!("Font face inspection failed: {error}");
    match error {
        CatalogueInspectionError::UnknownFace | CatalogueInspectionError::DataUnavailable => {
            CommandError::font_face_unavailable()
        }
        CatalogueInspectionError::NotFileBacked => CommandError::font_file_unavailable(),
        CatalogueInspectionError::Parser(
            FontInspectionError::InvalidCodepoint | FontInspectionError::MissingGlyph,
        ) => CommandError::font_glyph_unavailable(),
        CatalogueInspectionError::Parser(_) => CommandError::font_parser_unavailable(),
    }
}

fn read_capped_font(path: &str) -> Result<Vec<u8>, CommandError> {
    let metadata = std::fs::metadata(path).map_err(|_| CommandError::local_font_unreadable())?;
    if !metadata.is_file() {
        return Err(CommandError::local_font_unreadable());
    }
    if metadata.len() > local_fonts::MAX_LOCAL_FONT_BYTES {
        return Err(CommandError::local_font_too_large());
    }
    std::fs::read(path).map_err(|_| CommandError::local_font_unreadable())
}

fn map_local_font_error(error: &LocalFontError) -> CommandError {
    log::error!("Local font validation failed: {error}");
    match error {
        LocalFontError::TooLarge => CommandError::local_font_too_large(),
        LocalFontError::InvalidFont
        | LocalFontError::InvalidMetadata
        | LocalFontError::TooManyFaces => CommandError::local_font_invalid(),
    }
}

fn map_release_notes_error(error: &ReleaseNotesError) -> CommandError {
    log::error!("Release notes fetch failed: {error}");
    CommandError::release_notes_unavailable()
}

fn map_google_fonts_error(error: GoogleFontsError) -> CommandError {
    log::error!("Google Fonts operation failed: {error}");
    match error {
        GoogleFontsError::Manifest => CommandError::online_catalogue_unavailable(),
        GoogleFontsError::InvalidRequest => CommandError::invalid_google_font_request(),
        GoogleFontsError::Download => CommandError::font_download_failed(),
        GoogleFontsError::Integrity | GoogleFontsError::FontValidation => {
            CommandError::font_validation_failed()
        }
        GoogleFontsError::Database => CommandError::managed_storage_unavailable(),
        GoogleFontsError::Platform => CommandError::font_install_failed(),
        #[cfg(not(windows))]
        GoogleFontsError::UnsupportedPlatform => CommandError::font_platform_unsupported(),
    }
}

#[cfg(test)]
mod tests {
    use crate::dto::{FontGlyphOutlineRequest, FontGlyphVariationValue};

    use super::{
        MAX_ACTIVE_PARSER_EXPORTS, ParserExports, is_trusted_app_origin, is_trusted_origin_header,
        split_on_char_boundaries, update_version_matches, validate_export_id, validate_face_id,
        validate_glyph_outline_request,
    };

    #[test]
    fn updater_installation_requires_the_version_that_was_presented() {
        assert!(update_version_matches("0.1.1", "0.1.1"));
        assert!(!update_version_matches("0.1.1", "0.1.2"));
        assert!(!update_version_matches("", "0.1.1"));
    }

    #[test]
    fn sensitive_font_commands_only_trust_the_app_origin() {
        assert!(is_trusted_app_origin(
            &tauri::Url::parse("http://tauri.localhost/").expect("the production URL")
        ));
        assert!(!is_trusted_app_origin(
            &tauri::Url::parse("https://fonts.google.com/").expect("the remote URL")
        ));
    }

    #[test]
    fn preview_protocol_only_grants_cors_to_the_app_origin() {
        assert!(is_trusted_origin_header("http://tauri.localhost"));
        assert!(!is_trusted_origin_header("https://fonts.google.com"));
        assert!(!is_trusted_origin_header("null"));
        assert!(!is_trusted_origin_header("not a url"));
    }

    #[test]
    fn parser_commands_only_accept_opaque_face_ids() {
        assert!(validate_face_id("face:0123456789abcdef0123456789abcdef").is_ok());
        assert!(validate_face_id("C:\\Windows\\Fonts\\arial.ttf").is_err());
        assert!(validate_face_id("face:not-a-digest").is_err());
        assert!(validate_face_id("family:0123456789abcdef0123456789abcdef").is_err());
    }

    #[test]
    fn parser_exports_only_accept_short_opaque_ids() {
        assert!(validate_export_id("a1b2c3-d4e5").is_ok());
        assert!(validate_export_id("").is_err());
        assert!(validate_export_id("../../secret").is_err());
        assert!(validate_export_id(&"a".repeat(65)).is_err());
    }

    #[test]
    fn snapshot_chunks_never_split_a_character() {
        // Snapshot text carries the characters a font maps, so a byte split would tear
        // multi-byte characters in half and the reassembled document would not parse.
        let text = "€".repeat(16);
        let chunks = split_on_char_boundaries(&text, 8);

        assert!(chunks.len() > 1, "the text must actually be split");
        assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
        assert!(chunks.iter().all(|chunk| chunk.len() <= 8));
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn an_empty_snapshot_produces_no_chunks() {
        assert!(split_on_char_boundaries("", 1024).is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_real_snapshot_survives_the_round_trip_through_chunks() {
        use crate::font_inspection::{CancelToken, export_face_json};

        let data = std::fs::read(r"C:\Windows\Fonts\arial.ttf")
            .expect("Arial is part of the supported Windows font set");
        let snapshot =
            export_face_json(&data, 0, &CancelToken::default()).expect("a bounded snapshot");

        // Small chunks on purpose: the frontend reassembles whatever the stream sends, so
        // the seam has to hold at any chunk size, not just the production one.
        let chunks = split_on_char_boundaries(&snapshot.json, 997);
        let rebuilt = chunks.concat();

        assert!(chunks.len() > 1, "the snapshot must actually be split");
        assert_eq!(rebuilt, snapshot.json);
        serde_json::from_str::<serde_json::Value>(&rebuilt)
            .expect("the reassembled document parses");
    }

    #[test]
    fn a_registered_export_can_be_cancelled_until_it_is_finished() {
        let exports = ParserExports::default();
        let token = exports.begin("export-1").expect("a free slot");

        assert!(!token.is_cancelled());
        exports.cancel("export-1");
        assert!(token.is_cancelled());

        exports.finish("export-1");
        // Cancelling a finished export is a no-op rather than an error.
        exports.cancel("export-1");
        assert!(exports.begin("export-1").is_ok(), "the slot is free again");
    }

    #[test]
    fn parser_exports_are_bounded_and_never_reuse_a_live_id() {
        let exports = ParserExports::default();
        for index in 0..MAX_ACTIVE_PARSER_EXPORTS {
            exports
                .begin(&format!("export-{index}"))
                .expect("slots up to the limit");
        }

        assert!(
            exports.begin("export-overflow").is_err(),
            "the limit holds the number of concurrent parses down"
        );
        assert!(
            exports.begin("export-0").is_err(),
            "a live ID cannot be claimed twice"
        );
    }

    #[test]
    fn glyph_outline_requests_validate_codepoints_and_variations() {
        let valid = FontGlyphOutlineRequest {
            face_id: "face:0123456789abcdef0123456789abcdef".to_owned(),
            codepoint: u32::from('A'),
            variations: vec![FontGlyphVariationValue {
                tag: "wght".to_owned(),
                value: 650.0,
            }],
        };
        assert!(validate_glyph_outline_request(&valid).is_ok());

        let mut invalid = valid.clone();
        invalid.codepoint = 0x11_0000;
        assert!(validate_glyph_outline_request(&invalid).is_err());

        invalid = valid;
        invalid.variations[0].value = f32::NAN;
        assert!(validate_glyph_outline_request(&invalid).is_err());
    }
}
