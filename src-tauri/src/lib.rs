mod catalogue;
mod commands;
mod dto;
mod font_identity;
mod font_inspection;
mod font_origin;
mod font_platform;
mod font_variations;
mod google_fonts;
mod local_fonts;
mod local_import;
mod managed_installations;
mod managed_ownership;
mod managed_recovery;
mod managed_storage;
mod managed_uninstall;
mod preferences;
mod release_notes;
mod window_state;

/// Answers one request to the internal preview protocol.
///
/// The registry is keyed by opaque handle and never holds a filesystem path, so an unknown or
/// malformed handle is a 404 and nothing but already-validated font bytes can be served.
#[allow(clippy::needless_pass_by_value)] // Tauri hands a protocol handler owned values.
fn serve_preview_bytes<R: tauri::Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    use tauri::Manager;

    let handle = request.uri().path().trim_start_matches('/');
    let store = ctx.app_handle().state::<local_fonts::PreviewStore>();
    let (status, body) = match store.get(handle) {
        Some(bytes) => (tauri::http::StatusCode::OK, bytes.to_vec()),
        None => (tauri::http::StatusCode::NOT_FOUND, Vec::new()),
    };

    // The web view fetches fonts in CORS mode, so without an explicit grant the bytes are
    // blocked. Echo the requesting origin only when it is the app's own web view; any other
    // origin gets no grant and the fetch stays blocked.
    let allowed_origin = request
        .headers()
        .get(tauri::http::header::ORIGIN)
        .and_then(|origin| origin.to_str().ok())
        .filter(|origin| {
            commands::is_trusted_origin_header(
                origin,
                commands::development_origin_for(ctx.app_handle()),
            )
        })
        .map(std::borrow::ToOwned::to_owned);

    let mut response = tauri::http::Response::builder()
        .status(status)
        .header(tauri::http::header::CONTENT_TYPE, "font/ttf")
        .header(tauri::http::header::CACHE_CONTROL, "no-store")
        .header(tauri::http::header::VARY, "Origin");
    if let Some(origin) = allowed_origin {
        response = response.header(tauri::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    response.body(body).unwrap_or_else(|_| {
        tauri::http::Response::builder()
            .status(tauri::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .expect("an empty error response always builds")
    })
}

/// Starts the `FontNest` desktop application.
///
/// # Panics
///
/// Panics when Tauri cannot initialize or run the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::CatalogueState::default())
        .manage(commands::ParserExports::default())
        .manage(local_fonts::PreviewStore::default())
        .manage(window_state::Tracker::default())
        // Serves validated local-font bytes to the WebView by opaque handle only.
        // The registry never exposes a filesystem path, and an unknown or malformed
        // handle yields 404, so nothing but already-validated fonts can be loaded.
        .register_uri_scheme_protocol("fontnest-preview", serve_preview_bytes)
        .on_window_event(|window, event| {
            window_state::handle_window_event(window, event);
        })
        .setup(|app| {
            use tauri::Manager;

            // Managed font state is claimed once, at startup: this process takes the writer lock
            // and migrates the ledger, or the whole session stays read-only. Every command that
            // would mutate managed fonts checks this state before it does anything.
            let app_data_dir = app.path().app_data_dir()?;
            let managed_storage = managed_storage::ManagedStorage::initialize(&app_data_dir);
            if let Some(reason) = managed_storage.recovery_reason() {
                log::error!(
                    "FontNest started in read-only recovery mode: {reason:?}. Installing, updating, and removing fonts is disabled."
                );
            }
            app.manage(managed_storage);

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Put the window back where it was left, before it is shown, so restoring it is
            // not a visible jump. A rectangle that no longer lands on a monitor keeps its size
            // and gives up its position; see `window_state`.
            if let Some(window) = app.get_webview_window("main") {
                window_state::restore(&window, &app_data_dir);
            }

            // The main window launches hidden so the WebView's blank white background is
            // never shown while the frontend loads; the frontend reveals it after the first
            // themed frame paints. This is a safety net: if that reveal never runs (for
            // example a startup script error), show the window anyway so the app can never
            // be left running with no visible window.
            if let Some(window) = app.get_webview_window("main") {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    if matches!(window.is_visible(), Ok(false)) {
                        if let Err(error) = window.show() {
                            log::error!("FontNest could not reveal the main window: {error}");
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_installed_fonts,
            commands::inspect_font_face,
            commands::inspect_font_glyph_outline,
            commands::export_font_face_parser_json,
            commands::cancel_font_face_parser_export,
            commands::font_face_file_path,
            commands::reveal_font_face_file,
            commands::validate_font_file,
            commands::preview_font_face,
            commands::list_google_fonts,
            commands::get_google_font_details,
            commands::prepare_google_font_preview,
            commands::install_google_font,
            commands::uninstall_google_font,
            commands::managed_storage_status,
            commands::managed_font_inventory,
            commands::remove_managed_font,
            commands::restore_managed_font,
            commands::preflight_font_import,
            commands::import_font_files,
            commands::load_preferences,
            commands::save_preferences,
            commands::fetch_remote_changelog,
            commands::check_for_app_update,
            commands::install_app_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
