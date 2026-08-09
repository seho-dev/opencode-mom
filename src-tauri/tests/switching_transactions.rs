#[path = "common/switching.rs"]
mod switching_support;

use std::fs;
use std::path::Path;
use std::process::Command;

use omo_switch_tauri::core::switching::{
    recover_pending_transaction, SwitchError, SwitchTestControl, TransactionFault,
};
use switching_support::{
    category_row, group, paths, remove_temp, seed_groups, seed_oh_my, seed_opencode, seed_state,
    temp_home, use_case,
};

fn dual_target_group() -> omo_switch_tauri::core::models::ModelGroup {
    group(
        "99999999-9999-9999-9999-999999999999",
        "Dual Target",
        true,
        vec![category_row("quick", "new/oh-my")],
        vec![],
        vec![switching_support::override_row(
            "creative-ui-coder",
            "new/opencode",
        )],
    )
}

fn seeded_home(label: &str) -> std::path::PathBuf {
    let home = temp_home(label);
    seed_oh_my(&home);
    seed_opencode(&home);
    seed_state(&home, None, None);
    seed_groups(&home, &[dual_target_group()]);
    home
}

fn bytes(path: &Path) -> Vec<u8> {
    fs::read(path).expect("Then: file bytes read")
}

fn journal(home: &Path) -> std::path::PathBuf {
    paths(home)
        .omo_switch_dir()
        .join("transaction-journal.json")
}

#[test]
fn transaction_when_each_precommit_stage_fails_preserves_original_three_file_set() {
    for fault in [
        TransactionFault::StageCreate,
        TransactionFault::StageWrite,
        TransactionFault::StageFlush,
        TransactionFault::JournalCreate,
        TransactionFault::JournalWrite,
        TransactionFault::JournalFlush,
        TransactionFault::FirstTargetRename,
    ] {
        let home = seeded_home(fault.label());
        let resolved = paths(&home);
        let originals = [
            bytes(&resolved.opencode_file()),
            bytes(&resolved.oh_my_openagent_file()),
            bytes(&resolved.state_file()),
        ];
        let control = SwitchTestControl::fail_once(fault);

        let error = use_case(&home)
            .with_test_control(control)
            .switch_to(dual_target_group().id)
            .expect_err("When: injected stage fails");

        assert!(matches!(error, SwitchError::TransactionFailed { .. }));
        assert_eq!(bytes(&resolved.opencode_file()), originals[0]);
        assert_eq!(bytes(&resolved.oh_my_openagent_file()), originals[1]);
        assert_eq!(bytes(&resolved.state_file()), originals[2]);
        remove_temp(&home);
    }
}

#[test]
fn transaction_when_second_target_rename_fails_compensates_first_target_and_state() {
    let home = seeded_home("second-rename");
    let resolved = paths(&home);
    let originals = [
        bytes(&resolved.opencode_file()),
        bytes(&resolved.oh_my_openagent_file()),
        bytes(&resolved.state_file()),
    ];

    let error = use_case(&home)
        .with_test_control(SwitchTestControl::fail_once(
            TransactionFault::SecondTargetRename,
        ))
        .switch_to(dual_target_group().id)
        .expect_err("When: second rename fails");

    assert!(matches!(error, SwitchError::TransactionFailed { .. }));
    assert_eq!(bytes(&resolved.opencode_file()), originals[0]);
    assert_eq!(bytes(&resolved.oh_my_openagent_file()), originals[1]);
    assert_eq!(bytes(&resolved.state_file()), originals[2]);
    assert!(!journal(&home).exists());
    remove_temp(&home);
}

#[test]
fn transaction_when_state_rename_fails_compensates_both_targets() {
    let home = seeded_home("state-rename");
    let resolved = paths(&home);
    let originals = [
        bytes(&resolved.opencode_file()),
        bytes(&resolved.oh_my_openagent_file()),
        bytes(&resolved.state_file()),
    ];

    let error = use_case(&home)
        .with_test_control(SwitchTestControl::fail_once(TransactionFault::StateRename))
        .switch_to(dual_target_group().id)
        .expect_err("When: state rename fails");

    assert!(matches!(error, SwitchError::TransactionFailed { .. }));
    assert_eq!(bytes(&resolved.opencode_file()), originals[0]);
    assert_eq!(bytes(&resolved.oh_my_openagent_file()), originals[1]);
    assert_eq!(bytes(&resolved.state_file()), originals[2]);
    remove_temp(&home);
}

