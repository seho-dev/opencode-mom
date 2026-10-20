use std::cell::RefCell;
use std::sync::{Mutex, Weak};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{runtime::NSObjectProtocol, sel, MainThreadMarker};
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSAnimationContext, NSMainMenuWindowLevel, NSScreen, NSWindow,
    NSWindowCollectionBehavior, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use super::geometry::{frame_matches, placement, Placement, Rect, Screen};
use super::transition::{duration, verified_frame, Slot, Token, WindowId};
use super::{disabled, lock, IslandService, IslandView};
use crate::error::AppError;

fn failure() -> AppError {
    AppError::configuration("Mini Island window operation failed")
}

fn superseded() -> AppError {
    AppError::validation("Mini Island view transition superseded")
}

struct Request {
    view: IslandView,
    explicit: bool,
    instant: bool,
    target: Option<Placement>,
    sender: Option<tokio::sync::oneshot::Sender<Result<(), AppError>>>,
}

#[derive(Default)]
struct NativeSlot {
    transitions: Slot<Request>,
    window: Option<WindowId>,
}

thread_local! {
    static TRANSITIONS: RefCell<NativeSlot> = RefCell::new(NativeSlot::default());
}

fn native_window(app: &AppHandle) -> Result<Retained<NSWindow>, AppError> {
    let _mtm = MainThreadMarker::new().ok_or_else(failure)?;
    let window = app.get_webview_window("island").ok_or_else(failure)?;
    let pointer = window.ns_window().map_err(|_| failure())?;
    unsafe { Retained::retain(pointer.cast::<NSWindow>()) }.ok_or_else(failure)
}

fn identity(native: &NSWindow) -> WindowId {
    WindowId {
        pointer: native as *const NSWindow as usize,
        number: native.windowNumber(),
    }
}

fn current(app: &AppHandle, token: Token) -> bool {
    let Some(service) = app.try_state::<IslandService>() else {
        return false;
    };
    let (enabled, generation) = {
        let cache = lock(&service.cache);
        (cache.enabled, cache.generation)
    };
    let valid = TRANSITIONS.with(|slot| {
        let slot = slot.borrow();
        slot.window == Some(token.window)
            && slot
                .transitions
                .current(token, enabled, generation, token.window)
    });
    valid
        && native_window(app)
            .map(|native| identity(&native) == token.window)
            .unwrap_or(false)
}

pub(super) fn invalidate(retire: bool) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    let (active, pending, sender) = TRANSITIONS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let entries = slot.transitions.invalidate(retire);
        let sender = slot
            .transitions
            .active
            .as_mut()
            .and_then(|entry| entry.value.sender.take());
        if retire {
            slot.window = None;
        }
        (entries.0, entries.1, sender)
    });
    for sender in active
        .into_iter()
        .chain(pending)
        .filter_map(|entry| entry.value.sender)
        .chain(sender)
    {
        let _ = sender.send(Err(disabled()));
    }
}

pub(super) fn wants_noncompact() -> bool {
    TRANSITIONS.with(|slot| {
        let slot = slot.borrow();
        slot.transitions
            .active
            .iter()
            .chain(slot.transitions.pending.iter())
            .any(|entry| {
                slot.transitions.serial_matches(entry.token)
                    && entry.value.view != IslandView::Compact
            })
    })
}

pub(super) fn request(
    app: &AppHandle,
    view: IslandView,
    generation: u64,
) -> Result<tokio::sync::oneshot::Receiver<Result<(), AppError>>, AppError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let result = enqueue(
        app,
        generation,
        Request {
            view,
            explicit: true,
            instant: false,
            target: None,
            sender: Some(sender),
        },
    );
    if result.is_err() && view == IslandView::Compact {
        let service = app.state::<IslandService>();
        let collapsed = {
            let mut cache = lock(&service.cache);
            if cache.current(generation) {
                cache.collapse();
                true
            } else {
                false
            }
        };
        if collapsed {
            invalidate(false);
            let _ = app.emit_to("island", "island:collapsed", ());
        }
    }
    result?;
    Ok(receiver)
}

