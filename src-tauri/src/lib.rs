mod auth;
mod commands;
mod config;
mod error;
mod queue;
mod settings;
mod youtube;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            // One shared client means one connection pool. reqwest::Client is an Arc internally, so clones are cheap.
            let http = youtube::http_client()?;
            let auth = Arc::new(auth::Auth::new(&handle, http.clone())?);
            let q = Arc::new(queue::Queue::load(handle.clone())?);
            let settings_store = Arc::new(settings::SettingsStore::load(&handle)?);
            let initial_settings = settings_store.get();
            app.manage(settings_store);

            app.manage(http.clone());
            app.manage(auth.clone());
            app.manage(q.clone());

            let show_i = MenuItem::with_id(app, "show", "Show Emissa", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Emissa", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &sep, &quit_i])?;

            let mut builder = TrayIconBuilder::with_id("main_tray")
                .tooltip("Emissa")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            show_main_window(app);
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                        | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        } => {
                            show_main_window(tray.app_handle());
                        }
                        _ => {}
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }

            let tray = builder.build(app)?;
            let _ = tray.set_visible(initial_settings.allow_system_tray);

            tauri::async_runtime::spawn(queue::worker::run(handle, q, auth, http));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if let Some(store) = app.try_state::<Arc<settings::SettingsStore>>() {
                    if store.get().allow_system_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_status,
            commands::import_client_json,
            commands::sign_in,
            commands::sign_out,
            commands::get_channel,
            commands::open_in_studio,
            commands::open_video,
            commands::list_jobs,
            commands::add_files,
            commands::update_metadata,
            commands::set_thumbnail,
            commands::set_captions,
            commands::caption_access,
            commands::start,
            commands::start_all,
            commands::pause,
            commands::resume,
            commands::cancel,
            commands::retry,
            commands::remove,
            commands::clear_done,
            commands::reorder,
            commands::get_settings,
            commands::set_settings,
            commands::open_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
