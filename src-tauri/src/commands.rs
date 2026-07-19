use std::sync::Mutex;

use tauri::{Manager, ipc::Channel};
use tauri_plugin_updater::UpdaterExt;

use crate::catalogue::{self, CatalogueInspectionError, FontCatalogueStore};
use crate::dto::{
    AppUpdateEvent, AppUpdateInfo, CommandError, FontCatalogue, FontFaceInspection,
    FontGlyphOutline, FontGlyphOutlineRequest, FontParserJsonExport, GoogleFontFamilyDetails,
    GoogleFontInstallResult, GoogleFontPage, GoogleFontPageRequest, GoogleFontPreview, Greeting,
    InstallGoogleFontRequest, ValidatedLocalFont,
};
use crate::font_inspection::FontInspectionError;
use crate::google_fonts::{self, GoogleFontsError};
use crate::local_fonts::{self, LocalFontError};
use crate::release_notes::{self, ReleaseNotesError};

const MAX_NAME_LENGTH: usize = 64;
const FACE_ID_PREFIX: &str = "face:";
const SHA1_HEX_LENGTH: usize = 40;
const MAX_GLYPH_VARIATIONS: usize = 64;

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

    fn export_face_json(&self, face_id: &str) -> Result<FontParserJsonExport, CommandError> {
        let current = self
            .store
            .lock()
            .map_err(|_| CommandError::catalogue_unavailable())?;
        let store = current
            .as_ref()
            .ok_or_else(CommandError::catalogue_unavailable)?;
        store
            .export_face_json(face_id)
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
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub fn greet(name: String) -> Result<Greeting, CommandError> {
    let name = name.trim();

    if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
        return Err(CommandError::invalid_name());
    }

    Ok(Greeting::new(name))
}

#[tauri::command]
pub async fn scan_installed_fonts(app: tauri::AppHandle) -> Result<FontCatalogue, CommandError> {
    let scanned = tauri::async_runtime::spawn_blocking(catalogue::scan_installed_fonts)
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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub async fn export_font_face_parser_json(
    face_id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<FontParserJsonExport, CommandError> {
    ensure_trusted_window(&window)?;
    validate_face_id(&face_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<CatalogueState>().export_face_json(&face_id)
    })
    .await
    .map_err(|_| CommandError::font_parser_unavailable())?
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
    google_fonts::prepare_preview(&artifact_id, &cache_dir)
        .await
        .map_err(map_google_fonts_error)
}

#[tauri::command]
pub async fn install_google_font(
    request: InstallGoogleFontRequest,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<GoogleFontInstallResult, CommandError> {
    ensure_trusted_window(&window)?;
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
    let Some(digest) = face_id.strip_prefix(FACE_ID_PREFIX) else {
        return Err(CommandError::font_face_unavailable());
    };
    if digest.len() != SHA1_HEX_LENGTH || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CommandError::font_face_unavailable());
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
        greet, is_trusted_app_origin, is_trusted_origin_header, update_version_matches,
        validate_face_id, validate_glyph_outline_request,
    };

    #[test]
    fn greeting_uses_a_trimmed_name() {
        let greeting = greet("  Akari  ".to_owned()).expect("a valid greeting");

        assert_eq!(greeting.message, "Welcome to FontNest, Akari.");
    }

    #[test]
    fn greeting_rejects_an_empty_name() {
        let error = greet("   ".to_owned()).expect_err("an empty name must be rejected");

        assert_eq!(error.code, "invalid_name");
    }

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
        assert!(validate_face_id("face:0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(validate_face_id("C:\\Windows\\Fonts\\arial.ttf").is_err());
        assert!(validate_face_id("face:not-a-digest").is_err());
    }

    #[test]
    fn glyph_outline_requests_validate_codepoints_and_variations() {
        let valid = FontGlyphOutlineRequest {
            face_id: "face:0123456789abcdef0123456789abcdef01234567".to_owned(),
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