#[test]
fn transaction_when_compensation_fails_retains_journal_for_startup_recovery() {
    let home = seeded_home("compensation-failure");
    let error = use_case(&home)
        .with_test_control(SwitchTestControl::fail_sequence(&[
            TransactionFault::SecondTargetRename,
            TransactionFault::CompensationRename,
        ]))
        .switch_to(dual_target_group().id)
        .expect_err("When: compensation fails");

    assert!(
        matches!(error, SwitchError::CompensationFailed { .. }),
        "Then: compensation error is retained, got {error:?}"
    );
    assert!(journal(&home).exists());

    recover_pending_transaction(&paths(&home).omo_switch_dir())
        .expect("When: startup recovery compensates");
    assert!(!journal(&home).exists());
    remove_temp(&home);
}

#[test]
fn transaction_when_interrupted_after_each_commit_phase_recovers_consistent_bytes() {
    for fault in [
        TransactionFault::AfterJournalSync,
        TransactionFault::AfterFirstTargetRename,
        TransactionFault::AfterSecondTargetRename,
        TransactionFault::AfterStateRename,
    ] {
        let home = seeded_home(fault.label());
        let resolved = paths(&home);
        let original_state = bytes(&resolved.state_file());
        let error = use_case(&home)
            .with_test_control(SwitchTestControl::fail_once(fault))
            .switch_to(dual_target_group().id)
            .expect_err("When: forced interruption is injected");
        assert!(
            matches!(error, SwitchError::Interrupted { .. }),
            "Then: interruption is classified, got {error:?}"
        );
        assert!(journal(&home).exists());

        recover_pending_transaction(&resolved.omo_switch_dir())
            .expect("When: startup recovery resolves journal");

        assert!(!journal(&home).exists());
        let state = bytes(&resolved.state_file());
        let committed = state != original_state;
        let opencode = String::from_utf8(bytes(&resolved.opencode_file())).expect("UTF-8");
        let oh_my = String::from_utf8(bytes(&resolved.oh_my_openagent_file())).expect("UTF-8");
        assert_eq!(committed, opencode.contains("new/opencode"));
        assert_eq!(committed, oh_my.contains("new/oh-my"));
        remove_temp(&home);
    }
}

#[test]
fn transaction_when_switch_is_noop_creates_no_backup_stage_or_journal() {
    let home = seeded_home("transaction-noop");
    let target = dual_target_group();
    seed_state(&home, Some(target.id), Some(&target.name));

    use_case(&home)
        .switch_to(target.id)
        .expect("When: no-op succeeds");

    let root = paths(&home).omo_switch_dir();
    assert!(!root.join("backups").exists());
    assert!(!root.join("transaction-journal.json").exists());
    assert!(fs::read_dir(paths(&home).opencode_dir())
        .expect("Then: target directory lists")
        .all(|entry| !entry
            .expect("Then: entry reads")
            .file_name()
            .to_string_lossy()
            .contains(".omo-txn-")));
    remove_temp(&home);
}

#[test]
fn transaction_when_subprocess_aborts_after_first_rename_recovers_on_restart() {
    let home = seeded_home("subprocess-abort");
    let resolved = paths(&home);
    let original_state = bytes(&resolved.state_file());

    let status = Command::new(std::env::current_exe().expect("Given: current test executable"))
        .args(["--exact", "transaction_crash_child", "--nocapture"])
        .env("OMO_SWITCH_CRASH_HOME", &home)
        .status()
        .expect("When: crash child launches");

    assert!(!status.success());
    assert!(journal(&home).exists());
    recover_pending_transaction(&resolved.omo_switch_dir())
        .expect("When: restarted process recovers");
    assert_eq!(bytes(&resolved.state_file()), original_state);
    assert!(!String::from_utf8(bytes(&resolved.opencode_file()))
        .expect("UTF-8")
        .contains("new/opencode"));
    assert!(!String::from_utf8(bytes(&resolved.oh_my_openagent_file()))
        .expect("UTF-8")
        .contains("new/oh-my"));
    assert!(!journal(&home).exists());
    remove_temp(&home);
}

#[test]
fn transaction_crash_child() {
    let Ok(home) = std::env::var("OMO_SWITCH_CRASH_HOME") else {
        return;
    };
    let error = use_case(Path::new(&home))
        .with_test_control(SwitchTestControl::fail_once(
            TransactionFault::AfterFirstTargetRename,
        ))
        .switch_to(dual_target_group().id)
        .expect_err("When: child reaches interruption point");
    assert!(matches!(error, SwitchError::Interrupted { .. }));
    std::process::abort();
}
