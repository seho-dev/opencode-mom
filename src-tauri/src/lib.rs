mod app_shell;
pub mod application;
pub mod commands;
pub mod core;
pub mod runtime;
pub mod shell;
mod shell_lifecycle;

use tauri::Manager;

use runtime::AppRuntime;

pub type LiveAppRuntime = AppRuntime;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app_shell::dispatch_shell_action(app, shell::ShellAction::OpenSettings);
        }))
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let application = application::GroupApplicationService::live()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let runtime = LiveAppRuntime::new(application);
            runtime
                .startup()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(runtime);
            app_shell::install_window_lifecycle(app);
            app_shell::build_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_app_state,
            commands::save_group,
            commands::copy_group,
            commands::delete_group,
            commands::switch_group,
            commands::discover_open_code_agents
        ])
        .run(tauri::generate_context!())
        .expect("failed to run omo-switch Tauri shell")
}
