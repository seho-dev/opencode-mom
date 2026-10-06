use std::collections::HashMap;

use super::*;

struct Mock {
    active: Uuid,
    values: HashMap<Uuid, [u32; 2]>,
    fail_write: Option<(bool, u32)>,
    fail_refresh: bool,
    fail_read: bool,
    corrupt_apply: bool,
    execution: bool,
    journal_path: PathBuf,
}

impl Backend for Mock {
    fn active_scheme(&mut self) -> Result<Uuid, AppError> {
        Ok(self.active)
    }
    fn read(&mut self, scheme: Uuid, battery: bool) -> Result<u32, AppError> {
        if self.fail_read {
            return Err(AppError::configuration("read failed"));
        }
        Ok(self.values[&scheme][usize::from(battery)])
    }
    fn write(&mut self, scheme: Uuid, battery: bool, value: u32) -> Result<(), AppError> {
        let bytes = fs::read(&self.journal_path).expect("durable journal before first write");
        let journal: Journal = serde_json::from_slice(&bytes).unwrap();
        assert!(journal
            .originals
            .iter()
            .any(|original| original.scheme == scheme));
        if self.fail_write == Some((battery, value)) {
            self.fail_write = None;
            return Err(AppError::configuration("write failed"));
        }
        self.values.get_mut(&scheme).unwrap()[usize::from(battery)] = value;
        if self.corrupt_apply && battery && value == 0 {
            self.values.get_mut(&scheme).unwrap()[1] = 4;
        }
        Ok(())
    }
    fn refresh(&mut self, _: Uuid) -> Result<(), AppError> {
        if self.fail_refresh {
            Err(AppError::configuration("refresh failed"))
        } else {
            Ok(())
        }
    }
    fn execution(&mut self, busy: bool) -> Result<(), AppError> {
        self.execution = busy;
        Ok(())
    }
}

fn policy() -> Policy<Mock> {
    let directory = std::env::temp_dir().join(format!("mom-win-policy-{}", Uuid::new_v4()));
    let active = Uuid::new_v4();
    let backend = Mock {
        active,
        values: HashMap::from([(active, [2, 3])]),
        fail_write: None,
        fail_refresh: false,
        fail_read: false,
        corrupt_apply: false,
        execution: false,
        journal_path: directory.join("windows-lid-journal.json"),
    };
    Policy::new(backend, directory)
}

fn cleanup(policy: &Policy<Mock>) {
    fs::remove_dir_all(policy.path.parent().unwrap()).unwrap();
}

#[test]
fn prepare_does_not_apply_and_restores_exact_ac_dc_values() {
    let mut policy = policy();
    let scheme = policy.backend.active;
    policy.prepare().unwrap();
    assert_eq!(policy.backend.values[&scheme], [2, 3]);
    assert!(!policy.path.exists());
    policy.apply().unwrap();
    assert_eq!(policy.backend.values[&scheme], [0, 0]);
    assert!(policy.backend.execution);
    policy.idle().unwrap();
    assert_eq!(policy.backend.values[&scheme], [2, 3]);
    assert!(!policy.backend.execution);
    assert!(policy.journal.originals.is_empty());
    cleanup(&policy);
}

#[test]
fn partial_apply_rolls_back_and_original_zero_is_not_invented() {
    let mut policy = policy();
    let scheme = policy.backend.active;
    policy.backend.values.insert(scheme, [0, 2]);
    policy.backend.fail_write = Some((true, 0));
    assert!(policy.apply().is_err());
    assert_eq!(policy.backend.values[&scheme], [0, 2]);
    assert!(!policy.backend.execution);
    assert!(policy.journal.originals.is_empty());
    cleanup(&policy);
}

#[test]
fn external_changes_are_preserved_and_not_overwritten_on_renewal() {
    let mut policy = policy();
    let scheme = policy.backend.active;
    policy.apply().unwrap();
    policy.backend.values.get_mut(&scheme).unwrap()[0] = 1;
    assert!(policy.apply().is_err());
    policy.idle().unwrap();
    assert_eq!(policy.backend.values[&scheme], [1, 3]);
    cleanup(&policy);
}

#[test]
fn scheme_switch_restores_previous_before_recording_new_originals() {
    let mut policy = policy();
    let first = policy.backend.active;
    let second = Uuid::new_v4();
    policy.backend.values.insert(second, [3, 1]);
    policy.apply().unwrap();
    policy.backend.active = second;
    policy.apply().unwrap();
    assert_eq!(policy.backend.values[&first], [2, 3]);
    assert_eq!(policy.backend.values[&second], [0, 0]);
    assert_eq!(policy.journal.originals.len(), 1);
    assert_eq!(policy.journal.originals[0].scheme, second);
    policy.idle().unwrap();
    assert_eq!(policy.backend.values[&second], [3, 1]);
    cleanup(&policy);
}

#[test]
fn crash_recovery_and_failed_restore_keep_unresolved_journal() {
    let mut policy = policy();
    let scheme = policy.backend.active;
    policy.apply().unwrap();
    policy.backend.fail_write = Some((false, 2));
    assert!(policy.idle().is_err());
    assert_eq!(policy.backend.values[&scheme], [0, 3]);
    assert_eq!(policy.journal.originals.len(), 1);
    let directory = policy.path.parent().unwrap().to_path_buf();
    let mut recovered = Policy::new(policy.backend, directory);
    recovered.prepare().unwrap();
    assert_eq!(recovered.backend.values[&scheme], [2, 3]);
    assert!(recovered.journal.originals.is_empty());
    cleanup(&recovered);
}

#[test]
fn refresh_readback_failures_and_corrupt_journal_are_fail_closed() {
    let mut policy = policy();
    policy.backend.fail_refresh = true;
    assert!(policy.apply().is_err());
    assert_eq!(policy.journal.originals.len(), 1);
    policy.backend.fail_refresh = false;
    policy.backend.fail_read = true;
    assert!(policy.prepare().is_err());
    assert_eq!(policy.journal.originals.len(), 1);
    policy.backend.fail_read = false;
    policy.prepare().unwrap();
    fs::write(&policy.path, "broken").unwrap();
    let directory = policy.path.parent().unwrap().to_path_buf();
    let mut recovered = Policy::new(policy.backend, directory);
    assert!(recovered.prepare().is_err());
    assert_eq!(fs::read_to_string(&recovered.path).unwrap(), "broken");
    cleanup(&recovered);
}

#[test]
fn apply_readback_mismatch_rolls_back_without_erasing_conflicting_value() {
    let mut policy = policy();
    let scheme = policy.backend.active;
    policy.backend.corrupt_apply = true;
    assert!(policy.apply().is_err());
    assert_eq!(policy.backend.values[&scheme], [2, 4]);
    assert!(!policy.backend.execution);
    assert!(policy.journal.originals.is_empty());
    cleanup(&policy);
}
