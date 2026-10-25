#![allow(dead_code)]

use std::{
    io,
    panic::AssertUnwindSafe,
    process::ExitStatus,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use futures::FutureExt;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::{
    process::{ChildStderr, ChildStdin, ChildStdout, Command},
    sync::{watch, Mutex as TokioMutex, OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessState {
    Starting,
    Active,
    Closing,
    Closed,
    CleanupFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseReason {
    Normal,
    Eof,
    Timeout,
    Cancelled,
    Quit,
    Error(String),
}

impl std::fmt::Display for CloseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Eof => write!(f, "eof"),
            Self::Timeout => write!(f, "timeout"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Quit => write!(f, "quit"),
            Self::Error(e) => write!(f, "error: {e}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub graceful_timeout: Duration,
    pub kill_timeout: Duration,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            graceful_timeout: Duration::from_millis(300),
            kill_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseOutcome {
    pub reason: CloseReason,
    pub exit_status: Option<ExitStatus>,
}

impl CloseOutcome {
    pub fn status(&self) -> Option<ExitStatus> {
        self.exit_status
    }

    pub fn success(&self) -> bool {
        self.exit_status.is_some_and(|s| s.success())
    }
}

#[derive(Debug, Clone)]
pub enum ProcessError {
    BudgetExhausted,
    SpawnFailed(String),
    CleanupFailed(String),
    InvalidStateTransition {
        from: ProcessState,
        to: &'static str,
    },
    Io(String),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExhausted => write!(f, "process owner budget exhausted"),
            Self::SpawnFailed(e) => write!(f, "process spawn failed: {e}"),
            Self::CleanupFailed(e) => write!(f, "process cleanup failed: {e}"),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "invalid process state transition from {from:?} to {to}")
            }
            Self::Io(e) => write!(f, "process io error: {e}"),
        }
    }
}

impl std::error::Error for ProcessError {}

#[derive(Clone, Debug)]
pub struct OwnerBudget {
    semaphore: Arc<Semaphore>,
    limits: Arc<StdMutex<(usize, usize)>>,
    cleanup_failed: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetExhausted;

impl std::fmt::Display for BudgetExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "process owner budget exhausted")
    }
}

impl std::error::Error for BudgetExhausted {}

impl OwnerBudget {
    pub fn new(permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            limits: Arc::new(StdMutex::new((permits, 0))),
            cleanup_failed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn try_acquire(&self) -> Result<OwnerPermit, BudgetExhausted> {
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| BudgetExhausted)?;
        Ok(OwnerPermit::new(permit, self.clone()))
    }

    pub async fn acquire(&self) -> Result<OwnerPermit, BudgetExhausted> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| BudgetExhausted)?;
        Ok(OwnerPermit::new(permit, self.clone()))
    }

    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    pub fn total_permits(&self) -> usize {
        self.limits
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .0
    }

    pub fn cleanup_failed(&self) -> bool {
        self.cleanup_failed.load(Ordering::SeqCst)
    }

    pub(crate) fn record_cleanup_failure(&self) {
        self.cleanup_failed.store(true, Ordering::SeqCst);
    }

    pub async fn wait_idle(&self) {
        let _ = self
            .semaphore
            .acquire_many(self.total_permits() as u32)
            .await;
    }

    pub fn used_permits(&self) -> usize {
        let (total, debt) = *self
            .limits
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (total + debt).saturating_sub(self.available_permits())
    }

    pub fn set_limit(&self, permits: usize) {
        let mut limits = self
            .limits
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (total, debt) = &mut *limits;
        if permits < *total {
            let reduction = *total - permits;
            // Outstanding owners repay the reduction when their permits are released.
            *debt += reduction - self.semaphore.forget_permits(reduction);
        } else {
            let increase = permits - *total;
            let repaid = increase.min(*debt);
            *debt -= repaid;
            self.semaphore.add_permits(increase - repaid);
        }
        *total = permits;
    }
}

#[derive(Debug)]
pub struct OwnerPermit {
    inner: Option<OwnedSemaphorePermit>,
    budget: OwnerBudget,
}

impl OwnerPermit {
    pub(crate) fn budget(&self) -> &OwnerBudget {
        &self.budget
    }

    fn new(permit: OwnedSemaphorePermit, budget: OwnerBudget) -> Self {
        Self {
            inner: Some(permit),
            budget,
        }
    }