fn enqueue(app: &AppHandle, generation: u64, request: Request) -> Result<(), AppError> {
    let service = app.state::<IslandService>();
    if !lock(&service.cache).current(generation) {
        return Err(disabled());
    }
    let native = native_window(app)?;
    let window = identity(&native);
    let compact = request.explicit && request.view == IslandView::Compact;
    let (token, start, displaced) = TRANSITIONS.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.window != Some(window) {
            return Err(disabled());
        }
        Ok(slot.transitions.enqueue(generation, window, request))
    })?;
    if let Some(sender) = displaced.and_then(|entry| entry.value.sender) {
        let _ = sender.send(Err(superseded()));
    }
    if compact {
        {
            let mut cache = lock(&service.cache);
            if cache.current(generation) {
                cache.collapse();
            }
        }
        let _ = app.emit_to("island", "island:collapsed", ());
    }
    if start {
        start_transition(app, token);
    }
    Ok(())
}

fn start_transition(app: &AppHandle, token: Token) {
    if !current(app, token) {
        complete(Some(app), token, Err(superseded()));
        return;
    }
    let Some((view, instant)) = TRANSITIONS.with(|slot| {
        slot.borrow()
            .transitions
            .active
            .as_ref()
            .filter(|entry| entry.token == token)
            .map(|entry| (entry.value.view, entry.value.instant))
    }) else {
        return;
    };
    let prepared = (|| {
        let selected = screen()?;
        let target = placement(selected, view);
        let native = native_window(app)?;
        policy(&native);
        if !current(app, token) {
            return Err(superseded());
        }
        let before = rect(native.frame());
        let reduced_motion =
            NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        Ok((
            selected,
            target,
            native,
            if instant {
                None
            } else {
                duration(before, target.frame, reduced_motion)
            },
        ))
    })();
    let (selected, target, native, duration) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            complete(Some(app), token, Err(error));
            return;
        }
    };
    TRANSITIONS.with(|slot| {
        if let Some(entry) = slot
            .borrow_mut()
            .transitions
            .active
            .as_mut()
            .filter(|entry| entry.token == token)
        {
            entry.value.target = Some(target);
        }
    });
    if let Some(duration) = duration {
        let notifications = app.state::<IslandService>().notifications.clone();
        let retained = native.clone();
        let completion = RcBlock::new(move || {
            let _keep_alive = &retained;
            animation_completed(&notifications, token, selected, target, view);
        });
        let changes = RcBlock::new(move |context: std::ptr::NonNull<NSAnimationContext>| {
            unsafe { context.as_ref() }.setDuration(duration);
            if current(app, token) {
                native
                    .animator()
                    .setFrame_display(native_rect(target.frame), true);
            }
        });
        // The active token is installed before AppKit: a no-op completion can be synchronous.
        NSAnimationContext::runAnimationGroup_completionHandler(&changes, Some(&completion));
    } else {
        let result = if current(app, token) {
            place_instant(&native, selected, target, view, Some((app, token))).map(|()| target)
        } else {
            Err(superseded())
        };
        complete(Some(app), token, result);
    }
}

fn animation_completed(
    notifications: &Weak<Mutex<Option<AppHandle>>>,
    token: Token,
    selected: Screen,
    target: Placement,
    view: IslandView,
) {
    let app = notifications.upgrade().and_then(|slot| lock(&slot).clone());
    debug_assert!(MainThreadMarker::new().is_some());
    let result = match &app {
        Some(app) if current(app, token) => native_window(app).and_then(|native| {
            correct_and_readback(
                &native,
                selected,
                target,
                view,
                "after-animation",
                Some((app, token)),
            )?;
            Ok(target)
        }),
        _ => Err(superseded()),
    };
    complete(app.as_ref(), token, result);
}

