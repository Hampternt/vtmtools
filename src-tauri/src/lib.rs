mod shared;
mod tools;
mod db;

use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Manager;

pub struct DbState(pub Arc<SqlitePool>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("vtmtools.db");
            let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = SqlitePool::connect(&db_url).await
                    .expect("Failed to connect to database");
                sqlx::migrate!("./migrations").run(&pool).await
                    .expect("Failed to run migrations");
                db::seed::seed_dyscrasias(&pool).await
                    .expect("Failed to seed dyscrasias");
                handle.manage(DbState(Arc::new(pool)));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tools::resonance::roll_resonance,
            db::dyscrasia::list_dyscrasias,
            db::dyscrasia::add_dyscrasia,
            db::dyscrasia::update_dyscrasia,
            db::dyscrasia::delete_dyscrasia,
            db::dyscrasia::roll_random_dyscrasia,
            // tools::export::export_result_to_md is added in Task 11
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
