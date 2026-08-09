#[path = "common/switching.rs"]
mod switching_support;

use std::fs;

use omo_switch_tauri::core::switching::{SwitchError, SwitchOutcome};
use switching_support::{
    assert_dual_target_success, category_row, group, paths, remove_temp, seed_groups, seed_oh_my,
    seed_opencode, seed_state, temp_home, use_case,
};
use uuid::Uuid;

#[test]
fn switching_when_no_effective_opencode_overrides_skips_missing_opencode_config() {
    let home = temp_home("skip-opencode");
    seed_oh_my(&home);
    let target_group = group(
        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
        "Skip OpenCode",
        true,
        vec![category_row("quick", "cliproxyapi/gpt-5.4")],
        vec![],
        vec![switching_support::override_row("creative-ui-coder", "   ")],
    );
    seed_groups(&home, &[target_group.clone()]);

    let outcome = use_case(&home)
        .switch_to(target_group.id)
        .expect("When: switch succeeds");

    let resolved = paths(&home);
    assert!(matches!(outcome, SwitchOutcome::Success { .. }));
    assert!(!resolved.opencode_file().exists());
    assert!(resolved
        .omo_switch_dir()
        .join("backups")
        .join("oh-my-openagent")
        .exists());
    assert!(!resolved
        .omo_switch_dir()
        .join("backups")
        .join("opencode")
        .exists());

    remove_temp(&home);
}

#[test]
fn switching_when_target_is_current_group_returns_noop_without_touching_disk() {
    let home = temp_home("noop");
    seed_oh_my(&home);
    let target_group = group(
        "44444444-4444-4444-4444-444444444444",
        "ActiveGroup",
        true,
        vec![category_row("deep", "cliproxyapi/gpt-5.4")],
        vec![],
        vec![],
    );
    seed_groups(&home, &[target_group.clone()]);
    seed_state(&home, Some(target_group.id), Some("ActiveGroup"));
    let before =
        fs::read_to_string(paths(&home).oh_my_openagent_file()).expect("Given: Oh My reads");

    let outcome = use_case(&home)
        .switch_to(target_group.id)
        .expect("When: no-op succeeds");

    assert_eq!(outcome, SwitchOutcome::NoOp);
    assert_eq!(
        fs::read_to_string(paths(&home).oh_my_openagent_file()).expect("Then: Oh My reads"),
        before
    );
    assert!(!paths(&home).omo_switch_dir().join("backups").exists());

    remove_temp(&home);
}

#[test]
fn switching_when_group_is_missing_or_disabled_returns_typed_failure() {
    let home = temp_home("missing-disabled");
    seed_oh_my(&home);
    let disabled = group(
        "55555555-5555-5555-5555-555555555555",
        "Disabled",
        false,
        vec![category_row("quick", "model")],
        vec![],
        vec![],
    );
    seed_groups(&home, &[disabled.clone()]);

    let missing = use_case(&home)
        .switch_to(Uuid::parse_str("66666666-6666-6666-6666-666666666666").expect("UUID"))
        .expect_err("When: missing group fails");
    let disabled_error = use_case(&home)
        .switch_to(disabled.id)
        .expect_err("When: disabled group fails");

    assert_eq!(missing, SwitchError::GroupNotFound);
    assert_eq!(disabled_error, SwitchError::GroupDisabled);

    remove_temp(&home);
}

#[test]
fn switching_when_target_config_is_malformed_leaves_disk_unchanged() {
    let home = temp_home("malformed-oh-my");
    let resolved = paths(&home);
    fs::create_dir_all(resolved.opencode_dir()).expect("Given: opencode dir exists");
    fs::write(resolved.oh_my_openagent_file(), "{ invalid json").expect("Given: malformed writes");
    let target_group = group(
        "88888888-8888-8888-8888-888888888888",
        "Broken Input",
        true,
        vec![category_row("quick", "safe-model")],
        vec![],
        vec![],
    );
    seed_groups(&home, &[target_group.clone()]);

    let error = use_case(&home)
        .switch_to(target_group.id)
        .expect_err("When: malformed config fails");

    assert_eq!(error, SwitchError::LoadOhMyConfigFailed);
    assert_eq!(
        fs::read_to_string(resolved.oh_my_openagent_file()).expect("Then: malformed reads"),
        "{ invalid json"
    );

    remove_temp(&home);
}

#[test]
fn switching_when_opencode_write_succeeds_but_oh_my_write_fails_rolls_back_opencode() {
    let home = temp_home("rollback");
    seed_oh_my(&home);
    seed_opencode(&home);
    let target_group = group(
        "cccccccc-cccc-cccc-cccc-cccccccccccc",
        "Rollback",
        true,
        vec![category_row("quick", "cliproxyapi/gpt-5.4")],
        vec![],
        vec![switching_support::override_row(
            "creative-ui-coder",
            "cliproxyapi/gpt-5.4-xhigh",
        )],
    );
    seed_groups(&home, &[target_group.clone()]);
    let resolved = paths(&home);
    let original_opencode = fs::read(resolved.opencode_file()).expect("Given: OpenCode reads");
    let original_oh_my = fs::read(resolved.oh_my_openagent_file()).expect("Given: Oh My reads");
    let mut failing_use_case = use_case(&home);
    failing_use_case.fail_next_oh_my_save_for_test();

    let error = failing_use_case
        .switch_to(target_group.id)
        .expect_err("When: second write fails");

    assert!(matches!(error, SwitchError::TransactionFailed { .. }));
    assert_eq!(
        fs::read(resolved.opencode_file()).expect("Then: OpenCode reads"),
        original_opencode
    );
    assert_eq!(
        fs::read(resolved.oh_my_openagent_file()).expect("Then: Oh My reads"),
        original_oh_my
    );
    assert!(resolved
        .omo_switch_dir()
        .join("backups")
        .join("opencode")
        .exists());
    assert!(resolved
        .omo_switch_dir()
        .join("backups")
        .join("oh-my-openagent")
        .exists());

    remove_temp(&home);
}

#[test]
fn switching_when_effective_opencode_overrides_exist_writes_both_targets_and_preserves_opencode_unrelated_keys(
) {
    let home = temp_home("dual-target");
    seed_oh_my(&home);
    seed_opencode(&home);
    let target_group = group(
        "99999999-9999-9999-9999-999999999999",
        "Dual Target",
        true,
        vec![category_row(
            "unspecified-high",
            "cliproxyapi/gpt-5.4-xhigh",
        )],
        vec![switching_support::override_row(
            "oracle",
            "cliproxyapi/gpt-5.4-xhigh",
        )],
        vec![
            switching_support::override_row("creative-ui-coder", "cliproxyapi/gpt-5.4"),
            switching_support::override_row("Jenny", "cliproxyapi/gpt-5.4-xhigh"),
        ],
    );
    seed_groups(&home, &[target_group.clone()]);

    let outcome = use_case(&home)
        .switch_to(target_group.id)
        .expect("When: switch succeeds");

    assert_dual_target_success(&home, &target_group, outcome);
    remove_temp(&home);
}
