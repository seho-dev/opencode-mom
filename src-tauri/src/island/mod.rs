use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use crate::error::AppError;
use crate::models::{self, AppPreferences};
use crate::opencode_cli::island::{self as api, IslandApi, IslandChange, IslandSnapshot};
use crate::paths::ConfigPaths;

#[cfg(any(target_os = "macos", test))]
mod geometry;
#[cfg(target_os = "macos")]
mod hover;
#[cfg(any(target_os = "macos", test))]
mod transition;
#[cfg(target_os = "macos")]
mod window;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum IslandView {
    #[default]
    Compact,
    Peek,
    Expanded,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandState {
    supported: bool,
    enabled: bool,
    connection: &'static str,
    connection_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    sessions: Vec<api::IslandSession>,
    permissions: Vec<api::IslandPermission>,
    questions: Vec<api::IslandQuestion>,
    preferences: AppPreferences,
    notch_width: f64,
    top_inset: f64,
}

#[derive(Default)]
struct Cache {
    generation: u64,
    enabled: bool,
    view: IslandView,
    selected_session: Option<String>,
    connection: Option<&'static str>,
    error: Option<String>,
    snapshot: IslandSnapshot,
    api: Option<IslandApi>,
    api_generation: u64,
    cancel: Option<watch::Sender<bool>>,
    action_cancel: Option<watch::Sender<bool>>,
    notch_width: f64,
    top_inset: f64,
    revision: u64,
    published: u64,
    preferences: AppPreferences,
}

impl Cache {
    fn current(&self, generation: u64) -> bool {
        self.enabled && self.generation == generation
    }

    fn set_view(
        &mut self,
        generation: u64,
        view: IslandView,
    ) -> Result<(IslandView, Option<String>), AppError> {
        if !self.current(generation) {
            return Err(disabled());
        }
        let previous = (self.view, self.selected_session.clone());
        self.view = view;
        if view != IslandView::Expanded {
            self.selected_session = None;
        }
        Ok(previous)
    }

    fn collapse(&mut self) {
        self.view = IslandView::Compact;
        self.selected_session = None;
    }

    fn invalidate_api(&mut self) {
        self.api = None;
        self.api_generation = self.api_generation.wrapping_add(1);
        if let Some(cancel) = self.action_cancel.take() {
            let _ = cancel.send(true);
        }
    }

    fn start_actions(&mut self) {
        self.invalidate_api();
        self.action_cancel = Some(watch::channel(false).0);
    }

    fn stop(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.enabled = false;
        self.collapse();
        self.connection = None;
        self.error = None;
        self.snapshot = IslandSnapshot::default();
        self.invalidate_api();
        self.notch_width = 0.0;
        self.top_inset = 0.0;
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(true);
        }
    }

    fn publish(&mut self, generation: u64, revision: u64, snapshot: IslandSnapshot) -> bool {
        if !self.current(generation) || revision < self.published {
            return false;
        }
        self.published = revision;
        self.snapshot = snapshot;
        self.connection = Some("connected");
        self.error = None;
        true
    }

    fn fail(&mut self, generation: u64) -> bool {
        if !self.current(generation) {
            return false;
        }
        self.invalidate_api();
        self.snapshot = IslandSnapshot::default();
        self.connection = Some("error");
        self.error = Some("OpenCode connection unavailable; retrying".into());
        true
    }

    fn window_failed(&mut self, generation: u64) -> bool {
        if !self.current(generation) {
            return false;
        }
        self.stop();
        self.connection = Some("error");
        self.error = Some("Mini Island window could not be created".into());
        true
    }

    fn visible_message(&self, session_id: &str) -> bool {
        self.enabled
            && self.view == IslandView::Expanded
            && self.selected_session.as_deref() == Some(session_id)
    }

    fn reconnect(&mut self) -> Result<u64, AppError> {
        if !self.enabled {
            return Err(disabled());
        }
        self.generation = self.generation.wrapping_add(1);
        self.invalidate_api();
        self.snapshot = IslandSnapshot::default();
        self.connection = Some("connecting");
        self.error = None;
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(true);
        }
        Ok(self.generation)
    }
}

#[derive(Default)]
struct Coalescer {
    snapshot: bool,
    message_session: Option<String>,
    deadline: Option<tokio::time::Instant>,
}