fn complete(app: Option<&AppHandle>, token: Token, mut result: Result<Placement, AppError>) {
    let request = TRANSITIONS.with(|slot| {
        slot.borrow()
            .transitions
            .active
            .as_ref()
            .filter(|entry| entry.token == token)
            .map(|entry| (entry.value.view, entry.value.explicit))
    });
    let Some((view, explicit)) = request else {
        return;
    };
    if let Some(app) = app.filter(|app| current(app, token)) {
        let dirty =
            TRANSITIONS.with(|slot| std::mem::take(&mut slot.borrow_mut().transitions.dirty));
        if result.is_ok() && dirty {
            result = resize_owned(app, view, Some(token));
        }
        if !current(app, token) {
            result = Err(superseded());
        }
        if let Ok(target) = result {
            let service = app.state::<IslandService>();
            let previous = {
                let mut cache = lock(&service.cache);
                if cache.current(token.generation) {
                    let previous = cache.set_view(token.generation, view);
                    cache.notch_width = target.notch_width;
                    cache.top_inset = target.top_inset;
                    previous
                } else {
                    Err(disabled())
                }
            };
            result = previous.as_ref().map(|_| target).map_err(Clone::clone);
            if result.is_ok() && explicit && view == IslandView::Expanded && current(app, token) {
                if let Err(error) = focus(app) {
                    if current(app, token) {
                        if let Ok(previous) = previous {
                            let mut cache = lock(&service.cache);
                            if cache.current(token.generation) {
                                cache.view = previous.0;
                                cache.selected_session = previous.1;
                            }
                        }
                    }
                    result = Err(error);
                }
            }
        }
        if result.is_err() && current(app, token) {
            let mode = lock(&app.state::<IslandService>().cache).view;
            if resize_owned(app, mode, Some(token)).is_err() {
                eprintln!("failed to restore Mini Island native frame");
            }
        }
        if !current(app, token) {
            result = Err(superseded());
        }
    } else {
        result = Err(superseded());
    }
    let Some(entry) = TRANSITIONS.with(|slot| slot.borrow_mut().transitions.release(token)) else {
        return;
    };
    let success = result.is_ok();
    if let Some(sender) = entry.value.sender {
        let _ = sender.send(result.map(|_| ()));
    }
    if success {
        if let Some(app) = app {
            let latest = TRANSITIONS.with(|slot| {
                let slot = slot.borrow();
                slot.window == Some(token.window) && slot.transitions.serial_matches(token)
            });
            let enabled = lock(&app.state::<IslandService>().cache).current(token.generation);
            if latest && enabled {
                changed(app);
            }
        }
    }
    if let Some(app) = app {
        drain(app);
    } else {
        invalidate(true);
    }
}

fn drain(app: &AppHandle) {
    let next = TRANSITIONS.with(|slot| slot.borrow_mut().transitions.next());
    if let Some(token) = next {
        start_transition(app, token);
    } else {
        let dirty = TRANSITIONS.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.transitions.active.is_none() && std::mem::take(&mut slot.transitions.dirty)
        });
        if dirty {
            reconcile(app, false);
        }
    }
}

pub(super) fn reconcile(app: &AppHandle, display_changed: bool) {
    let target = TRANSITIONS.with(|slot| {
        slot.borrow()
            .transitions
            .active
            .as_ref()
            .and_then(|entry| entry.value.target.map(|target| (entry.value.view, target)))
    });
    let display_changed = display_changed
        || target.is_some_and(|(view, target)| {
            screen()
                .map(|screen| placement(screen, view) != target)
                .unwrap_or(false)
        });
    let idle = TRANSITIONS.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.window.is_some() && slot.transitions.geometry_event(display_changed)
    });
    if !idle {
        return;
    }
    let service = app.state::<IslandService>();
    let (generation, view) = {
        let cache = lock(&service.cache);
        if !cache.enabled {
            return;
        }
        (cache.generation, cache.view)
    };
    let _ = enqueue(
        app,
        generation,
        Request {
            view,
            explicit: false,
            instant: true,
            target: None,
            sender: None,
        },
    );
}

fn destroyed(window: WindowId) -> bool {
    if TRANSITIONS.with(|slot| slot.borrow().window == Some(window)) {
        invalidate(true);
        super::hover::remove();
        true
    } else {
        false
    }
}

pub(super) fn event_is_current(window: &tauri::Window) -> bool {
    let Ok(pointer) = window.ns_window() else {
        return false;
    };
    let native = unsafe { &*pointer.cast::<NSWindow>() };
    let window = identity(native);
    TRANSITIONS.with(|slot| slot.borrow().window == Some(window))
}

fn rect(value: NSRect) -> Rect {
    Rect {
        x: value.origin.x,
        y: value.origin.y,
        width: value.size.width,
        height: value.size.height,
    }
}

