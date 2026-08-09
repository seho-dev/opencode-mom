use std::fs;

#[test]
fn quick_switch_when_reopened_restores_and_focuses_in_required_order() {
    let source = app_shell_source();
    let restore_window = function_body(&source, "fn restore_window");

    assert_in_order(
        restore_window,
        &["unminimize", "move_window", "show()", "set_focus()"],
    );
}

#[test]
fn close_request_when_not_explicit_quit_hides_to_tray() {
    let source = app_shell_source();

    assert!(
        source.contains("on_window_event") && source.contains("CloseRequested"),
        "native shell has no close-request interception"
    );
    assert!(
        source.contains("prevent_close") && source.contains("hide()"),
        "close requests are not distinguished from explicit quit"
    );
}

#[test]
fn second_instance_when_activated_restores_the_settings_main_window() {
    let source = lib_source();
    let callback = function_body(&source, "tauri_plugin_single_instance::init")
        .split_once("}))")
        .map(|(body, _)| body)
        .expect("Given: second-instance callback ends");

    assert!(callback.contains("OpenSettings"));
    assert!(
        !callback.contains("ShowQuickSwitch"),
        "second-instance activation must not divert to the quick-switch shortcut"
    );
    assert!(
        callback.contains("dispatch_shell_action"),
        "second-instance callback does not use the unified shell controller"
    );
    assert!(
        !callback.contains("WebviewWindowBuilder") && !callback.contains("create_window"),
        "second-instance callback must not create a duplicate settings window"
    );
}

#[test]
fn settings_main_window_starts_visible_and_missing_instances_restore_with_close_lifecycle() {
    let source = app_shell_source();
    let config: serde_json::Value =
        serde_json::from_str(&tauri_config_source()).expect("Given: valid Tauri config");
    let windows = config["app"]["windows"]
        .as_array()
        .expect("Given: Tauri window configs");
    let settings_window = windows
        .iter()
        .find(|window| window["label"] == "settings")
        .expect("Given: settings window config");
    let quick_switch_window = windows
        .iter()
        .find(|window| window["label"] == "quick-switch")
        .expect("Given: quick-switch window config");

    assert_eq!(settings_window["url"], "/settings");
    assert_eq!(settings_window["title"], "omo-switch");
    assert_eq!(settings_window["visible"], true);
    assert_eq!(quick_switch_window["visible"], false);
    assert_eq!(
        windows
            .iter()
            .filter(|window| window["label"] == "settings")
            .count(),
        1,
        "settings has exactly one configured window"
    );

    let open_settings = named_function_body(&source, "pub fn open_settings");
    assert!(
        open_settings.contains("if app.get_webview_window(\"settings\").is_none()"),
        "settings must be created only when its configured window is missing"
    );
    assert!(
        open_settings.contains(
            "WebviewWindowBuilder::new(app, \"settings\", WebviewUrl::App(\"/settings\".into()))"
        ),
        "missing settings window must use Tauri's App-relative /settings URL"
    );
    assert!(
        open_settings.contains(".title(\"omo-switch\")"),
        "dynamically created settings windows must retain the product title"
    );
    assert!(
        open_settings.contains(".visible(false)"),
        "dynamically created settings windows must remain hidden until restored"
    );
    assert_in_order(
        open_settings,
        &[
            "WebviewWindowBuilder::new",
            ".build()?",
            "install_hide_on_close(&window)",
            "restore_window(app, \"settings\", Position::Center)",
        ],
    );

    let hide_on_close = named_function_body(&source, "fn install_hide_on_close");
    assert!(
        hide_on_close.contains("on_window_event")
            && hide_on_close.contains("CloseRequested")
            && hide_on_close.contains("prevent_close")
            && hide_on_close.contains("hide()"),
        "new settings windows must hide instead of closing"
    );

    let restore_window = named_function_body(&source, "fn restore_window");
    assert_in_order(restore_window, &["show()", "set_focus()"]);
}

#[test]
fn tray_left_click_when_activated_restores_the_settings_main_window() {
    let source = app_shell_source();
    let callback = source
        .split_once(".on_tray_icon_event(|tray, event| {")
        .and_then(|(_, suffix)| suffix.split_once("\n        })\n        .build"))
        .map(|(callback, _)| callback)
        .expect("Given: tray icon callback exists");

    assert!(callback.contains("button: MouseButton::Left"));
    assert!(callback.contains("ShellAction::OpenSettings"));
    assert!(
        !callback.contains("ShellAction::ShowQuickSwitch"),
        "left-click must restore the settings main window instead of the quick-switch shortcut"
    );
}

fn app_shell_source() -> String {
    fs::read_to_string("src/app_shell.rs").expect("Given: src/app_shell.rs exists")
}

fn lib_source() -> String {
    fs::read_to_string("src/lib.rs").expect("Given: src/lib.rs exists")
}

fn tauri_config_source() -> String {
    fs::read_to_string("tauri.conf.json").expect("Given: tauri.conf.json exists")
}

fn function_body<'a>(source: &'a str, marker: &str) -> &'a str {
    source
        .split_once(marker)
        .map(|(_, body)| body)
        .expect("Given: expected source marker exists")
}

fn named_function_body<'a>(source: &'a str, marker: &str) -> &'a str {
    let source = function_body(source, marker);
    let start = source
        .find('{')
        .expect("Given: expected function body start");
    let mut depth = 0;

    for (index, character) in source[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start + 1..start + index];
                }
            }
            _ => {}
        }
    }

    panic!("Given: expected function body end");
}

fn assert_in_order(source: &str, markers: &[&str]) {
    let mut remainder = source;
    for marker in markers {
        let (_, next) = remainder
            .split_once(marker)
            .unwrap_or_else(|| panic!("missing required lifecycle step: {marker}"));
        remainder = next;
    }
}