impl Coalescer {
    fn record(&mut self, event: IslandChange, cache: &Cache, now: tokio::time::Instant) {
        match event {
            IslandChange::Snapshot => self.snapshot = true,
            IslandChange::Messages { session_id } if cache.visible_message(&session_id) => {
                self.message_session = Some(session_id);
            }
            IslandChange::Messages { .. } => return,
        }
        self.deadline
            .get_or_insert(now + Duration::from_millis(250));
    }

    fn notify(&self, cache: &Cache) -> bool {
        self.snapshot
            || self
                .message_session
                .as_deref()
                .is_some_and(|id| cache.visible_message(id))
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn disabled() -> AppError {
    AppError::validation("Mini Island is disabled or its connection changed")
}

async fn race<A: Future, B: Future>(first: A, second: B) -> Result<A::Output, B::Output> {
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    std::future::poll_fn(|context| {
        if let std::task::Poll::Ready(value) = first.as_mut().poll(context) {
            return std::task::Poll::Ready(Ok(value));
        }
        second.as_mut().poll(context).map(Err)
    })
    .await
}

async fn cancellable<T>(
    cancel: &mut watch::Receiver<bool>,
    future: impl Future<Output = T>,
) -> Result<T, AppError> {
    if *cancel.borrow() {
        return Err(disabled());
    }
    match race(cancel.changed(), future).await {
        Ok(_) => Err(disabled()),
        Err(value) => Ok(value),
    }
}

pub(crate) struct IslandService {
    cache: Arc<Mutex<Cache>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    // ponytail: serialize user mutations globally; use per-session locks only if contention matters.
    mutations: tokio::sync::Mutex<()>,
    notifications: Weak<Mutex<Option<AppHandle>>>,
}

pub(crate) struct TargetRefresh<'a>(MutexGuard<'a, Cache>);

impl TargetRefresh<'_> {
    pub(crate) fn invalidate(mut self) -> Option<u64> {
        if !self.0.enabled {
            return None;
        }
        self.0.selected_session = None;
        self.0.reconnect().ok()
    }
}

impl Default for IslandService {
    fn default() -> Self {
        Self {
            cache: Arc::new(Mutex::new(Cache::default())),
            task: Mutex::new(None),
            mutations: tokio::sync::Mutex::new(()),
            notifications: Weak::new(),
        }
    }
}

impl IslandService {
    #[cfg(test)]
    pub(crate) fn is_enabled(&self) -> bool {
        lock(&self.cache).enabled
    }

    pub(crate) fn state(&self) -> IslandState {
        let cache = lock(&self.cache);
        IslandState {
            supported: cfg!(target_os = "macos"),
            enabled: cache.enabled,
            connection: cache.connection.unwrap_or("disabled"),
            connection_generation: cache.generation,
            error: cache.error.clone(),
            sessions: cache.snapshot.sessions.clone(),
            permissions: cache.snapshot.permissions.clone(),
            questions: cache.snapshot.questions.clone(),
            preferences: cache.preferences,
            notch_width: cache.notch_width,
            top_inset: cache.top_inset,
        }
    }

    pub(crate) fn update_preferences(&self, preferences: AppPreferences) {
        lock(&self.cache).preferences = preferences;
    }

