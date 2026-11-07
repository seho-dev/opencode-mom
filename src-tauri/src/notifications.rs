use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;

use tauri::{AppHandle, Manager};
#[cfg(target_os = "windows")]
use tauri_plugin_notification::NotificationExt;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::logging::{record, LogStatus, LogType};
use crate::models::{self, LocalePreference, NotificationSettings};
use crate::opencode::island::{self as api, IslandApi, IslandChange};
use crate::paths::ConfigPaths;

#[cfg(target_os = "macos")]
#[path = "notifications/native.rs"]
mod native;
#[path = "notifications/permission.rs"]
pub(crate) mod permission;

const MAX_EVENT_IDS: usize = 256;

#[derive(Default)]
struct State {
    settings: NotificationSettings,
    locale: LocalePreference,
    generation: u64,
    stopped: bool,
    cancel: CancellationToken,
    task: Option<tauri::async_runtime::JoinHandle<()>>,
}

impl State {
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    fn current(&self, generation: u64) -> bool {
        !self.stopped && self.settings.enabled && self.generation == generation
    }
}

pub(crate) struct NotificationService {
    state: Arc<Mutex<State>>,
    // The run loop owns this handle; the monitor must not retain its own managed service.
    app: Weak<Mutex<Option<AppHandle>>>,
}

pub(crate) struct TargetRefresh<'a>(MutexGuard<'a, State>);

impl TargetRefresh<'_> {
    pub(crate) fn invalidate(mut self) {
        self.0.invalidate();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

impl NotificationService {
    pub(crate) fn settings(&self) -> NotificationSettings {
        lock(&self.state).settings
    }

    pub(crate) fn set_settings(
        &self,
        paths: &ConfigPaths,
        settings: NotificationSettings,
    ) -> Result<NotificationSettings, AppError> {
        {
            let _write = models::config_write_lock();
            let mut config = models::load_config(&paths.config_file())?;
            config.state.notifications = settings;
            models::save_config(&paths.config_file(), &config)?;
            let mut state = lock(&self.state);
            if state.settings.enabled != settings.enabled {
                state.invalidate();
            }
            state.settings = settings;
            state.locale = config.state.preferences.locale;
        }
        self.start_monitor();
        Ok(settings)
    }

    pub(crate) fn update_locale(&self, locale: LocalePreference) {
        lock(&self.state).locale = locale;
    }

    pub(crate) fn prepare_target_refresh(&self) -> TargetRefresh<'_> {
        TargetRefresh(lock(&self.state))
    }

    pub(crate) fn finish_target_refresh(&self) {
        self.start_monitor();
    }

    fn start_monitor(&self) {
        if !cfg!(any(target_os = "macos", target_os = "windows")) {
            return;
        }
        let mut state = lock(&self.state);
        if state.stopped || !state.settings.enabled || state.task.is_some() {
            return;
        }
        state.cancel = CancellationToken::new();
        state.task = Some(tauri::async_runtime::spawn(monitor(
            self.app.clone(),
            self.state.clone(),
            state.generation,
            state.cancel.clone(),
        )));
    }

    pub(crate) fn shutdown(&self) {
        let mut state = lock(&self.state);
        state.stopped = true;
        state.invalidate();
    }
}

impl Drop for NotificationService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Clone, Copy)]
enum Category {
    Completed,
    ApprovalRequired,
    Error,
}

impl Category {
    fn enabled(self, settings: NotificationSettings) -> bool {
        settings.enabled
            && match self {
                Self::Completed => settings.agent_completed,
                Self::ApprovalRequired => settings.approval_required,
                Self::Error => settings.agent_error,
            }
    }

    fn text(self, locale: LocalePreference) -> (&'static str, &'static str) {
        match (self, locale) {
            (Self::Completed, LocalePreference::En) => (
                "Agent completed",
                "An OpenCode agent has completed its task.",
            ),
            (Self::ApprovalRequired, LocalePreference::En) => (
                "Approval required",
                "An OpenCode agent is waiting for your approval. Open OpenCode to review it.",
            ),
            (Self::Error, LocalePreference::En) => (
                "Agent error",
                "An OpenCode agent encountered an error. Open OpenCode to review it.",
            ),
            (Self::Completed, LocalePreference::Zh) => {
                ("Agent 已完成", "OpenCode Agent 已完成任务。")
            }
            (Self::ApprovalRequired, LocalePreference::Zh) => (
                "需要审批",
                "OpenCode Agent 正在等待审批，请打开 OpenCode 查看。",
            ),
            (Self::Error, LocalePreference::Zh) => (
                "Agent 出错",
                "OpenCode Agent 遇到错误，请打开 OpenCode 查看。",
            ),
        }
    }
}

