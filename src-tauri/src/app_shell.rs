use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    App, AppHandle, Manager, Runtime, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::application::GroupApplicationService;
use crate::shell::{resolve_shell_action, ShellAction, ShellStateSnapshot};

pub fn build_tray(app: &mut App) -> tauri::Result<()> {
    let state = app.state::<GroupApplicationService>();
    let menu = build_menu(app, &state)?;

    TrayIconBuilder::with_id("omo-switch-tray")
        .tooltip("omo-switch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(action) = resolve_shell_action(event.id().as_ref()) {
                dispatch_shell_action(app, action);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                dispatch_shell_action(tray.app_handle(), ShellAction::OpenSettings);
            }
        })
        .build(app)?;

    Ok(())
}

pub fn dispatch_shell_action<R: Runtime>(app: &AppHandle<R>, action: ShellAction) {
    match action {
        ShellAction::ShowQuickSwitch => {
            report_shell_error(restore_window(app, "quick-switch", Position::TrayCenter))
        }
        ShellAction::OpenSettings => {
            if app.get_webview_window("settings").is_some() {
                report_shell_error(open_settings(app));
            } else {
                let app = app.clone();
                std::thread::spawn(move || report_shell_error(open_settings(&app)));
            }
        }
        ShellAction::ReloadAppState => refresh_tray(app),
        ShellAction::Quit => app.exit(0),
        ShellAction::SwitchGroup(group_id) => {
            if let Some(application) = app.try_state::<GroupApplicationService>() {
                let _ = application.switch_group(group_id);
            }
            refresh_tray(app);
        }
    }
}

pub fn open_settings<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if app.get_webview_window("settings").is_none() {
        let window =
            WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("/settings".into()))
                .title("omo-switch")
                .inner_size(980.0, 620.0)
                .visible(false)
                .resizable(true)
                .build()?;
        install_hide_on_close(&window);
    }

    restore_window(app, "settings", Position::Center)
}

pub fn install_window_lifecycle(app: &App) {
    for label in ["quick-switch", "settings"] {
        if let Some(window) = app.get_webview_window(label) {
            install_hide_on_close(&window);
        }
    }
}

fn install_hide_on_close<R: Runtime>(window: &WebviewWindow<R>) {
    let label = window.label().to_owned();
    window.on_window_event({
        let window = window.clone();
        move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("failed to hide {label} window: {error}");
                }
            }
        }
    });
}

pub fn refresh_tray<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id("omo-switch-tray") else {
        return;
    };
    let Some(application) = app.try_state::<GroupApplicationService>() else {
        return;
    };
    if let Ok(menu) = build_menu(app, &application) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu<R: Runtime, M: Manager<R>>(
    manager: &M,
    application: &State<'_, GroupApplicationService>,
) -> tauri::Result<Menu<R>> {
    let model = load_shell_state(application).menu_model();
    let current_group = MenuItem::with_id(
        manager,
        "current_group",
        model.current_group_label,
        false,
        None::<&str>,
    )?;
    let menu = Menu::with_items(manager, &[&current_group])?;
    let separator = PredefinedMenuItem::separator(manager)?;
    menu.append(&separator)?;

    for group in model.group_items {
        let item = CheckMenuItem::with_id(
            manager,
            group.action.id(),
            group.title,
            true,
            group.active,
            None::<&str>,
        )?;
        menu.append(&item)?;
    }

    let separator = PredefinedMenuItem::separator(manager)?;
    menu.append(&separator)?;
    for item in model.shell_items {
        let menu_item =
            MenuItem::with_id(manager, item.action.id(), item.title, true, None::<&str>)?;
        menu.append(&menu_item)?;
    }
    Ok(menu)
}

fn load_shell_state(application: &GroupApplicationService) -> ShellStateSnapshot {
    match application.load_app_state() {
        Ok(response) => ShellStateSnapshot {
            groups: response.groups,
            app_state: response.app_state,
        },
        Err(_) => ShellStateSnapshot {
            groups: Vec::new(),
            app_state: crate::core::models::AppSelectionState::default(),
        },
    }
}

fn restore_window<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
    position: Position,
) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };

    activate_app(app)?;
    if window.is_minimized()? {
        window.unminimize()?;
    }
    window.move_window_constrained(position)?;
    window.show()?;
    window.set_focus()
}

#[cfg(target_os = "macos")]
fn activate_app<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;
    app.show()
}

#[cfg(not(target_os = "macos"))]
fn activate_app<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

fn report_shell_error(result: tauri::Result<()>) {
    if let Err(error) = result {
        eprintln!("shell lifecycle action failed: {error}");
    }
}
