pub mod agents;
pub mod agents_md;
pub mod behavior;
pub mod commands;
pub mod custom_commands;
pub mod document;
pub mod error;
pub mod groups;
pub(crate) mod http_server;
mod island;
pub mod jsonc;
pub(crate) mod logging;
pub mod mcp;
pub(crate) mod mcp_proxy;
pub mod models;
mod notifications;
pub mod opencode;
pub mod paths;
pub mod power;
pub(crate) mod private_file;
pub mod projection;
pub(crate) mod protected_data;
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
    let notification_owner = std::sync::Arc::new(std::sync::Mutex::new(None));
    let setup_notification_owner = notification_owner.clone();
    // 0: running, 1: shutting down, 2: ready to exit.
    let exit_flag = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));
    let builder = tauri::Builder::default();
    let builder = builder.on_window_event(|window, event| {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        tray::on_window_event(window, event);
        island::on_window_event(window, event);
    });
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_autostart::Builder::new().build());
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let builder = builder.plugin(tauri_plugin_notification::init());
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = tray::open_main(app, None) {
                eprintln!("failed to restore main window: {error}");
            }
        }))
        .setup(move |app| {
            let paths = paths::ConfigPaths::from_environment()?;
            let logging = logging::LoggingService::new(&paths);
            logging.install();
            app.manage(logging);
            opencode::target::initialize(&paths)?;
            if let Ok(directory) = app.path().app_cache_dir() {
                let _ = std::fs::create_dir_all(&directory);
                opencode::initialize_usage_cache(directory);
            }
            let proxy = std::sync::Arc::new(mcp_proxy::ProxyService::new(&paths).unwrap_or_else(
                |error| {
                    eprintln!("warning: MCP proxy initialization failed; disabling proxy: {error}");
                    mcp_proxy::ProxyService::new_disabled(&paths)
                },
            ));
            app.manage(proxy.clone());
            app.manage(power::PowerService::new(paths.app_config_dir())?);
            app.manage(paths.clone());
            setup_http(app, &paths, &proxy);
            *setup_notification_owner
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(app.handle().clone());
            notifications::setup(
                app.handle(),
                &paths,
                std::sync::Arc::downgrade(&setup_notification_owner),
            );
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
            logging::record(
                logging::LogType::App,
                logging::LogStatus::Success,
                "app.started",
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_app_state,
            commands::get_logging_state,
            commands::get_notification_settings,
            commands::get_notification_permission,
            commands::request_notification_permission,
            commands::open_notification_settings,
            commands::set_notification_settings,
            commands::set_logging_enabled,
            commands::query_logs,
            commands::open_logs_directory,
            commands::clear_logs,
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
            commands::opencode_list_models,
            commands::opencode_generate_prompt,
            commands::opencode_get_installation,
            commands::opencode_get_service_status,
            commands::opencode_set_service_target,
            commands::opencode_check_update,
            commands::opencode_upgrade,
            commands::opencode_reload,
            commands::opencode_token_usage_records,
            commands::opencode_token_usage_snapshot,
            commands::opencode_active_sessions,
            commands::get_lid_protection,
            commands::set_lid_protection,
            commands::open_main_window,
            commands::hide_tray_window,
            commands::quit_app,
            commands::create_provider,
            commands::update_provider,
            commands::delete_provider,
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
            commands::set_mcp_disabled,
            commands::delete_mcp,
            commands::list_skills,
            commands::get_skill,
            commands::create_skill,
            commands::update_skill,
            commands::list_slash_commands,
            commands::get_slash_command,
            commands::create_slash_command,
            commands::update_slash_command,
            commands::delete_slash_command,
            commands::list_plugin_runtime,
            commands::update_plugin_package,
            commands::prepare_plugin_removal,
            commands::remove_plugin_declaration,
            commands::get_global_behavior,
            commands::save_global_behavior,
            commands::get_autostart,
            commands::set_autostart,
            commands::save_group,
            commands::delete_group,
            commands::switch_group,
            commands::save_preferences,
            commands::get_mom_http_settings,
            commands::set_mom_http_settings,
            commands::get_mcp_proxy_state,
            commands::get_mcp_proxy_settings,
            commands::save_mcp_proxy_settings,
            commands::rotate_mom_http_key,
            commands::get_mcp_proxy_server,
            commands::create_mcp_proxy_server,
            commands::update_mcp_proxy_server,
            commands::delete_mcp_proxy_server,
            commands::import_mcp_proxy_server,
            commands::list_mcp_proxy_processes,
            commands::close_mcp_proxy_unit
        ])
        .build(tauri::generate_context!())
        .expect("failed to run omo-switch Tauri shell")
        .run(move |app, event| match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if exit_flag.load(std::sync::atomic::Ordering::SeqCst) != 2 {
                    api.prevent_exit();
                }

                if exit_flag
                    .compare_exchange(
                        0,
                        1,
                        std::sync::atomic::Ordering::SeqCst,
                        std::sync::atomic::Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    let app_handle = app.clone();
                    if let Some(service) = app.try_state::<notifications::NotificationService>() {
                        service.shutdown();
                    }
                    let exit_code = code.unwrap_or(0);
                    if let Some(proxy) = app.try_state::<std::sync::Arc<mcp_proxy::ProxyService>>()
                    {
                        let proxy = proxy.inner().clone();
                        let exit_flag = exit_flag.clone();
                        tauri::async_runtime::spawn(async move {
                            shutdown_http(&app_handle, &proxy, std::time::Duration::from_secs(8))
                                .await;
                            exit_flag.store(2, std::sync::atomic::Ordering::SeqCst);
                            app_handle.exit(exit_code);
                        });
                    } else {
                        exit_flag.store(2, std::sync::atomic::Ordering::SeqCst);
                        app_handle.exit(exit_code);
                    }
                }
            }
            tauri::RunEvent::Exit => {
                if let Some(service) = app.try_state::<notifications::NotificationService>() {
                    service.shutdown();
                }
                notification_owner
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take();
                if let Some(proxy) = app.try_state::<std::sync::Arc<mcp_proxy::ProxyService>>() {
                    let proxy = proxy.inner().clone();
                    tauri::async_runtime::block_on(async move {
                        shutdown_http(app, &proxy, std::time::Duration::from_secs(1)).await;
                    });
                }

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
                logging::record(
                    logging::LogType::App,
                    logging::LogStatus::Success,
                    "app.stopped",
                );
                if let Some(service) = app.try_state::<std::sync::Arc<logging::LoggingService>>() {
                    service.shutdown(std::time::Duration::from_secs(2));
                }
            }
            _ => {}
        });
}

