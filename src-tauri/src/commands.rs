use tauri::Manager;

use crate::catalogue;
use crate::dto::{
    CommandError, FontCatalogue, GoogleFontFamilyDetails, GoogleFontInstallResult, GoogleFontPage,
    GoogleFontPageRequest, GoogleFontPreview, Greeting, InstallGoogleFontRequest,
};
use crate::google_fonts::{self, GoogleFontsError};

const MAX_NAME_LENGTH: usize = 64;

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
pub async fn scan_installed_fonts() -> Result<FontCatalogue, CommandError> {
    tauri::async_runtime::spawn_blocking(catalogue::scan_installed_fonts)
        .await
        .map_err(|_| CommandError::catalogue_unavailable())
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

fn ensure_trusted_window(window: &tauri::WebviewWindow) -> Result<(), CommandError> {
    let url = window.url().map_err(|_| CommandError::untrusted_origin())?;
    if is_trusted_app_origin(&url) {
        Ok(())
    } else {
        Err(CommandError::untrusted_origin())
    }
}

fn is_trusted_app_origin(url: &tauri::Url) -> bool {
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
    use super::{greet, is_trusted_app_origin};

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
    fn sensitive_font_commands_only_trust_the_app_origin() {
        assert!(is_trusted_app_origin(
            &tauri::Url::parse("http://tauri.localhost/").expect("the production URL")
        ));
        assert!(!is_trusted_app_origin(
            &tauri::Url::parse("https://fonts.google.com/").expect("the remote URL")
        ));
    }
}