    pub fn release(&mut self) {
        if let Some(permit) = self.inner.take() {
            let mut limits = self
                .budget
                .limits
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if limits.1 > 0 {
                limits.1 -= 1;
                permit.forget();
            } else {
                drop(permit);
            }
        }
    }

    pub fn retain_exhausted(&mut self) {
        if let Some(permit) = self.inner.take() {
            permit.forget();
        }
    }

    pub fn is_held(&self) -> bool {
        self.inner.is_some()
    }
}

impl Drop for OwnerPermit {
    fn drop(&mut self) {
        self.release();
    }
}

type CloseReceiver = watch::Receiver<Option<Result<CloseOutcome, ProcessError>>>;

struct ProcessOwnerInner {
    budget: OwnerBudget,
    state: StdMutex<ProcessState>,
    config: ProcessConfig,
    pid: Option<u32>,
    stdin: StdMutex<Option<ChildStdin>>,
    stdout: StdMutex<Option<ChildStdout>>,
    stderr: StdMutex<Option<ChildStderr>>,
    child: TokioMutex<Option<Box<dyn ChildWrapper>>>,
    permit: StdMutex<Option<OwnerPermit>>,
    tasks: StdMutex<Vec<JoinHandle<()>>>,
    close_coordinator: StdMutex<Option<CloseReceiver>>,
    cached_outcome: StdMutex<Option<CloseOutcome>>,
}

#[derive(Clone)]
pub struct ProcessOwner {
    inner: Arc<ProcessOwnerInner>,
}

impl ProcessOwner {
    pub fn spawn(
        mut command: Command,
        permit: OwnerPermit,
        config: ProcessConfig,
    ) -> Result<Self, ProcessError> {
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());

        let mut wrap = CommandWrap::from(command);
        wrap.wrap(KillOnDrop);
        #[cfg(unix)]
        wrap.wrap(process_wrap::tokio::ProcessGroup::leader());
        #[cfg(windows)]
        wrap.wrap(process_wrap::tokio::JobObject);

        let mut child = wrap
            .spawn()
            .map_err(|e| ProcessError::SpawnFailed(e.to_string()))?;
        let pid = child.id();
        let stdin = child.stdin().take();
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();

        let inner = Arc::new(ProcessOwnerInner {
            budget: permit.budget().clone(),
            state: StdMutex::new(ProcessState::Starting),
            config,
            pid,
            stdin: StdMutex::new(stdin),
            stdout: StdMutex::new(stdout),
            stderr: StdMutex::new(stderr),
            child: TokioMutex::new(Some(child)),
            permit: StdMutex::new(Some(permit)),
            tasks: StdMutex::new(Vec::new()),
            close_coordinator: StdMutex::new(None),
            cached_outcome: StdMutex::new(None),
        });

