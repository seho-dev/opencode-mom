use std::fs;

use omo_switch_tauri::core::models::AppSelectionState;
use omo_switch_tauri::shell::{
    ShellAction, ShellMenuItemKind, ShellStateSnapshot, OPEN_SETTINGS, QUIT, RELOAD_APP_STATE,
    SHOW_QUICK_SWITCH,
};

#[test]
fn tray_contract_when_scaffolded_preserves_primary_window_action_and_right_click_labels() {
    // Given: the Settings main window is opened by a primary-window action, not a tray menu item.
    // When: the tray contract is inspected.
    // Then: its action contract retains primary-window opening, while labels list right-click items.
    assert_eq!(SHOW_QUICK_SWITCH, "show_quick_switch");
    assert_eq!(OPEN_SETTINGS, "open_settings");
    assert_eq!(RELOAD_APP_STATE, "reload_app_state");
    assert_eq!(QUIT, "quit");
    assert_eq!(ShellAction::OpenSettings.id(), OPEN_SETTINGS);
    let menu = ShellStateSnapshot {
        groups: Vec::new(),
        app_state: AppSelectionState::default(),
    }
    .menu_model();
    assert_eq!(
        menu.shell_items
            .iter()
            .map(|item| (item.kind, item.title.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (ShellMenuItemKind::Reload, "Reload"),
            (ShellMenuItemKind::QuickSwitch, "Quick Switch"),
            (ShellMenuItemKind::Quit, "Quit"),
        ]
    );
}

#[test]
fn tauri_config_when_scaffolded_uses_static_sveltekit_build() {
    // Given: Tauri must consume SvelteKit's adapter-static output.
    // When: the current checked-in config is inspected.
    // Then: frontendDist points at ../build and no dev URL is required for build.
    let config = fs::read_to_string("tauri.conf.json").expect("tauri.conf.json must exist");

    assert!(config.contains(r#""frontendDist": "../build""#));
    assert!(!config.contains("../public"));
}

#[test]
fn cargo_manifest_when_scaffolded_enables_tauri_tray_icon_feature() {
    // Given: tray support is a platform viability gate.
    // When: the Rust manifest is inspected.
    // Then: Tauri compiles with the tray-icon feature.
    let manifest = fs::read_to_string("Cargo.toml").expect("Cargo.toml must exist");

    assert!(manifest.contains("tray-icon"));
}
