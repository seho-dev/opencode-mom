pub mod agents;
pub mod agents_md;
pub mod commands;
pub mod document;
pub mod dto;
pub mod error;
pub mod groups;
pub mod jsonc;
pub mod models;
pub mod paths;
pub mod projection;
pub mod providers;
pub mod refs;
pub mod replacement;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_minimized().unwrap_or(false) {
                    let _ = window.unminimize();
                }
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let paths = paths::ConfigPaths::from_environment()?;
            app.manage(paths);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_app_state,
            commands::list_providers,
            commands::reveal_provider_option,
            commands::create_provider,
            commands::update_provider,
            commands::delete_provider,
            commands::list_models,
            commands::create_model,
            commands::update_model,
            commands::delete_model,
            commands::replace_model_references,
            commands::rename_model,
            commands::rename_provider,
            commands::list_agents,
            commands::get_agent,
            commands::create_agent,
            commands::update_agent,
            commands::delete_agent,
            commands::save_group,
            commands::copy_group,
            commands::delete_group,
            commands::switch_group,
            commands::load_config_diagnostics,
            commands::validate_config
        ])
        .run(tauri::generate_context!())
        .expect("failed to run omo-switch Tauri shell")
}
