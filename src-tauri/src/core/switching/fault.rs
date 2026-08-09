use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionFault {
    StageCreate,
    StageWrite,
    StageFlush,
    JournalCreate,
    JournalWrite,
    JournalFlush,
    FirstTargetRename,
    SecondTargetRename,
    StateRename,
    CompensationRename,
    AfterJournalSync,
    AfterFirstTargetRename,
    AfterSecondTargetRename,
    AfterStateRename,
}

impl TransactionFault {
    pub const fn label(self) -> &'static str {
        match self {
            Self::StageCreate => "stage-create",
            Self::StageWrite => "stage-write",
            Self::StageFlush => "stage-flush",
            Self::JournalCreate => "journal-create",
            Self::JournalWrite => "journal-write",
            Self::JournalFlush => "journal-flush",
            Self::FirstTargetRename => "first-target-rename",
            Self::SecondTargetRename => "second-target-rename",
            Self::StateRename => "state-rename",
            Self::CompensationRename => "compensation-rename",
            Self::AfterJournalSync => "after-journal-sync",
            Self::AfterFirstTargetRename => "after-first-target-rename",
            Self::AfterSecondTargetRename => "after-second-target-rename",
            Self::AfterStateRename => "after-state-rename",
        }
    }
}

pub(crate) fn is_interruption(error: &io::Error) -> bool {
    error.to_string().starts_with("after-")
}

pub(crate) fn interruption_stage(error: &io::Error) -> &'static str {
    match error.to_string().as_str() {
        "after-journal-sync" => "after-journal-sync",
        "after-first-target-rename" => "after-first-target-rename",
        "after-second-target-rename" => "after-second-target-rename",
        "after-state-rename" => "after-state-rename",
        _ => "unknown-interruption",
    }
}

#[derive(Debug, Clone, Default)]
pub struct SwitchTestControl {
    faults: Arc<Mutex<VecDeque<TransactionFault>>>,
}

impl SwitchTestControl {
    pub fn fail_once(fault: TransactionFault) -> Self {
        Self::fail_sequence(&[fault])
    }

    pub fn fail_sequence(faults: &[TransactionFault]) -> Self {
        Self {
            faults: Arc::new(Mutex::new(faults.iter().copied().collect())),
        }
    }

    pub(crate) fn inject(&self, stage: TransactionFault) -> io::Result<()> {
        let mut faults = self
            .faults
            .lock()
            .map_err(|_| io::Error::other("transaction fault lock poisoned"))?;
        if faults.iter().position(|fault| *fault == stage) == Some(0) {
            faults.pop_front();
            return Err(io::Error::other(stage.label()));
        }
        Ok(())
    }
}
