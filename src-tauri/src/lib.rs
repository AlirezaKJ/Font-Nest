mod catalogue;
mod commands;
mod dto;
mod font_inspection;
mod font_platform;
mod google_fonts;
mod local_fonts;
mod managed_installations;
mod release_notes;

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
        .manage(local_fonts::PreviewStore::default())
        // Serves validated local-font bytes to the WebView by opaque handle only.
        // The registry never exposes a filesystem path, and an unknown or malformed
        // handle yields 404, so nothing but already-validated fonts can be loaded.
        .register_uri_scheme_protocol("fontnest-preview", |ctx, request| {
            use tauri::Manager;

            let handle = request.uri().path().trim_start_matches('/');
            let store = ctx.app_handle().state::<local_fonts::PreviewStore>();
            let (status, body) = match store.get(handle) {
                Some(bytes) => (tauri::http::StatusCode::OK, bytes.to_vec()),
                None => (tauri::http::StatusCode::NOT_FOUND, Vec::new()),
            };

            // The web view fetches fonts in CORS mode, so without an explicit grant the
            // bytes are blocked. Echo the requesting origin only when it is the app's own
            // web view; any other origin gets no grant and the fetch stays blocked.
            let allowed_origin = request
                .headers()
                .get(tauri::http::header::ORIGIN)
                .and_then(|origin| origin.to_str().ok())
                .filter(|origin| commands::is_trusted_origin_header(origin))
                .map(std::borrow::ToOwned::to_owned);

            let mut response = tauri::http::Response::builder()
                .status(status)
                .header(tauri::http::header::CONTENT_TYPE, "font/ttf")
                .header(tauri::http::header::CACHE_CONTROL, "no-store")
                .header(tauri::http::header::VARY, "Origin");
            if let Some(origin) = allowed_origin {
                response =
                    response.header(tauri::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
            }
            response.body(body).unwrap_or_else(|_| {
                tauri::http::Response::builder()
                    .status(tauri::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Vec::new())
                    .expect("an empty error response always builds")
            })
        })
        .setup(|app| {
            use tauri::Manager;

            let app_data_dir = app.path().app_data_dir()?;
            if let Err(error) = google_fonts::initialize_storage(&app_data_dir) {
                log::error!("Managed font storage could not be initialized: {error}");
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
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
            commands::greet,
            commands::scan_installed_fonts,
            commands::inspect_font_face,
            commands::inspect_font_glyph_outline,
            commands::export_font_face_parser_json,
            commands::font_face_file_path,
            commands::reveal_font_face_file,
            commands::validate_font_file,
            commands::list_google_fonts,
            commands::get_google_font_details,
            commands::prepare_google_font_preview,
            commands::install_google_font,
            commands::fetch_remote_changelog,
            commands::check_for_app_update,
            commands::install_app_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
