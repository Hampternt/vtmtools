mod shared;
mod tools;
mod db;
mod bridge;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tauri::Manager;

use crate::bridge::source::BridgeSource;
use crate::bridge::types::SourceKind;

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
                db::seed::seed_advantages(&pool).await
                    .expect("Failed to seed advantages");
                handle.manage(DbState(Arc::new(pool)));

                // Bridge layer — sources registered here, accept loops spawned.
                let mut sources: HashMap<SourceKind, Arc<dyn BridgeSource>> = HashMap::new();
                sources.insert(SourceKind::Roll20, Arc::new(bridge::roll20::Roll20Source));
                sources.insert(SourceKind::Foundry, Arc::new(bridge::foundry::FoundrySource));

                let foundry_tls = match bridge::tls::ensure_cert(&app_data_dir).await {
                    Ok(acc) => Some(acc),
                    Err(e) => {
                        eprintln!("[bridge] TLS init failed (Foundry wss disabled): {e}");
                        None
                    }
                };

                let bridge_state = Arc::new(bridge::BridgeState::new(sources));
                handle.manage(bridge::BridgeConn(Arc::clone(&bridge_state)));
                tauri::async_runtime::spawn(bridge::start_servers(
                    bridge_state,
                    handle.clone(),
                    foundry_tls,
                ));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tools::resonance::roll_resonance,
            tools::skill_check::roll_skill_check,
            db::dyscrasia::list_dyscrasias,
            db::dyscrasia::add_dyscrasia,
            db::dyscrasia::update_dyscrasia,
            db::dyscrasia::delete_dyscrasia,
            db::dyscrasia::roll_random_dyscrasia,
            tools::export::export_result_to_md,
            tools::foundry_chat::trigger_foundry_roll,
            tools::foundry_chat::post_foundry_chat,
            tools::character::character_set_field,
            tools::character::character_add_advantage,
            tools::character::character_remove_advantage,
            bridge::commands::bridge_get_characters,
            bridge::commands::bridge_get_rolls,
            bridge::commands::bridge_get_status,
            bridge::commands::bridge_refresh,
            bridge::commands::bridge_set_attribute,
            bridge::commands::bridge_get_source_info,
            db::saved_character::save_character,
            db::saved_character::list_saved_characters,
            db::saved_character::update_saved_character,
            db::saved_character::delete_saved_character,
            db::saved_character::patch_saved_field,
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
            db::modifier::list_character_modifiers,
            db::modifier::list_all_character_modifiers,
            db::modifier::add_character_modifier,
            db::modifier::update_character_modifier,
            db::modifier::delete_character_modifier,
            db::modifier::set_modifier_active,
            db::modifier::set_modifier_hidden,
            db::modifier::set_modifier_zone,
            db::modifier::materialize_advantage_modifier,
            db::status_template::list_status_templates,
            db::status_template::add_status_template,
            db::status_template::update_status_template,
            db::status_template::delete_status_template,
            tools::gm_screen::gm_screen_push_to_foundry,
            db::advantage::list_advantages,
            db::advantage::add_advantage,
            db::advantage::update_advantage,
            db::advantage::delete_advantage,
            db::advantage::roll_random_advantage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