        Ok(Self { inner })
    }

    pub fn activate(&self) -> Result<(), ProcessError> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match *state {
            ProcessState::Starting => {
                *state = ProcessState::Active;
                Ok(())
            }
            ref current => Err(ProcessError::InvalidStateTransition {
                from: current.clone(),
                to: "Active",
            }),
        }
    }

    pub fn state(&self) -> ProcessState {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state(), ProcessState::Active)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.state(), ProcessState::Closed)
    }

    pub fn pid(&self) -> Option<u32> {
        self.inner.pid
    }

    pub fn config(&self) -> &ProcessConfig {
        &self.inner.config
    }

    pub(crate) fn budget(&self) -> &OwnerBudget {
        &self.inner.budget
    }

    pub fn take_stdin(&self) -> Option<ChildStdin> {
        self.inner
            .stdin
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }

    pub fn take_stdout(&self) -> Option<ChildStdout> {
        self.inner
            .stdout
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }

    pub fn take_stderr(&self) -> Option<ChildStderr> {
        self.inner
            .stderr
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }

    pub fn drop_stdin(&self) {
        let mut guard = self
            .inner
            .stdin
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _ = guard.take();
    }

    pub fn attach_task(&self, handle: JoinHandle<()>) {
        let state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if matches!(
            *state,
            ProcessState::Closing | ProcessState::Closed | ProcessState::CleanupFailed(_)
        ) {
            handle.abort();
        } else {
            self.inner
                .tasks
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(handle);
        }
    }

    pub async fn try_wait(&self) -> Result<Option<ExitStatus>, ProcessError> {
        let mut guard = self.inner.child.lock().await;
        if let Some(child) = guard.as_mut() {
            child
                .try_wait()
                .map_err(|e| ProcessError::Io(e.to_string()))
        } else {
            Ok(self
                .inner
                .cached_outcome
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_ref()
                .and_then(|o| o.exit_status))
        }
    }

    pub async fn close(&self, reason: CloseReason) -> Result<CloseOutcome, ProcessError> {
        let mut rx = {
            let mut coord = self
                .inner
                .close_coordinator
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(ref rx) = *coord {
                rx.clone()
            } else {
                let (tx, rx) = watch::channel(None);
                *coord = Some(rx.clone());
                let inner = self.inner.clone();
                tokio::spawn(async move {
                    let outcome = match AssertUnwindSafe(Self::execute_close(&inner, reason))
                        .catch_unwind()
                        .await
                    {
                        Ok(outcome) => outcome,
                        Err(_) => {
                            let message =
                                "close coordinator panicked; process outcome uncertain".to_string();
                            *inner
                                .state
                                .lock()
                                .unwrap_or_else(|error| error.into_inner()) =
                                ProcessState::CleanupFailed(message.clone());
                            if let Some(mut permit) = inner
                                .permit
                                .lock()
                                .unwrap_or_else(|error| error.into_inner())
                                .take()
                            {
                                permit.retain_exhausted();
                            }
                            Err(ProcessError::CleanupFailed(message))
                        }
                    };
                    if let Err(error) = &outcome {
                        inner.budget.record_cleanup_failure();
                        eprintln!("warning: MCP process cleanup failed: {error}");
                    }
                    let _ = tx.send(Some(outcome));
                });
                rx
            }
        };

        loop {
            if let Some(ref res) = *rx.borrow() {
                return res.clone();
            }
            if rx.changed().await.is_err() {
                self.inner.budget.record_cleanup_failure();
                return Err(ProcessError::CleanupFailed(
                    "close coordinator task terminated unexpectedly".into(),
                ));
            }
        }
    }

    async fn execute_close(
        inner: &Arc<ProcessOwnerInner>,
        reason: CloseReason,
    ) -> Result<CloseOutcome, ProcessError> {
        // Step 1: Transition state atomically to Closing.
        {
            let mut state = inner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match *state {
                ProcessState::Closed => {
                    let cached = inner
                        .cached_outcome
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .clone();
                    return Ok(cached.unwrap_or(CloseOutcome {
                        reason,
                        exit_status: None,
                    }));
                }
                ProcessState::CleanupFailed(ref err) => {
                    return Err(ProcessError::CleanupFailed(err.clone()));
                }
                ProcessState::Starting | ProcessState::Active => {
                    *state = ProcessState::Closing;
                }
                ProcessState::Closing => {}
            }
        }

        // Step 2: Drop / shutdown stdin pipe so the child receives EOF if it's reading stdin.
        {
            let mut stdin_guard = inner
                .stdin
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let _ = stdin_guard.take();
        }

        // Step 3: Allow a short graceful window (e.g. configurable or 100-300ms in test) for normal EOF exit.
        let mut exit_status: Option<ExitStatus> = None;
        let mut exited = false;

        {
            let child_wait_fut = async {
                let mut guard = inner.child.lock().await;
                if let Some(child) = guard.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => Ok(status),
                        Ok(None) => child.wait().await,
                        Err(e) => Err(e),
                    }
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "child not found"))
                }
            };

            if let Ok(Ok(status)) =
                tokio::time::timeout(inner.config.graceful_timeout, child_wait_fut).await
            {
                exit_status = Some(status);
                exited = true;
            }
        }

        // Step 4: If not exited, signal process group (SIGTERM then SIGKILL on Unix; JobObject terminate on Windows).
        #[cfg(unix)]
        if !exited {
            if let Some(pgid) = inner.pid.filter(|&p| p > 0) {
                // SAFETY: spawn made this owned child's positive PID its process-group ID;
                // negation targets that group, never the caller's group (PID zero).
                let _ = unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGTERM) };

                let term_grace =
                    std::cmp::min(inner.config.graceful_timeout, Duration::from_millis(150));

                let term_wait_fut = async {
                    let mut guard = inner.child.lock().await;
                    if let Some(child) = guard.as_mut() {
                        match child.try_wait() {
                            Ok(Some(status)) => Ok(status),
                            Ok(None) => child.wait().await,
                            Err(e) => Err(e),
                        }
                    } else {
                        Err(io::Error::new(io::ErrorKind::NotFound, "child not found"))
                    }
                };

                if let Ok(Ok(status)) = tokio::time::timeout(term_grace, term_wait_fut).await {
                    exit_status = Some(status);
                    exited = true;
                } else {
                    // SAFETY: the same owned, nonzero process-group ID used for SIGTERM.
                    let _ = unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGKILL) };
                }
            }
        }

        #[cfg(windows)]
        if !exited {
            let mut guard = inner.child.lock().await;
            if let Some(child) = guard.as_mut() {
                let _ = child.start_kill();
            }
        }

        // Step 5: Await and reap direct child (child.kill().await, child.wait().await).
        // On Unix, ensure any residual grandchildren in the group are signalled even if leader exited first
        #[cfg(unix)]
        if exited {
            if let Some(pgid) = inner.pid.filter(|&p| p > 0) {
                // SAFETY: this nonzero process-group ID belongs to the child spawned above;
                // signalling the group also reaches descendants after its leader exits.
                let _ = unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGTERM) };
                // SAFETY: same owned, nonzero process-group ID as the preceding call.
                let _ = unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGKILL) };
            }
        }

        let mut failures = Vec::new();
        if !exited {
            let reap_fut = async {
                let mut guard = inner.child.lock().await;
                if let Some(child) = guard.as_mut() {
                    let _ = Box::into_pin(child.kill()).await;
                    child.wait().await
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "child not found"))
                }
            };

            match tokio::time::timeout(inner.config.kill_timeout, reap_fut).await {
                Ok(Ok(status)) => {
                    exit_status = Some(status);
                }
                Ok(Err(e)) => {
                    failures.push(format!("error reaping child process: {e}"));
                }
                Err(_) => {
                    failures.push("timeout reaping child process".to_string());
                }
            }
        }

        // Step 6: Join any owned background worker tasks.
        let tasks: Vec<JoinHandle<()>> = {
            let mut guard = inner
                .tasks
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            guard.drain(..).collect()
        };
        for handle in tasks {
            handle.abort();
            if let Err(error) = handle.await {
                if !error.is_cancelled() {
                    failures.push(format!("owned process task failed: {error}"));
                }
            }
        }

        if !failures.is_empty() {
            let message = failures.join("; ");
            *inner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner()) =
                ProcessState::CleanupFailed(message.clone());
            if let Some(mut permit) = inner
                .permit
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take()
            {
                if exit_status.is_some() {
                    permit.release();
                } else {
                    permit.retain_exhausted();
                }
            }
            return Err(ProcessError::CleanupFailed(message));
        }

        // Step 7: Transition to Closed (or CleanupFailed), release permit.
        let outcome = CloseOutcome {
            reason,
            exit_status,
        };

        *inner
            .cached_outcome
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(outcome.clone());
        *inner
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = ProcessState::Closed;

        if let Some(mut permit) = inner
            .permit
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            permit.release();
        }

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn normal_clean_exit_on_stdin_close() {
        let budget = OwnerBudget::new(4);
        let permit = budget.try_acquire().unwrap();
        assert_eq!(budget.available_permits(), 3);

        let cmd = Command::new("cat");
        let config = ProcessConfig {
            graceful_timeout: Duration::from_millis(300),
            kill_timeout: Duration::from_secs(2),
        };
        let owner = ProcessOwner::spawn(cmd, permit, config).unwrap();
        assert_eq!(owner.state(), ProcessState::Starting);
        owner.activate().unwrap();
        assert_eq!(owner.state(), ProcessState::Active);

        let pid = owner.pid().unwrap();
        #[cfg(unix)]
        {
            let ps_before = Command::new("/bin/ps")
                .args(["-p", &pid.to_string(), "-o", "pid="])
                .output()
                .await
                .unwrap();
            assert!(ps_before.status.success());
        }

        let outcome = owner.close(CloseReason::Normal).await.unwrap();
        assert_eq!(outcome.reason, CloseReason::Normal);
        assert!(outcome.success());
        assert_eq!(owner.state(), ProcessState::Closed);
        assert_eq!(budget.available_permits(), 4);

        #[cfg(unix)]
        {
            let ps_after = Command::new("/bin/ps")
                .args(["-p", &pid.to_string(), "-o", "pid="])
                .output()
                .await
                .unwrap();
            assert!(!ps_after.status.success() || ps_after.stdout.is_empty());
        }
    }

    #[tokio::test]
    async fn child_that_hangs_is_killed_and_reaped_tasks_joined() {
        let budget = OwnerBudget::new(2);
        let permit = budget.try_acquire().unwrap();
        let config = ProcessConfig {
            graceful_timeout: Duration::from_millis(150),
            kill_timeout: Duration::from_millis(800),
        };

        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 60"]);

        let owner = ProcessOwner::spawn(cmd, permit, config).unwrap();
        let pid = owner.pid().unwrap();
        owner.activate().unwrap();

        let task_ran = Arc::new(AtomicBool::new(false));
        let task_finished = Arc::new(AtomicBool::new(false));
        let tr = task_ran.clone();
        let tf = task_finished.clone();
        let handle = tokio::spawn(async move {
            tr.store(true, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(60)).await;
            tf.store(true, Ordering::SeqCst);
        });
        owner.attach_task(handle);

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(task_ran.load(Ordering::SeqCst));

        let outcome = owner.close(CloseReason::Timeout).await.unwrap();
        assert_eq!(outcome.reason, CloseReason::Timeout);
        assert_eq!(owner.state(), ProcessState::Closed);
        assert_eq!(budget.available_permits(), 2);

        assert!(!task_finished.load(Ordering::SeqCst));

        #[cfg(unix)]
        {
            let ps = Command::new("/bin/ps")
                .args(["-p", &pid.to_string(), "-o", "pid="])
                .output()
                .await
                .unwrap();
            assert!(!ps.status.success() || ps.stdout.is_empty());
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wrapper_leader_exits_first_descendant_group_killed_and_reaped() {
        let budget = OwnerBudget::new(2);
        let permit = budget.try_acquire().unwrap();
        let config = ProcessConfig {
            graceful_timeout: Duration::from_millis(200),
            kill_timeout: Duration::from_secs(1),
        };

        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 60 & echo $!; exit 0"]);

        let owner = ProcessOwner::spawn(cmd, permit, config).unwrap();
        let stdout = owner.take_stdout().unwrap();
        owner.activate().unwrap();

        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let grandchild_pid: u32 = line.trim().parse().expect("grandchild PID");

        let ps_before = Command::new("/bin/ps")
            .args(["-p", &grandchild_pid.to_string(), "-o", "pid="])
            .output()
            .await
            .unwrap();
        assert!(ps_before.status.success());
        assert_eq!(
            String::from_utf8_lossy(&ps_before.stdout).trim(),
            grandchild_pid.to_string()
        );

        let _ = owner.close(CloseReason::Normal).await.unwrap();
        assert_eq!(owner.state(), ProcessState::Closed);
        assert_eq!(budget.available_permits(), 2);

        tokio::time::sleep(Duration::from_millis(100)).await;
        let ps_after = Command::new("/bin/ps")
            .args(["-p", &grandchild_pid.to_string(), "-o", "pid="])
            .output()
            .await
            .unwrap();
        assert!(!ps_after.status.success() || ps_after.stdout.is_empty());
    }

    #[tokio::test]
    async fn idempotent_repeated_close_concurrent_and_sequential() {
        let budget = OwnerBudget::new(1);
        let permit = budget.try_acquire().unwrap();
        let config = ProcessConfig {
            graceful_timeout: Duration::from_millis(150),
            kill_timeout: Duration::from_secs(1),
        };

        let cmd = Command::new("cat");
        let owner = ProcessOwner::spawn(cmd, permit, config).unwrap();
        owner.activate().unwrap();

        let mut handles = Vec::new();
        for _ in 0..5 {
            let owner_clone = owner.clone();
            handles.push(tokio::spawn(async move {
                owner_clone.close(CloseReason::Quit).await
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap().unwrap());
        }

        let first = &results[0];
        for res in &results[1..] {
            assert_eq!(res.reason, first.reason);
            assert_eq!(res.exit_status, first.exit_status);
        }

        for _ in 0..3 {
            let seq = owner.close(CloseReason::Normal).await.unwrap();
            assert_eq!(seq.reason, first.reason);
            assert_eq!(seq.exit_status, first.exit_status);
        }

        assert_eq!(budget.available_permits(), 1);
        assert_eq!(owner.state(), ProcessState::Closed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unrelated_process_is_not_touched_or_killed() {
        let mut unrelated = Command::new("sleep").arg("30").spawn().unwrap();
        let unrelated_pid = unrelated.id().unwrap();

        let budget = OwnerBudget::new(2);
        let permit = budget.try_acquire().unwrap();
        let config = ProcessConfig {
            graceful_timeout: Duration::from_millis(150),
            kill_timeout: Duration::from_millis(500),
        };

        let cmd = Command::new("cat");
        let owner = ProcessOwner::spawn(cmd, permit, config).unwrap();
        owner.activate().unwrap();

        owner.close(CloseReason::Normal).await.unwrap();

        let ps = Command::new("/bin/ps")
            .args(["-p", &unrelated_pid.to_string(), "-o", "pid="])
            .output()
            .await
            .unwrap();
        assert!(ps.status.success());
        assert_eq!(
            String::from_utf8_lossy(&ps.stdout).trim(),
            unrelated_pid.to_string()
        );

        let _ = unrelated.start_kill();
        let _ = unrelated.wait().await;
    }

    #[tokio::test]
    async fn budget_and_permit_lifecycle_and_cleanup_failed() {
        let budget = OwnerBudget::new(2);
        assert_eq!(budget.available_permits(), 2);

        let permit1 = budget.try_acquire().unwrap();
        let permit2 = budget.try_acquire().unwrap();
        assert_eq!(budget.available_permits(), 0);

        assert!(budget.try_acquire().is_err());

        let cmd = Command::new("cat");
        let owner1 = ProcessOwner::spawn(cmd, permit1, ProcessConfig::default()).unwrap();
        owner1.close(CloseReason::Normal).await.unwrap();
        assert_eq!(budget.available_permits(), 1);

        let mut permit2 = permit2;
        permit2.retain_exhausted();
        drop(permit2);
        assert_eq!(budget.available_permits(), 1);
    }

    #[tokio::test]
    async fn spawn_failure_releases_permit() {
        let budget = OwnerBudget::new(2);
        let permit = budget.try_acquire().unwrap();
        assert_eq!(budget.available_permits(), 1);

        let cmd = Command::new("non_existent_executable_path_12345");
        let res = ProcessOwner::spawn(cmd, permit, ProcessConfig::default());
        assert!(res.is_err());
        assert_eq!(budget.available_permits(), 2);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_reap_still_joins_tasks_and_reports_all_failures() {
        let budget = OwnerBudget::new(1);
        let owner = ProcessOwner::spawn(
            Command::new("cat"),
            budget.try_acquire().unwrap(),
            ProcessConfig {
                graceful_timeout: Duration::ZERO,
                kill_timeout: Duration::from_millis(5),
            },
        )
        .unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        owner.attach_task(tokio::spawn(async move {
            ready_tx.send(()).unwrap();
            panic!("fake owned-task failure");
        }));
        ready_rx.await.unwrap();
        let child_guard = owner.inner.child.lock().await;
        let error = owner
            .close(CloseReason::Quit)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("timeout reaping"), "{error}");
        assert!(error.contains("fake owned-task failure"), "{error}");
        assert!(owner.inner.tasks.lock().unwrap().is_empty());
        assert_eq!(budget.available_permits(), 0);
        drop(child_guard);
        let mut child_guard = owner.inner.child.lock().await;
        let child = child_guard.as_mut().unwrap();
        let _ = child.start_kill();
        child.wait().await.unwrap();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_job_object_lifecycle() {
        let budget = OwnerBudget::new(2);
        let permit = budget.try_acquire().unwrap();
        let mut cmd = Command::new("powershell");
        cmd.args(["-Command", "Start-Sleep -Seconds 60"]);
        let owner = ProcessOwner::spawn(cmd, permit, ProcessConfig::default()).unwrap();
        let outcome = owner.close(CloseReason::Timeout).await.unwrap();
        assert_eq!(owner.state(), ProcessState::Closed);
        assert_eq!(budget.available_permits(), 2);
    }
}
