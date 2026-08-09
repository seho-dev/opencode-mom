#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use omo_switch_tauri::core::models::{
    AppSelectionState, ModelGroup, ModelGroupAgentOverride, ModelGroupCategoryMapping,
};
use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};
use omo_switch_tauri::core::repository::{
    AppStateRepository, ModelGroupRepository, OhMyOpenAgentConfigRepository,
    OpenCodeConfigRepository,
};
use omo_switch_tauri::core::switching::SwitchOutcome;
use omo_switch_tauri::core::switching::{SwitchGroupRepositories, SwitchGroupUseCase};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub fn temp_home(label: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("task-8-temp")
        .join(format!("omo-switch-{label}-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("Given: temp home directory is created");
    path
}

pub fn remove_temp(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub fn paths(home: &Path) -> ConfigPaths {
    ConfigPaths::from_home_env(HomeEnv::new(Some(home), None)).expect("Given: HOME path resolves")
}

pub fn category_row(category_name: &str, model_ref: &str) -> ModelGroupCategoryMapping {
    ModelGroupCategoryMapping {
        category_name: category_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}

pub fn override_row(agent_name: &str, model_ref: &str) -> ModelGroupAgentOverride {
    ModelGroupAgentOverride {
        agent_name: agent_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}

pub fn group(
    id: &str,
    name: &str,
    is_enabled: bool,
    category_mappings: Vec<ModelGroupCategoryMapping>,
    agent_overrides: Vec<ModelGroupAgentOverride>,
    open_code_agent_overrides: Vec<ModelGroupAgentOverride>,
) -> ModelGroup {
    ModelGroup {
        id: Uuid::parse_str(id).expect("Given: UUID fixture parses"),
        name: name.to_owned(),
        description: None,
        category_mappings,
        agent_overrides,
        open_code_agent_overrides,
        is_enabled,
        updated_at: instant("2023-11-14T22:13:20Z"),
    }
}

pub fn use_case(home: &Path) -> SwitchGroupUseCase<fn() -> OffsetDateTime> {
    let resolved = paths(home);
    let repositories = SwitchGroupRepositories {
        model_groups: ModelGroupRepository::new(resolved.groups_file()),
        app_state: AppStateRepository::new(resolved.state_file()),
        backups_root: resolved.omo_switch_dir(),
        opencode: OpenCodeConfigRepository::new(resolved.opencode_file()),
        oh_my_openagent: OhMyOpenAgentConfigRepository::new(resolved.oh_my_openagent_file()),
    };
    SwitchGroupUseCase::new(repositories, fixed_now)
}

pub fn seed_groups(home: &Path, groups: &[ModelGroup]) {
    let repo = ModelGroupRepository::new(paths(home).groups_file());
    repo.save(groups).expect("Given: groups save");
}

pub fn seed_state(home: &Path, selected_group_id: Option<Uuid>, selected_group_name: Option<&str>) {
    let repo = AppStateRepository::new(paths(home).state_file());
    let state = AppSelectionState {
        selected_group_id,
        selected_group_name: selected_group_name.map(str::to_owned),
        ..AppSelectionState::default()
    };
    repo.save(&state).expect("Given: app state saves");
}

pub fn seed_oh_my(home: &Path) {
    let resolved = paths(home);
    fs::create_dir_all(resolved.opencode_dir()).expect("Given: opencode dir exists");
    fs::write(
        resolved.oh_my_openagent_file(),
        fixture("ohmy/current-oh-my-openagent.json"),
    )
    .expect("Given: Oh My fixture writes");
}

pub fn seed_opencode(home: &Path) {
    let resolved = paths(home);
    fs::create_dir_all(resolved.opencode_dir()).expect("Given: opencode dir exists");
    fs::write(
        resolved.opencode_file(),
        fixture("opencode/current-opencode.json"),
    )
    .expect("Given: OpenCode fixture writes");
}

pub fn assert_dual_target_success(home: &Path, target_group: &ModelGroup, outcome: SwitchOutcome) {
    let resolved = paths(home);
    let oh_my = OhMyOpenAgentConfigRepository::new(resolved.oh_my_openagent_file())
        .load()
        .expect("Then: Oh My loads");
    let opencode = OpenCodeConfigRepository::new(resolved.opencode_file())
        .load()
        .expect("Then: OpenCode loads");
    assert!(matches!(outcome, SwitchOutcome::Success { .. }));
    assert_eq!(
        oh_my.agents()["oracle"]["model"],
        "cliproxyapi/gpt-5.4-xhigh"
    );
    assert_eq!(
        oh_my.categories()["unspecified-high"]["model"],
        "cliproxyapi/gpt-5.4-xhigh"
    );
    assert_eq!(
        opencode.agents()["creative-ui-coder"]["model"],
        "cliproxyapi/gpt-5.4"
    );
    assert_eq!(
        opencode.agents()["Jenny"]["model"],
        "cliproxyapi/gpt-5.4-xhigh"
    );
    assert!(opencode.raw().get("$schema").is_some());
    assert!(opencode.raw().get("plugin").is_some());
    assert!(opencode.raw().get("provider").is_some());
    assert!(opencode.agents().get("karen").is_some());
    let state = AppStateRepository::new(resolved.state_file())
        .load()
        .expect("Then: state loads");
    assert_eq!(state.selected_group_id, Some(target_group.id));
    assert_eq!(state.selected_group_name.as_deref(), Some("Dual Target"));
    assert_eq!(
        state
            .last_successful_write
            .as_ref()
            .map(|write| write.target.as_str()),
        Some("switch-group")
    );
    assert!(state
        .last_successful_write
        .as_ref()
        .and_then(|write| write.backup_path.as_ref())
        .is_some_and(
            |summary| summary.contains("opencode:") && summary.contains("oh-my-openagent:")
        ));
}

pub fn instant(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse(value, &Rfc3339).expect("Given: RFC3339 fixture parses")
}

fn fixed_now() -> OffsetDateTime {
    instant("2023-11-14T22:21:40Z")
}

fn fixture(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Given: src-tauri has workspace parent")
        .join("fixtures")
        .join("compatibility")
        .join(relative_path);
    fs::read_to_string(path).expect("Given: fixture reads")
}
