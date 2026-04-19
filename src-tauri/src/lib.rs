mod shared;
mod tools;
mod db;
mod roll20;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
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
                let opts = SqliteConnectOptions::from_str(&db_url)
                    .expect("Invalid db_url")
                    .foreign_keys(true);
                let pool = SqlitePool::connect_with(opts).await
                    .expect("Failed to connect to database");
                sqlx::migrate!("./migrations").run(&pool).await
                    .expect("Failed to run migrations");
                db::seed::seed_dyscrasias(&pool).await
                    .expect("Failed to seed dyscrasias");
                handle.manage(DbState(Arc::new(pool)));

                // Roll20 WebSocket integration
                let roll20_state = Arc::new(roll20::Roll20State::new());
                let roll20_state_for_ws = Arc::clone(&roll20_state);
                let handle_for_ws = handle.clone();
                handle.manage(roll20::Roll20Conn(roll20_state));
                tauri::async_runtime::spawn(
                    roll20::start_ws_server(roll20_state_for_ws, handle_for_ws)
                );
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
            tools::export::export_result_to_md,
            roll20::commands::get_roll20_characters,
            roll20::commands::get_roll20_status,
            roll20::commands::refresh_roll20_data,
            roll20::commands::send_roll20_chat,
            roll20::commands::set_roll20_attribute,
            db::chronicle::list_chronicles,
            db::chronicle::get_chronicle,
            db::chronicle::create_chronicle,
            db::chronicle::update_chronicle,
            db::chronicle::delete_chronicle,
            db::node::list_nodes,
            db::node::get_node,
            db::node::create_node,
            db::node::update_node,
            db::node::delete_node,
            db::node::get_parent_of,
            db::node::get_children_of,
            db::node::get_siblings_of,
            db::node::get_path_to_root,
            db::node::get_subtree,
            db::edge::list_edges,
            db::edge::list_edges_for_node,
            db::edge::create_edge,
            db::edge::update_edge,
            db::edge::delete_edge,
            db::advantage::list_advantages,
            db::advantage::add_advantage,
            db::advantage::update_advantage,
            db::advantage::delete_advantage,
            db::advantage::roll_random_advantage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