    pub(crate) fn prepare_target_refresh(&self) -> TargetRefresh<'_> {
        // Block old publications until persistence either fails or invalidates this connection.
        TargetRefresh(lock(&self.cache))
    }

    pub(crate) fn finish_target_refresh(
        &self,
        app: &AppHandle,
        generation: Option<u64>,
    ) -> Result<(), AppError> {
        let Some(generation) = generation else {
            return Ok(());
        };
        on_main(app, move |app| {
            let service = app.state::<IslandService>();
            let _transition = crate::opencode_cli::target::transition()?;
            if !lock(&service.cache).current(generation) {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            {
                if app.get_webview_window("island").is_none() {
                    let mut cache = lock(&service.cache);
                    if cache.fail(generation) {
                        cache.error =
                            Some("Mini Island window unavailable; use Refresh to retry".into());
                    }
                    return Err(AppError::configuration(
                        "Mini Island window unavailable; use Refresh to retry",
                    ));
                }
                window::invalidate(false);
                window::reconcile(app, true);
                service.start_monitor(generation);
                changed(app);
            }
            Ok(())
        })
    }

    pub(crate) fn set_enabled(
        &self,
        app: &AppHandle,
        paths: &ConfigPaths,
        enabled: bool,
    ) -> Result<(), AppError> {
        let _transition = crate::opencode_cli::target::transition()?;
        if enabled && !cfg!(target_os = "macos") {
            return Err(AppError::validation(
                "Mini Island is only supported on macOS",
            ));
        }
        {
            let _write = models::config_write_lock();
            let mut config = models::load_config(&paths.config_file())?;
            config.state.island_enabled = enabled;
            models::save_config(&paths.config_file(), &config)?;
            self.update_preferences(config.state.preferences);
        }
        if let Err(error) = self.apply_enabled(app, enabled) {
            if persist_enabled(paths, false).is_err() {
                eprintln!("failed to persist disabled Mini Island after window setup failure");
            }
            return Err(error);
        }
        Ok(())
    }

    fn apply_enabled(&self, app: &AppHandle, enabled: bool) -> Result<(), AppError> {
        let active = lock(&self.cache).enabled;
        if enabled && active && app.get_webview_window("island").is_some() {
            return Ok(());
        }
        self.shutdown();
        #[cfg(target_os = "macos")]
        {
            if !enabled {
                window::close(app);
                return Ok(());
            }
            let (generation, mode) = {
                let mut cache = lock(&self.cache);
                cache.enabled = true;
                cache.connection = Some("connecting");
                (cache.generation, cache.view)
            };
            match window::open(app, mode) {
                Ok(placement) => {
                    let mut cache = lock(&self.cache);
                    if !cache.current(generation) {
                        return Err(disabled());
                    }
                    cache.notch_width = placement.notch_width;
                    cache.top_inset = placement.top_inset;
                }
                Err(error) => {
                    window::close(app);
                    lock(&self.cache).window_failed(generation);
                    eprintln!("Mini Island setup failed; main app remains available");
                    return Err(error);
                }
            }
            self.start_monitor(generation);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, enabled);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn start_monitor(&self, generation: u64) {
        // AppHandle retains managed state; borrow the run loop's handle weakly to avoid a cycle.
        let cache = self.cache.clone();
        let notifications = self.notifications.clone();
        self.replace_monitor(generation, move |receiver| {
            monitor(notifications, cache, generation, receiver)
        });
    }

    #[cfg(any(target_os = "macos", test))]
    fn replace_monitor<F: Future<Output = ()> + Send + 'static>(
        &self,
        generation: u64,
        run: impl FnOnce(watch::Receiver<bool>) -> F,
    ) -> bool {
        let mut task = lock(&self.task);
        let (cancel, receiver) = watch::channel(false);
        {
            let mut cache = lock(&self.cache);
            if !cache.current(generation) {
                return false;
            }
            if let Some(previous) = cache.cancel.replace(cancel) {
                let _ = previous.send(true);
            }
        }
        if let Some(previous) = task.take() {
            previous.abort();
        }
        let next = tauri::async_runtime::spawn(run(receiver));
        {
            let cache = lock(&self.cache);
            if cache.current(generation) {
                *task = Some(next);
                return true;
            }
        }
        next.abort();
        false
    }

    pub(crate) fn shutdown(&self) {
        let (generation, task) = {
            let mut task = lock(&self.task);
            let mut cache = lock(&self.cache);
            cache.stop();
            (cache.generation, task.take())
        };
        if let Some(task) = task {
            task.abort();
        }
        #[cfg(target_os = "macos")]
        {
            window::invalidate(true);
            hover::remove();
            if objc2::MainThreadMarker::new().is_none() {
                if let Some(app) = self
                    .notifications
                    .upgrade()
                    .and_then(|slot| lock(&slot).clone())
                {
                    let cache = Arc::downgrade(&self.cache);
                    let _ = app.run_on_main_thread(move || {
                        let retired = cache.upgrade().map_or(true, |cache| {
                            let cache = lock(&cache);
                            !cache.enabled && cache.generation == generation
                        });
                        if retired {
                            window::invalidate(true);
                            hover::remove();
                        }
                    });
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = generation;
    }

    pub(crate) fn enabled_generation(&self) -> Result<u64, AppError> {
        let cache = lock(&self.cache);
        if !cache.enabled {
            return Err(disabled());
        }
        Ok(cache.generation)
    }

    pub(crate) fn set_view(
        &self,
        app: &AppHandle,
        view: IslandView,
        generation: u64,
    ) -> Result<tokio::sync::oneshot::Receiver<Result<(), AppError>>, AppError> {
        if !lock(&self.cache).current(generation) {
            return Err(disabled());
        }
        #[cfg(target_os = "macos")]
        {
            window::request(app, view, generation)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = app;
            lock(&self.cache).set_view(generation, view)?;
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let _ = sender.send(Ok(()));
            Ok(receiver)
        }
    }

    fn context(
        &self,
        expected_connection_generation: Option<u64>,
    ) -> Result<(u64, u64, IslandApi, watch::Receiver<bool>), AppError> {
        let cache = lock(&self.cache);
        if !cache.enabled
            || expected_connection_generation.is_some_and(|expected| expected != cache.generation)
        {
            return Err(disabled());
        }
        let api = cache
            .api
            .clone()
            .ok_or_else(|| AppError::configuration("Mini Island is not connected"))?;
        let cancel = cache
            .action_cancel
            .as_ref()
            .ok_or_else(disabled)?
            .subscribe();
        Ok((cache.generation, cache.api_generation, api, cancel))
    }

    fn connected_generation(&self, generation: u64, api_generation: u64) -> bool {
        let cache = lock(&self.cache);
        cache.current(generation) && cache.api.is_some() && cache.api_generation == api_generation
    }

    pub(crate) async fn request<T, F, Fut>(
        &self,
        expected_connection_generation: u64,
        action: F,
        mutation: bool,
    ) -> Result<T, AppError>
    where
        F: FnOnce(IslandApi) -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let context = self.context(Some(expected_connection_generation))?;
        self.request_with_context(context, action, mutation).await
    }

    async fn request_with_context<T, F, Fut>(
        &self,
        context: (u64, u64, IslandApi, watch::Receiver<bool>),
        action: F,
        mutation: bool,
    ) -> Result<T, AppError>
    where
        F: FnOnce(IslandApi) -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let mut cancel = context.3;
        let _mutation = if mutation {
            Some(cancellable(&mut cancel, self.mutations.lock()).await?)
        } else {
            None
        };
        if !self.connected_generation(context.0, context.1) || *cancel.borrow() {
            return Err(disabled());
        }
        let result = cancellable(&mut cancel, action(context.2)).await?;
        if !self.connected_generation(context.0, context.1) || *cancel.borrow() {
            return Err(disabled());
        }
        result
    }

    pub(crate) async fn messages(
        &self,
        session_id: String,
        expected_connection_generation: u64,
    ) -> Result<Vec<api::IslandMessage>, AppError> {
        if session_id.len() > 256
            || !session_id.strip_prefix("ses_").is_some_and(|suffix| {
                !suffix.is_empty()
                    && suffix
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            })
        {
            return Err(AppError::validation("Invalid Island session identifier"));
        }
        let context = self.context(Some(expected_connection_generation))?;
        {
            let mut cache = lock(&self.cache);
            if !cache.current(context.0) || cache.api_generation != context.1 {
                return Err(disabled());
            }
            if cache.view == IslandView::Expanded {
                cache.selected_session = Some(session_id.clone());
            }
        }
        self.request_with_context(
            context,
            |api| async move { api.messages(&session_id).await },
            false,
        )
        .await
    }

    fn retry_connection(&self, app: &AppHandle, generation: u64) -> Result<(), AppError> {
        let _transition = crate::opencode_cli::target::transition()?;
        if !lock(&self.cache).current(generation) {
            return Err(disabled());
        }
        #[cfg(target_os = "macos")]
        {
            if app.get_webview_window("island").is_none() {
                let result = self.apply_enabled(app, true);
                if result.is_err() {
                    if let Some(paths) = app.try_state::<ConfigPaths>() {
                        if persist_enabled(&paths, false).is_err() {
                            eprintln!("failed to persist disabled Mini Island after refresh setup failure");
                        }
                    }
                }
                return result;
            }
            let generation = {
                let mut cache = lock(&self.cache);
                if !cache.current(generation) {
                    return Err(disabled());
                }
                cache.reconnect()?
            };
            window::invalidate(false);
            window::reconcile(app, true);
            self.start_monitor(generation);
            changed(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = app;
        Ok(())
    }

    pub(crate) async fn refresh(&self, app: &AppHandle) -> Result<(), AppError> {
        let has_window = app.get_webview_window("island").is_some();
        let retry = {
            let cache = lock(&self.cache);
            if !cache.enabled {
                return Err(disabled());
            }
            (cache.api.is_none() || !has_window).then_some(cache.generation)
        };
        if let Some(generation) = retry {
            return on_main(app, move |app| {
                app.state::<IslandService>()
                    .retry_connection(app, generation)
            });
        }
        let (generation, api_generation, api, mut cancel) = self.context(None)?;
        let revision = next_revision(&self.cache, generation)?;
        let result = cancellable(&mut cancel, api.snapshot()).await?;
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(_) => {
                {
                    let mut cache = lock(&self.cache);
                    if !cache.current(generation)
                        || cache.api_generation != api_generation
                        || *cancel.borrow()
                    {
                        return Err(disabled());
                    }
                    cache.fail(generation);
                }
                return on_main(app, move |app| {
                    app.state::<IslandService>()
                        .retry_connection(app, generation)
                });
            }
        };
        let published = {
            let mut cache = lock(&self.cache);
            if !cache.current(generation)
                || cache.api.is_none()
                || cache.api_generation != api_generation
                || *cancel.borrow()
            {
                return Err(disabled());
            }
            cache.publish(generation, revision, snapshot)
        };
        if published {
            changed(app);
        }
        Ok(())
    }
}

impl Drop for IslandService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn next_revision(cache: &Mutex<Cache>, generation: u64) -> Result<u64, AppError> {
    let mut cache = lock(cache);
    if !cache.current(generation) {
        return Err(disabled());
    }
    cache.revision += 1;
    Ok(cache.revision)
}

fn persist_enabled(paths: &ConfigPaths, enabled: bool) -> Result<(), AppError> {
    let _write = models::config_write_lock();
    let mut config = models::load_config(&paths.config_file())?;
    config.state.island_enabled = enabled;
    models::save_config(&paths.config_file(), &config)
}

fn changed(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let Some(service) = app.try_state::<IslandService>() else {
            return;
        };
        let generation = {
            let cache = lock(&service.cache);
            if !cache.enabled {
                return;
            }
            cache.generation
        };
        let handle = app.clone();
        if app
            .run_on_main_thread(move || {
                let Some(service) = handle.try_state::<IslandService>() else {
                    return;
                };
                let current = lock(&service.cache).current(generation);
                if current && handle.get_webview_window("island").is_some() {
                    window::changed(&handle);
                }
            })
            .is_err()
        {
            eprintln!("failed to dispatch Mini Island window notification");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[cfg(target_os = "macos")]
async fn monitor(
    notifications: Weak<Mutex<Option<AppHandle>>>,
    cache: Arc<Mutex<Cache>>,
    generation: u64,
    mut cancel: watch::Receiver<bool>,
) {
    let delays = [1, 2, 5, 10];
    let mut failures = 0usize;
    loop {
        if *cancel.borrow() || !lock(&cache).current(generation) {
            return;
        }
        let result =
            match cancellable(&mut cancel, connected(&notifications, &cache, generation)).await {
                Ok(result) => result,
                Err(_) => return,
            };
        if !lock(&cache).current(generation) {
            return;
        }
        if result.is_err() {
            let mut state = lock(&cache);
            if !state.current(generation) {
                return;
            }
            if state.connection == Some("connected") {
                failures = 0;
            }
            state.fail(generation);
            drop(state);
            notify(&notifications);
        }
        let delay = delays[failures.min(delays.len() - 1)];
        failures = failures.saturating_add(1);
        if cancellable(&mut cancel, tokio::time::sleep(Duration::from_secs(delay)))
            .await
            .is_err()
        {
            return;
        }
    }
}

#[cfg(target_os = "macos")]
async fn connected(
    notifications: &Weak<Mutex<Option<AppHandle>>>,
    cache: &Mutex<Cache>,
    generation: u64,
) -> Result<(), AppError> {
    let api = api::connect().await?;
    // Subscribe before the snapshot so pending events cannot fall into a startup gap.
    let mut events = api.events().await?;
    let revision = next_revision(cache, generation)?;
    let snapshot = api.snapshot().await?;
    let api_generation = {
        let mut state = lock(cache);
        if !state.publish(generation, revision, snapshot) {
            return Err(disabled());
        }
        state.start_actions();
        state.api = Some(api.clone());
        state.api_generation
    };
    notify(notifications);
    let mut pending = Coalescer::default();
    loop {
        if let Some(wake) = pending.deadline {
            match race(tokio::time::sleep_until(wake), events.next()).await {
                Err(event) => {
                    let state = lock(cache);
                    if !state.current(generation) || state.api_generation != api_generation {
                        return Err(disabled());
                    }
                    pending.record(event?, &state, tokio::time::Instant::now());
                    continue;
                }
                Ok(_) => {}
            }
            if pending.snapshot {
                let revision = next_revision(cache, generation)?;
                let snapshot = api.snapshot().await?;
                let mut state = lock(cache);
                if !state.current(generation)
                    || state.api_generation != api_generation
                    || state.api.is_none()
                {
                    return Err(disabled());
                }
                state.publish(generation, revision, snapshot);
            }
            let notify_visible = {
                let state = lock(cache);
                if !state.current(generation)
                    || state.api_generation != api_generation
                    || state.api.is_none()
                {
                    return Err(disabled());
                }
                pending.notify(&state)
            };
            if notify_visible {
                notify(notifications);
            }
            pending = Coalescer::default();
        }
        let event = events.next().await?;
        let state = lock(cache);
        if !state.current(generation) || state.api_generation != api_generation {
            return Err(disabled());
        }
        pending.record(event, &state, tokio::time::Instant::now());
    }
}

#[cfg(target_os = "macos")]
fn notify(notifications: &Weak<Mutex<Option<AppHandle>>>) {
    let handle = notifications.upgrade().and_then(|slot| lock(&slot).clone());
    if let Some(app) = handle {
        changed(&app);
    }
}

pub(crate) fn setup(
    app: &AppHandle,
    paths: &ConfigPaths,
    notifications: Weak<Mutex<Option<AppHandle>>>,
) {
    let mut service = IslandService::default();
    service.notifications = notifications;
    app.manage(service);
    match models::load_config(&paths.config_file()) {
        Ok(config) => {
            let service = app.state::<IslandService>();
            service.update_preferences(config.state.preferences);
            #[cfg(target_os = "macos")]
            if config.state.island_enabled {
                let _transition = crate::opencode_cli::target::transition();
                if service.apply_enabled(app, true).is_err() {
                    if persist_enabled(paths, false).is_err() {
                        eprintln!("failed to persist disabled Mini Island after startup failure");
                    }
                }
            }
        }
        Err(_) => eprintln!("Mini Island preferences unavailable; feature left disabled"),
    }
}

pub(crate) fn on_main<T: Send + 'static>(
    app: &AppHandle,
    action: impl FnOnce(&AppHandle) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    #[cfg(target_os = "macos")]
    if objc2::MainThreadMarker::new().is_none() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let _ = sender.send(action(&handle));
        })
        .map_err(|_| AppError::configuration("Mini Island native dispatcher unavailable"))?;
        return receiver
            .recv()
            .map_err(|_| AppError::configuration("Mini Island native dispatcher closed"))?;
    }
    action(app)
}

