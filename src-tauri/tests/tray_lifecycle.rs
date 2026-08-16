use omo_switch_tauri::core::models::{AppSelectionState, ModelGroup};
use omo_switch_tauri::shell::{
    resolve_shell_action, ShellAction, ShellMenuItemKind, ShellStateSnapshot,
};
use uuid::Uuid;

#[test]
fn tray_menu_when_state_has_groups_renders_enabled_groups_and_active_marker() {
    let active_id = Uuid::new_v4();
    let disabled_id = Uuid::new_v4();
    let inactive_id = Uuid::new_v4();
    let snapshot = ShellStateSnapshot {
        groups: vec![
            group(active_id, "Active", true),
            group(disabled_id, "Disabled", false),
            group(inactive_id, "Candidate", true),
        ],
        app_state: state(Some(active_id)),
    };

    let model = snapshot.menu_model();

    assert_eq!(model.current_group_label, "Current Group: Active");
    assert_eq!(model.group_items.len(), 2);
    assert_eq!(model.group_items[0].title, "Active");
    assert!(model.group_items[0].active);
    assert_eq!(model.group_items[1].title, "Candidate");
    assert!(!model.group_items[1].active);
    assert!(!model
        .group_items
        .iter()
        .any(|item| item.title == "Disabled"));
}

#[test]
fn tray_menu_when_current_group_is_stale_falls_back_to_none_without_marking_groups() {
    let stale_id = Uuid::new_v4();
    let group_id = Uuid::new_v4();
    let snapshot = ShellStateSnapshot {
        groups: vec![group(group_id, "Available", true)],
        app_state: state(Some(stale_id)),
    };

    let model = snapshot.menu_model();

    assert_eq!(model.current_group_label, "Current Group: None");
    assert!(!model.group_items[0].active);
}

#[test]
fn tray_menu_when_no_enabled_groups_contains_shell_actions_without_group_items() {
    let snapshot = ShellStateSnapshot {
        groups: vec![group(Uuid::new_v4(), "Disabled", false)],
        app_state: state(None),
    };

    let model = snapshot.menu_model();

    assert_eq!(model.current_group_label, "Current Group: None");
    assert_eq!(model.group_items, []);
    assert_eq!(
        model
            .shell_items
            .iter()
            .map(|item| item.kind)
            .collect::<Vec<_>>(),
        vec![
            ShellMenuItemKind::Reload,
            ShellMenuItemKind::QuickSwitch,
            ShellMenuItemKind::Quit,
        ]
    );
    assert!(
        !model
            .shell_items
            .iter()
            .any(|item| item.action == ShellAction::OpenSettings),
        "Settings is opened by the primary-window path, not a right-click tray item"
    );
    assert_eq!(
        resolve_shell_action("open_settings"),
        Some(ShellAction::OpenSettings),
        "the primary-window action remains available to tray-left-click and IPC callers"
    );
}

fn group(id: Uuid, name: &str, is_enabled: bool) -> ModelGroup {
    ModelGroup {
        id,
        name: name.to_owned(),
        description: None,
        category_mappings: Vec::new(),
        agent_overrides: Vec::new(),
        open_code_agent_overrides: Vec::new(),
        is_enabled,
        updated_at: time::OffsetDateTime::UNIX_EPOCH,
    }
}

fn state(selected_group_id: Option<Uuid>) -> AppSelectionState {
    AppSelectionState {
        selected_group_id,
        selected_group_name: None,
        last_successful_write: None,
        last_warning_summary: None,
        last_error_summary: None,
        migration_version: 1,
    }
}