fn screen() -> Result<Screen, AppError> {
    let mtm = MainThreadMarker::new().ok_or_else(failure)?;
    let screens = NSScreen::screens(mtm);
    let selected = screens
        .iter()
        .find(|screen| {
            screen.respondsToSelector(sel!(safeAreaInsets)) && screen.safeAreaInsets().top > 0.0
        })
        .or_else(|| screens.iter().next())
        .ok_or_else(failure)?;
    let has_safe_area = selected.respondsToSelector(sel!(safeAreaInsets))
        && selected.respondsToSelector(sel!(auxiliaryTopLeftArea))
        && selected.respondsToSelector(sel!(auxiliaryTopRightArea));
    let (top_inset, notch_left, notch_right) = if has_safe_area {
        let left = selected.auxiliaryTopLeftArea();
        let right = selected.auxiliaryTopRightArea();
        (
            selected.safeAreaInsets().top,
            left.origin.x + left.size.width,
            right.origin.x,
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    Ok(Screen {
        frame: rect(selected.frame()),
        visible: rect(selected.visibleFrame()),
        top_inset,
        notch_left,
        notch_right,
    })
}

fn readback(
    native: &NSWindow,
    selected: Screen,
    target: Placement,
    mode: IslandView,
    phase: &str,
) -> Result<(), AppError> {
    let actual = rect(native.frame());
    let actual_top = actual.y + actual.height;
    let screen_top = selected.frame.y + selected.frame.height;
    let requested_top = target.frame.y + target.frame.height;
    let requested_matched = frame_matches(actual, target.frame);
    let diagnostic = format!(
        "Mini Island native readback: phase={phase}, mode={mode:?}, window={}, level={}, requested={:?}, actual={actual:?}, requested.maxY={requested_top}, NSWindow.frame.maxY={actual_top}, NSScreen.frame.maxY={screen_top}, top_gap(screen-actual)={}, top_delta(actual-requested)={}, horizontal_origin_delta(actual-requested)={}, requested_exact={}, requested_matched={requested_matched}",
        native.windowNumber(),
        native.level(),
        target.frame,
        screen_top - actual_top,
        actual_top - requested_top,
        actual.x - target.frame.x,
        actual == target.frame,
    );
    eprintln!("{diagnostic}");
    #[cfg(debug_assertions)]
    if std::fs::write(
        std::env::temp_dir().join("opencode-mom-island-native-frame.log"),
        format!("pid={}, {diagnostic}\n", std::process::id()),
    )
    .is_err()
    {
        eprintln!("failed to write Mini Island native frame diagnostic");
    }
    verified_frame(actual, target.frame)
}

fn open_new_window(
    registered: bool,
    create: impl FnOnce() -> Result<Placement, AppError>,
) -> Result<Placement, AppError> {
    // Destroy is queued; a registered label may still belong to the retiring window.
    if registered {
        return Err(failure());
    }
    create()
}

pub(super) fn open(app: &AppHandle, mode: IslandView) -> Result<Placement, AppError> {
    open_new_window(app.get_webview_window("island").is_some(), || {
        let target = placement(screen()?, mode);
        let window = WebviewWindowBuilder::new(app, "island", WebviewUrl::App("/island".into()))
            .title("MOM Mini Island")
            .inner_size(target.frame.width, target.frame.height)
            .transparent(true)
            .decorations(false)
            .resizable(false)
            .focused(false)
            .accept_first_mouse(true)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .shadow(false)
            .visible(false)
            .background_color(tauri::window::Color(0, 0, 0, 0))
            .build()
            .map_err(|_| failure())?;
        let pointer = window.ns_window().map_err(|_| failure())?;
        let target = resize(app, mode)?;
        // Tauri show() makes macOS windows key; orderFront keeps automatic startup non-activating.
        unsafe {
            let native = &*pointer.cast::<NSWindow>();
            native.orderFront(None);
            readback(native, screen()?, target, mode, "after-orderFront")?;
        }
        super::hover::install(app)?;
        let native = native_window(app)?;
        let identity = identity(&native);
        TRANSITIONS.with(|slot| slot.borrow_mut().window = Some(identity));
        let service = app.state::<IslandService>();
        let cache = std::sync::Arc::downgrade(&service.cache);
        let notifications = service.notifications.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) && destroyed(identity) {
                let app = notifications.upgrade().and_then(|slot| lock(&slot).clone());
                if let Some(service) = app
                    .as_ref()
                    .and_then(|app| app.try_state::<IslandService>())
                {
                    service.shutdown();
                } else if let Some(cache) = cache.upgrade() {
                    lock(&cache).stop();
                }
            }
        });
        Ok(target)
    })
}

pub(super) fn resize(app: &AppHandle, mode: IslandView) -> Result<Placement, AppError> {
    resize_owned(app, mode, None)
}