pub(crate) fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() != "island" {
        return;
    }
    #[cfg(target_os = "macos")]
    if matches!(event, tauri::WindowEvent::Destroyed) {
        // The per-window listener captures native identity; a label cannot identify a retired window.
        return;
    }
    #[cfg(target_os = "macos")]
    if !window::event_is_current(window) {
        return;
    }
    let app = window.app_handle();
    let Some(service) = app.try_state::<IslandService>() else {
        return;
    };
    if !lock(&service.cache).enabled {
        return;
    }
    match event {
        tauri::WindowEvent::Focused(false) => {
            #[cfg(target_os = "macos")]
            let pending = window::wants_noncompact();
            #[cfg(not(target_os = "macos"))]
            let pending = false;
            if lock(&service.cache).view != IslandView::Compact || pending {
                if let Ok(generation) = service.enabled_generation() {
                    let _ = service.set_view(app, IslandView::Compact, generation);
                }
            }
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Ok(generation) = service.enabled_generation() {
                let _ = service.set_view(app, IslandView::Compact, generation);
            }
        }
        #[cfg(target_os = "macos")]
        tauri::WindowEvent::ScaleFactorChanged { .. }
        | tauri::WindowEvent::Moved(_)
        | tauri::WindowEvent::Focused(true) => {
            window::reconcile(
                app,
                matches!(event, tauri::WindowEvent::ScaleFactorChanged { .. }),
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
