mod auth;
mod commands;
mod config;
mod error;
mod queue;
mod youtube;

use std::sync::Arc;

use tauri::Manager;

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

            app.manage(http.clone());
            app.manage(auth.clone());
            app.manage(q.clone());

            tauri::async_runtime::spawn(queue::worker::run(handle, q, auth, http));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_status,
            commands::import_client_json,
            commands::sign_in,
            commands::sign_out,
            commands::get_channel,
            commands::open_in_studio,
            commands::list_jobs,
            commands::add_files,
            commands::update_metadata,
            commands::set_thumbnail,
            commands::start,
            commands::start_all,
            commands::pause,
            commands::resume,
            commands::cancel,
            commands::retry,
            commands::remove,
            commands::clear_done,
            commands::reorder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