fn resize_owned(
    app: &AppHandle,
    mode: IslandView,
    token: Option<Token>,
) -> Result<Placement, AppError> {
    ensure_current(token.map(|token| (app, token)))?;
    let _mtm = MainThreadMarker::new().ok_or_else(failure)?;
    let selected = screen()?;
    let target = placement(selected, mode);
    let native = native_window(app)?;
    // Use public AppKit point coordinates directly: Tao mixes display pixels and window DPI.
    policy(&native);
    place_instant(
        &native,
        selected,
        target,
        mode,
        token.map(|token| (app, token)),
    )?;
    Ok(target)
}

fn policy(native: &NSWindow) {
    let level = NSMainMenuWindowLevel + 2;
    if native.level() != level {
        native.setLevel(level);
    }
    let collection = NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle;
    if native.collectionBehavior() != collection {
        native.setCollectionBehavior(collection);
    }
}

fn native_rect(frame: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(frame.x, frame.y),
        NSSize::new(frame.width, frame.height),
    )
}

fn ensure_current(owner: Option<(&AppHandle, Token)>) -> Result<(), AppError> {
    if owner.is_some_and(|(app, token)| !current(app, token)) {
        Err(superseded())
    } else {
        Ok(())
    }
}

fn place_instant(
    native: &NSWindow,
    selected: Screen,
    target: Placement,
    mode: IslandView,
    owner: Option<(&AppHandle, Token)>,
) -> Result<(), AppError> {
    ensure_current(owner)?;
    let frame = native_rect(target.frame);
    let before = native.frame();
    if !frame_matches(rect(before), target.frame) {
        native.setFrame_display(frame, true);
        correct_and_readback(
            native,
            selected,
            target,
            mode,
            "after-setFrameOrigin",
            owner,
        )?;
    }
    Ok(())
}

fn correct_and_readback(
    native: &NSWindow,
    selected: Screen,
    target: Placement,
    mode: IslandView,
    phase: &str,
    owner: Option<(&AppHandle, Token)>,
) -> Result<(), AppError> {
    ensure_current(owner)?;
    let frame = native_rect(target.frame);
    let actual = native.frame();
    if actual.origin != frame.origin && !frame_matches(rect(actual), target.frame) {
        ensure_current(owner)?;
        native.setFrameOrigin(frame.origin);
    }
    ensure_current(owner)?;
    readback(native, selected, target, mode, phase)
}

pub(super) fn focus(app: &AppHandle) -> Result<(), AppError> {
    app.get_webview_window("island")
        .ok_or_else(failure)?
        .set_focus()
        .map_err(|_| failure())
}

pub(super) fn close(app: &AppHandle) {
    invalidate(true);
    super::hover::remove();
    if let Some(window) = app.get_webview_window("island") {
        if window.destroy().is_err() {
            eprintln!("failed to destroy Mini Island window");
        }
    }
}

pub(super) fn changed(app: &AppHandle) {
    if app.emit_to("island", "island:changed", ()).is_err() {
        eprintln!("failed to notify Mini Island window");
    }
}

#[cfg(test)]
mod tests {
    use super::super::Cache;
    use super::*;

    #[test]
    fn delayed_destroy_rejects_reenable_until_label_is_removed() {
        let mut cache = Cache {
            enabled: true,
            ..Default::default()
        };
        let mut native_setup_calls = 0;
        let target = Placement {
            frame: Rect::default(),
            notch_width: 0.0,
            top_inset: 0.0,
        };
        let mut registered = true;
        cache.stop();
        cache.enabled = true;
        cache.connection = Some("connecting");
        let generation = cache.generation;
        assert!(open_new_window(registered, || {
            native_setup_calls += 1;
            Ok(target)
        })
        .is_err());
        assert_eq!(native_setup_calls, 0);
        assert!(cache.window_failed(generation));
        assert!(!cache.enabled);
        assert!(cache.cancel.is_none());
        assert!(cache.api.is_none());
        assert_eq!(cache.connection, Some("error"));

        // Simulate runtime Destroyed removing the label; no automatic retry occurs.
        registered = false;
        assert!(!cache.enabled);
        cache.enabled = true;
        cache.connection = Some("connecting");
        assert_eq!(
            open_new_window(registered, || {
                native_setup_calls += 1;
                Ok(target)
            })
            .unwrap(),
            target
        );
        assert_eq!(native_setup_calls, 1);
        assert!(cache.enabled);
    }
}
