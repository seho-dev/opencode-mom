pub mod agents;
pub mod agents_md;
pub mod commands;
pub mod document;
pub mod error;
pub mod groups;
pub mod jsonc;
pub mod mcp;
pub mod models;
pub mod opencode_cli;
pub mod paths;
pub mod power;
pub mod projection;
pub mod providers;
pub mod refs;
pub mod replacement;
mod resource_file;
pub mod skills;
mod tray;
pub mod updates;

use tauri::Manager;

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let builder = builder.on_window_event(tray::on_window_event);
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_autostart::Builder::new().build());
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = tray::open_main(app, None) {
                eprintln!("failed to restore main window: {error}");
            }
        }))
        .setup(|app| {
            let paths = paths::ConfigPaths::from_environment()?;
            app.manage(power::PowerService::new(paths.app_config_dir())?);
            app.manage(paths);
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            tray::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_app_state,
            commands::get_app_info,
            commands::check_for_updates,
            commands::open_project_page,
            commands::list_providers,
            commands::list_custom_providers,
            commands::opencode_list_models,
            commands::opencode_resolve_binary,
            commands::opencode_reload,
            commands::opencode_token_usage_records,
            commands::opencode_active_sessions,
            commands::get_lid_protection,
            commands::set_lid_protection,
            commands::open_main_window,
            commands::hide_tray_window,
            commands::quit_app,
            commands::create_provider,
            commands::update_provider,
            commands::delete_provider,
            commands::list_models,
            commands::create_model,
            commands::update_model,
            commands::delete_model,
            commands::list_agents,
            commands::get_agent,
            commands::create_agent,
            commands::update_agent,
            commands::delete_agent,
            commands::list_mcps,
            commands::get_mcp,
            commands::create_mcp,
            commands::update_mcp,
            commands::delete_mcp,
            commands::list_skills,
            commands::get_skill,
            commands::create_skill,
            commands::update_skill,
            commands::get_autostart,
            commands::set_autostart,
            commands::save_group,
            commands::copy_group,
            commands::delete_group,
            commands::switch_group,
            commands::save_preferences
        ])
        .build(tauri::generate_context!())
        .expect("failed to run omo-switch Tauri shell")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(service) = app.try_state::<power::PowerService>() {
                    service.shutdown();
                }
            }
        });
}
