pub type Result<T> = std::result::Result<T, &'static str>;

pub trait Backend {
    fn read(&mut self) -> Result<bool>;
    fn write(&mut self, value: bool) -> Result<()>;
    fn journal(&mut self) -> Result<Option<bool>>;
    fn save(&mut self, original: bool) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
}

pub struct Policy<B: Backend> {
    pub backend: B,
    active: bool,
    externally_changed: bool,
}

impl<B: Backend> Policy<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            active: false,
            externally_changed: false,
        }
    }

    pub fn recover(&mut self) -> Result<bool> {
        self.restore()
    }

    pub fn busy(&mut self) -> Result<bool> {
        if self.externally_changed {
            return Err("The power policy was changed outside this app");
        }
        let live = self.backend.read()?;
        if self.active {
            if !live {
                self.externally_changed = true;
                if self.backend.journal()?.is_some() {
                    self.backend.clear()?;
                }
                return Err("The power policy was changed outside this app");
            }
            return Ok(true);
        }
        if live {
            self.active = true;
            return Ok(true);
        }
        // The durable root journal must exist before the first persistent write.
        self.backend.save(false)?;
        let applied = self.backend.write(true).and_then(|_| {
            if self.backend.read()? {
                Ok(true)
            } else {
                Err("The helper could not verify the applied power policy")
            }
        });
        match applied {
            Ok(flag) => {
                self.active = true;
                Ok(flag)
            }
            Err(error) => {
                self.restore()?;
                Err(error)
            }
        }
    }

    pub fn idle(&mut self) -> Result<bool> {
        let live = self.restore()?;
        self.active = false;
        self.externally_changed = false;
        Ok(live)
    }

    fn restore(&mut self) -> Result<bool> {
        let original = self.backend.journal()?;
        let live = self.backend.read()?;
        let Some(original) = original else {
            return Ok(live);
        };
        if !original && live {
            self.backend.write(original)?;
            if self.backend.read()? != original {
                return Err("The helper could not verify restoration; recovery is required");
            }
        }
        // A live value different from the applied value belongs to an external writer.
        self.backend.clear()?;
        Ok(if !original && live { original } else { live })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Mock {
        live: bool,
        snapshot: Option<bool>,
        events: Vec<&'static str>,
        reads: VecDeque<Result<bool>>,
        writes: VecDeque<Result<()>>,
        save_fails: bool,
    }

    impl Backend for Mock {
        fn read(&mut self) -> Result<bool> {
            self.events.push("read");
            self.reads.pop_front().unwrap_or(Ok(self.live))
        }
        fn write(&mut self, value: bool) -> Result<()> {
            assert!(
                self.snapshot.is_some(),
                "Mutation without a durable snapshot"
            );
            self.events.push(if value { "apply" } else { "restore" });
            self.writes.pop_front().unwrap_or(Ok(()))?;
            self.live = value;
            Ok(())
        }
        fn journal(&mut self) -> Result<Option<bool>> {
            Ok(self.snapshot)
        }
        fn save(&mut self, original: bool) -> Result<()> {
            self.events.push("snapshot+fsync");
            if self.save_fails {
                return Err("Snapshot failed");
            }
            self.snapshot = Some(original);
            Ok(())
        }
        fn clear(&mut self) -> Result<()> {
            self.events.push("clear");
            self.snapshot = None;
            Ok(())
        }
    }

    #[test]
    fn prepare_is_recovery_only_and_snapshot_precedes_write() {
        let mut policy = Policy::new(Mock::default());
        assert!(!policy.recover().unwrap());
        assert_eq!(policy.backend.events, ["read"]);
        assert!(policy.busy().unwrap());
        assert_eq!(
            policy.backend.events,
            ["read", "read", "snapshot+fsync", "apply", "read"]
        );
        assert!(!policy.idle().unwrap());
        assert!(policy.backend.snapshot.is_none());
    }

    #[test]
    fn original_one_is_untouched_and_external_changes_are_preserved() {
        let mut policy = Policy::new(Mock {
            live: true,
            ..Mock::default()
        });
        assert!(policy.busy().unwrap());
        assert!(policy.idle().unwrap());
        assert_eq!(policy.backend.events, ["read", "read"]);
        policy.backend.live = false;
        policy.busy().unwrap();
        policy.backend.live = false;
        assert!(policy.busy().is_err());
        assert!(policy.busy().is_err());
        assert!(!policy.idle().unwrap());
        assert!(!policy.backend.events.contains(&"restore"));
    }

    #[test]
    fn snapshot_failure_never_mutates() {
        let mut policy = Policy::new(Mock {
            save_fails: true,
            ..Mock::default()
        });
        assert!(policy.busy().is_err());
        assert_eq!(policy.backend.events, ["read", "snapshot+fsync"]);
    }

    #[test]
    fn apply_failure_and_readback_failure_roll_back() {
        let mut policy = Policy::new(Mock {
            writes: VecDeque::from([Err("Apply failed")]),
            ..Mock::default()
        });
        assert!(policy.busy().is_err());
        assert!(!policy.backend.live);
        assert!(policy.backend.snapshot.is_none());
        let mut policy = Policy::new(Mock {
            reads: VecDeque::from([Ok(false), Err("Readback failed")]),
            ..Mock::default()
        });
        assert!(policy.busy().is_err());
        assert!(!policy.backend.live);
        assert!(policy.backend.events.contains(&"restore"));
        assert!(policy.backend.snapshot.is_none());
    }

    #[test]
    fn failed_restore_retains_journal_for_next_authorized_startup() {
        let mut policy = Policy::new(Mock {
            live: true,
            snapshot: Some(false),
            writes: VecDeque::from([Err("Restore failed")]),
            ..Mock::default()
        });
        assert!(policy.recover().is_err());
        assert_eq!(policy.backend.snapshot, Some(false));
        assert!(!policy.recover().unwrap());
        assert!(policy.backend.snapshot.is_none());
        let mut policy = Policy::new(Mock {
            live: true,
            snapshot: Some(false),
            reads: VecDeque::from([Ok(true), Ok(true)]),
            ..Mock::default()
        });
        assert!(policy.idle().is_err());
        assert_eq!(policy.backend.snapshot, Some(false));
    }

    #[test]
    fn recovery_preserves_a_changed_value() {
        let mut policy = Policy::new(Mock {
            snapshot: Some(false),
            ..Mock::default()
        });
        assert!(!policy.recover().unwrap());
        assert!(policy.backend.snapshot.is_none());
        assert_eq!(policy.backend.events, ["read", "clear"]);
    }
}
