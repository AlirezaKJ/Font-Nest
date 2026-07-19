mod catalogue;
mod commands;
mod dto;
mod font_inspection;
mod font_platform;
mod google_fonts;
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
        .manage(commands::CatalogueState::default())
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
