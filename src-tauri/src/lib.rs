mod adapters;
mod commands;
mod db;
mod models;
mod services;

use commands::{
    integrations::detect_integrations, integrations::get_available_integrations,
    integrations::list_integrations, projects::create_project, projects::delete_project,
    projects::get_project, projects::get_projects, projects::launch_project, projects::list_projects,
    projects::update_project,
    time_logs::create_time_log, time_logs::get_time_logs,
};
use db::init_database;
use sqlx::SqlitePool;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray, Builder,
};

struct AppState {
    db_pool: Option<SqlitePool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let pool = init_database().await.expect("failed to init database");
    let app_state = AppState {
        db_pool: Some(pool),
    };

    Builder::default()
        .manage(app_state)
        .setup(|app| {
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let tray_menu = MenuBuilder::new(app).items(&[&quit_item]).build()?;

            tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            create_project,
            get_projects,
            list_projects,
            get_project,
            update_project,
            delete_project,
            launch_project,
            detect_integrations,
            list_integrations,
            get_available_integrations,
            get_time_logs,
            create_time_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
