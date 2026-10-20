pub mod agents;
pub mod agents_md;
pub mod commands;
pub mod document;
pub mod error;
pub mod groups;
mod island;
pub mod jsonc;
pub mod mcp;
pub mod models;
pub mod opencode_cli;
pub mod paths;
pub mod power;
pub mod projection;
pub mod providers;
pub mod refs;
mod resource_file;
pub mod skills;
mod tray;
pub mod updates;

use tauri::Manager;

pub fn run() {
    let island_notifications = std::sync::Arc::new(std::sync::Mutex::new(None));
    let setup_notifications = island_notifications.clone();
    let builder = tauri::Builder::default();
    let builder = builder.on_window_event(|window, event| {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        tray::on_window_event(window, event);
        island::on_window_event(window, event);
    });
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_autostart::Builder::new().build());
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = tray::open_main(app, None) {
                eprintln!("failed to restore main window: {error}");
            }
        }))
        .setup(move |app| {
            let paths = paths::ConfigPaths::from_environment()?;
            opencode_cli::target::initialize(&paths)?;
            app.manage(power::PowerService::new(paths.app_config_dir())?);
            app.manage(paths.clone());
            *setup_notifications
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(app.handle().clone());
            island::setup(
                app.handle(),
                &paths,
                std::sync::Arc::downgrade(&setup_notifications),
            );
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            tray::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_app_state,
            commands::get_island_state,
            commands::set_island_enabled,
            commands::refresh_island,
            commands::set_island_view,
            commands::island_messages,
            commands::island_send,
            commands::island_reply_permission,
            commands::island_reply_question,
            commands::island_cancel_question,
            commands::island_interrupt,
            commands::pick_project_folder,
            commands::add_project,
            commands::remove_project,
            commands::select_project,
            commands::get_app_info,
            commands::check_for_updates,
            commands::open_project_page,
            commands::list_providers,
            commands::opencode_list_models,
            commands::opencode_resolve_binary,
            commands::opencode_get_installation,
            commands::opencode_get_service_status,
            commands::opencode_set_service_target,
            commands::opencode_check_update,
            commands::opencode_upgrade,
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
        .run(move |app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(service) = app.try_state::<island::IslandService>() {
                    service.shutdown();
                }
                island_notifications
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take();
                if let Some(service) = app.try_state::<power::PowerService>() {
                    service.shutdown();
                }
            }
        });
}
