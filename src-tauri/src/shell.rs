use uuid::Uuid;

use crate::core::models::{AppSelectionState, ModelGroup};

pub use crate::shell_lifecycle::{WindowLifecycleState, WindowRestoreStep};

pub const SHOW_QUICK_SWITCH: &str = "show_quick_switch";
pub const OPEN_SETTINGS: &str = "open_settings";
pub const RELOAD_APP_STATE: &str = "reload_app_state";
pub const QUIT: &str = "quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAction {
    ShowQuickSwitch,
    OpenSettings,
    ReloadAppState,
    Quit,
    SwitchGroup(Uuid),
}

impl ShellAction {
    pub fn id(self) -> String {
        match self {
            Self::ShowQuickSwitch => SHOW_QUICK_SWITCH.to_owned(),
            Self::OpenSettings => OPEN_SETTINGS.to_owned(),
            Self::ReloadAppState => RELOAD_APP_STATE.to_owned(),
            Self::Quit => QUIT.to_owned(),
            Self::SwitchGroup(group_id) => format!("switch_group:{group_id}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellMenuItemKind {
    Reload,
    QuickSwitch,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellMenuItem {
    pub kind: ShellMenuItemKind,
    pub title: String,
    pub action: ShellAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellGroupMenuItem {
    pub id: Uuid,
    pub title: String,
    pub active: bool,
    pub action: ShellAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellMenuModel {
    pub current_group_label: String,
    pub group_items: Vec<ShellGroupMenuItem>,
    pub shell_items: Vec<ShellMenuItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellStateSnapshot {
    pub groups: Vec<ModelGroup>,
    pub app_state: AppSelectionState,
}

impl ShellStateSnapshot {
    pub fn menu_model(&self) -> ShellMenuModel {
        let active_group = self
            .app_state
            .selected_group_id
            .and_then(|id| self.groups.iter().find(|group| group.id == id));
        let active_group_id = active_group.map(|group| group.id);
        let current_group_name = active_group
            .map(|group| group.name.as_str())
            .unwrap_or("None");
        ShellMenuModel {
            current_group_label: format!("Current Group: {current_group_name}"),
            group_items: self
                .groups
                .iter()
                .filter(|group| group.is_enabled)
                .map(|group| ShellGroupMenuItem {
                    id: group.id,
                    title: group.name.clone(),
                    active: Some(group.id) == active_group_id,
                    action: ShellAction::SwitchGroup(group.id),
                })
                .collect(),
            shell_items: shell_items(),
        }
    }
}

pub fn resolve_shell_action(id: &str) -> Option<ShellAction> {
    match id {
        SHOW_QUICK_SWITCH => Some(ShellAction::ShowQuickSwitch),
        OPEN_SETTINGS => Some(ShellAction::OpenSettings),
        RELOAD_APP_STATE => Some(ShellAction::ReloadAppState),
        QUIT => Some(ShellAction::Quit),
        _ => id
            .strip_prefix("switch_group:")
            .and_then(|group_id| Uuid::parse_str(group_id).ok())
            .map(ShellAction::SwitchGroup),
    }
}

#[derive(Debug, Default)]
pub struct ShellActionRecorder {
    events: Vec<&'static str>,
}

impl ShellActionRecorder {
    pub fn dispatch(&mut self, action: ShellAction) {
        match action {
            ShellAction::ShowQuickSwitch => self.events.push(SHOW_QUICK_SWITCH),
            ShellAction::OpenSettings => self.events.push(OPEN_SETTINGS),
            ShellAction::ReloadAppState => self.events.push(RELOAD_APP_STATE),
            ShellAction::Quit => self.events.push(QUIT),
            ShellAction::SwitchGroup(_) => self.events.push("switch_group"),
        }
    }

    pub fn events(&self) -> &[&'static str] {
        &self.events
    }
}

fn shell_items() -> Vec<ShellMenuItem> {
    vec![
        ShellMenuItem {
            kind: ShellMenuItemKind::Reload,
            title: "Reload".to_owned(),
            action: ShellAction::ReloadAppState,
        },
        ShellMenuItem {
            kind: ShellMenuItemKind::QuickSwitch,
            title: "Quick Switch".to_owned(),
            action: ShellAction::ShowQuickSwitch,
        },
        ShellMenuItem {
            kind: ShellMenuItemKind::Quit,
            title: "Quit".to_owned(),
            action: ShellAction::Quit,
        },
    ]
}