async fn monitor(
    app: Weak<Mutex<Option<AppHandle>>>,
    state: Arc<Mutex<State>>,
    generation: u64,
    cancel: CancellationToken,
) {
    // ponytail: retain 256 IDs across reconnects, not across disable/target changes or app restarts.
    let mut seen = VecDeque::new();
    let mut failed = false;
    let mut failures = 0usize;
    loop {
        let connected = cancel
            .run_until_cancelled(async {
                let api = api::connect_notifications().await?;
                let events = api.events().await?;
                Ok::<_, AppError>((api, events))
            })
            .await;
        let Some(connected) = connected else {
            return;
        };
        if let Ok((api, mut events)) = connected {
            if failed {
                record(LogType::App, LogStatus::Success, "notifications_connected");
                failed = false;
            }
            loop {
                let event = match cancel.run_until_cancelled(events.next()).await {
                    Some(Ok(event)) => event,
                    Some(Err(_)) => break,
                    None => return,
                };
                failures = 0;
                if cancel
                    .run_until_cancelled(handle_event(
                        &app, &state, generation, &cancel, &api, &mut seen, event,
                    ))
                    .await
                    .is_none()
                {
                    return;
                }
            }
        }
        if !lock(&state).current(generation) || cancel.is_cancelled() {
            return;
        }
        if !failed {
            record(
                LogType::App,
                LogStatus::Error,
                "notifications_connection_failed",
            );
            failed = true;
        }
        let seconds = [1, 2, 5, 10][failures.min(3)];
        failures = failures.saturating_add(1);
        if cancel
            .run_until_cancelled(tokio::time::sleep(Duration::from_secs(seconds)))
            .await
            .is_none()
        {
            return;
        }
    }
}

async fn handle_event(
    app: &Weak<Mutex<Option<AppHandle>>>,
    state: &Arc<Mutex<State>>,
    generation: u64,
    cancel: &CancellationToken,
    api: &IslandApi,
    seen: &mut VecDeque<String>,
    event: IslandChange,
) {
    let (event_id, session_id, category) = match event {
        IslandChange::Completed {
            event_id,
            session_id,
        } => (event_id, session_id, Category::Completed),
        IslandChange::ApprovalRequired {
            event_id,
            session_id,
        } => (event_id, session_id, Category::ApprovalRequired),
        IslandChange::Failed {
            event_id,
            session_id,
        } => (event_id, session_id, Category::Error),
        IslandChange::Snapshot | IslandChange::Messages { .. } => return,
    };
    if seen.contains(&event_id) {
        return;
    }
    if seen.len() == MAX_EVENT_IDS {
        seen.pop_front();
    }
    seen.push_back(event_id);
    {
        let state = lock(state);
        if !state.current(generation) || !category.enabled(state.settings) {
            return;
        }
    }
    if matches!(category, Category::Completed) {
        match api.session(&session_id).await {
            Ok(session) if session.parent_id.is_none() => {}
            Ok(_) => return,
            Err(_) => {
                record(
                    LogType::App,
                    LogStatus::Error,
                    "notifications_session_unavailable",
                );
                return;
            }
        }
    }
    let app = app.clone();
    let state = state.clone();
    let cancel = cancel.clone();
    let api = api.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let Some(app) = app.upgrade().and_then(|slot| lock(&slot).clone()) else {
            return;
        };
        let locale = {
            let state = lock(&state);
            if !state.current(generation) || !category.enabled(state.settings) {
                return;
            }
            state.locale
        };
        if cancel.is_cancelled() || api.ensure_current().is_err() {
            return;
        }
        // Dispatch is best-effort: an OS notification already accepted cannot be withdrawn.
        let (title, body) = category.text(locale);
        #[cfg(target_os = "windows")]
        if permission::windows_permission(&app) != permission::NotificationPermission::Granted {
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let native_app = app.clone();
            if app
                .run_on_main_thread(move || {
                    {
                        let state = lock(&state);
                        if !state.current(generation) || !category.enabled(state.settings) {
                            return;
                        }
                    }
                    if cancel.is_cancelled() || api.ensure_current().is_err() {
                        return;
                    }
                    if native::send(&native_app, title, body).is_err() {
                        record(LogType::App, LogStatus::Error, "notifications_show_failed");
                    }
                })
                .is_err()
            {
                record(
                    LogType::App,
                    LogStatus::Error,
                    "notifications_dispatch_failed",
                );
            }
        }
        #[cfg(target_os = "windows")]
        if app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .is_err()
        {
            record(LogType::App, LogStatus::Error, "notifications_show_failed");
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = (app, title, body);
    })
    .await;
    if result.is_err() {
        record(
            LogType::App,
            LogStatus::Error,
            "notifications_dispatch_failed",
        );
    }
}

pub(crate) fn setup(app: &AppHandle, paths: &ConfigPaths, owner: Weak<Mutex<Option<AppHandle>>>) {
    let mut state = State::default();
    match models::load_config(&paths.config_file()) {
        Ok(config) => {
            state.settings = config.state.notifications;
            state.locale = config.state.preferences.locale;
        }
        Err(_) => record(
            LogType::App,
            LogStatus::Error,
            "notifications_preferences_unavailable",
        ),
    }
    app.manage(NotificationService {
        state: Arc::new(Mutex::new(state)),
        app: owner,
    });
    app.state::<NotificationService>().start_monitor();
}

#[cfg(test)]
mod tests;