fn setup_http<R: tauri::Runtime>(
    app: &tauri::App<R>,
    paths: &paths::ConfigPaths,
    proxy: &mcp_proxy::ProxyService,
) {
    let http: http_server::HttpServerState =
        http_server::HttpServer::new(paths, axum::Router::new().merge(proxy.router()))
            .map(std::sync::Arc::new);
    app.manage(http.clone());

    match http {
        Ok(http) => {
            tauri::async_runtime::spawn(async move {
                if let Err(error) = http.start().await {
                    eprintln!("warning: failed to start MOM HTTP service: {error}");
                }
            });
        }
        Err(error) => {
            eprintln!(
                "warning: MOM HTTP service initialization failed; disabling HTTP service: {error}"
            );
        }
    }
}

async fn shutdown_http<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    proxy: &mcp_proxy::ProxyService,
    deadline: std::time::Duration,
) {
    let (http_result, proxy_result) = tokio::join!(
        async {
            if let Some(http) = app.try_state::<http_server::HttpServerState>() {
                if let Ok(http) = http.inner().as_ref() {
                    http.shutdown(deadline).await
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        },
        proxy.shutdown(deadline),
    );

    for error in [http_result, proxy_result]
        .into_iter()
        .filter_map(Result::err)
    {
        eprintln!("warning: failed to cleanly shut down MOM HTTP service: {error}");
    }
}
