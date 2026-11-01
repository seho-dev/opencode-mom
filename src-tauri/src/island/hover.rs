use std::cell::{Cell, RefCell};
use std::sync::{Mutex, Weak};

use objc2::{
    define_class, msg_send, rc::Retained, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{NSEvent, NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindow};
use objc2_foundation::{NSObject, NSObjectProtocol, NSRect};
use tauri::{AppHandle, Emitter, Manager};

use super::{lock, Cache, IslandService, IslandView};
use crate::error::AppError;

struct HoverIvars {
    notifications: Weak<Mutex<Option<AppHandle>>>,
    cache: Weak<Mutex<Cache>>,
    window_number: isize,
    active: Cell<bool>,
    inside: Cell<bool>,
}

define_class!(
    // SAFETY: NSObject is the actual superclass, all ivars are initialized before
    // init, and MainThreadOnly confines Cell/RefCell state and AppKit to one thread.
    #[unsafe(super = NSObject)]
    #[name = "MOMIslandHoverObserver"]
    #[thread_kind = MainThreadOnly]
    #[ivars = HoverIvars]
    struct HoverObserver;

    // SAFETY: the class inherits NSObject's protocol implementation without overrides.
    unsafe impl NSObjectProtocol for HoverObserver {}

    impl HoverObserver {
        // SAFETY: matches AppKit's mouseEntered: ABI (one borrowed NSEvent, void);
        // AppKit invokes this main-thread-only tracking owner while the event is live.
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.notify(true, false);
        }

        // SAFETY: matches AppKit's mouseExited: ABI and borrows its live NSEvent
        // only during the main-thread callback, without storing it.
        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.notify(false, false);
        }

        // SAFETY: matches AppKit's mouseMoved: ABI and borrows its live NSEvent
        // only during the main-thread callback, without storing it.
        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, _event: &NSEvent) {
            self.notify(true, true);
        }
    }
);

impl HoverObserver {
    fn notify(&self, entered: bool, moved: bool) {
        let ivars = self.ivars();
        if !ivars.active.get() {
            return;
        }
        let Some(cache) = ivars.cache.upgrade() else {
            return;
        };
        let (enabled, view) = {
            let cache = lock(&cache);
            (cache.enabled, cache.view)
        };
        let edge = ivars.inside.replace(entered) != entered;
        if !should_notify(enabled, view, edge, moved) {
            return;
        }
        let Some(notifications) = ivars.notifications.upgrade() else {
            return;
        };
        let Some(app) = lock(&notifications).clone() else {
            return;
        };
        let Some(window) = app.get_webview_window("island") else {
            return;
        };
        let Ok(pointer) = window.ns_window() else {
            return;
        };
        // SAFETY: Tauri owns this NSWindow and MainThreadOnly confines this call;
        // retain accepts null and keeps the object alive across reentrant AppKit calls.
        let Some(native) = (unsafe { Retained::retain(pointer.cast::<NSWindow>()) }) else {
            return;
        };
        if native.windowNumber() != ivars.window_number || !native.isVisible() {
            return;
        }
        #[cfg(debug_assertions)]
        if edge {
            let (running, connection) = {
                let cache = lock(&cache);
                (
                    cache
                        .snapshot
                        .sessions
                        .iter()
                        .filter(|session| session.status == "running")
                        .count(),
                    cache.connection,
                )
            };
            eprintln!(
                "Mini Island pointer: entered={entered}, view={view:?}, key={}, running={running}, connection={connection:?}",
                native.isKeyWindow(),
            );
        }
        let event = if entered {
            "island:pointer-enter"
        } else {
            "island:pointer-leave"
        };
        if app.emit_to("island", event, ()).is_err() {
            eprintln!("failed to notify Mini Island pointer boundary");
        }
    }
}

struct Tracking {
    view: Retained<NSView>,
    area: Retained<NSTrackingArea>,
    owner: Retained<HoverObserver>,
}

impl Drop for Tracking {
    fn drop(&mut self) {
        // NSTrackingArea does not retain its owner; detach before releasing the observer.
        self.owner.ivars().active.set(false);
        self.view.removeTrackingArea(&self.area);
    }
}

thread_local! {
    static TRACKING: RefCell<Option<Tracking>> = const { RefCell::new(None) };
}

fn options() -> NSTrackingAreaOptions {
    NSTrackingAreaOptions::MouseEnteredAndExited
        | NSTrackingAreaOptions::MouseMoved
        | NSTrackingAreaOptions::ActiveAlways
        | NSTrackingAreaOptions::InVisibleRect
}

fn should_notify(enabled: bool, view: IslandView, edge: bool, moved: bool) -> bool {
    enabled && view != IslandView::Expanded && (edge || (moved && view == IslandView::Compact))
}

pub(super) fn install(app: &AppHandle) -> Result<(), AppError> {
    let failure = || AppError::configuration("Mini Island pointer tracking unavailable");
    let mtm = MainThreadMarker::new().ok_or_else(failure)?;
    let window = app.get_webview_window("island").ok_or_else(failure)?;
    let pointer = window.ns_window().map_err(|_| failure())?;
    // SAFETY: Tauri owns this NSWindow and the marker proves main-thread access;
    // retain accepts null and keeps it alive across reentrant AppKit calls below.
    let native = unsafe { Retained::retain(pointer.cast::<NSWindow>()) }.ok_or_else(failure)?;
    let view = native.contentView().ok_or_else(failure)?;
    let service = app.state::<IslandService>();
    let this = HoverObserver::alloc(mtm).set_ivars(HoverIvars {
        notifications: service.notifications.clone(),
        cache: std::sync::Arc::downgrade(&service.cache),
        window_number: native.windowNumber(),
        active: Cell::new(true),
        inside: Cell::new(false),
    });
    // SAFETY: this is a freshly allocated main-thread NSObject subclass with all
    // ivars initialized; inherited init returns the same owned +1 observer type.
    let owner: Retained<HoverObserver> = unsafe { msg_send![super(this), init] };
    // SAFETY: owner implements the tracking selectors with AppKit's ABI; Tracking
    // retains it and the area/view on the main thread, detaching the area before
    // releasing owner because NSTrackingArea itself does not retain that pointer.
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::default(),
            options(),
            Some(&owner),
            None,
        )
    };
    remove();
    view.addTrackingArea(&area);
    #[cfg(debug_assertions)]
    eprintln!(
        "Mini Island tracking: window={}, key={}, mouse_moved={}, ignores_mouse={}, options={:#x}, bounds={:?}",
        native.windowNumber(),
        native.isKeyWindow(),
        native.acceptsMouseMovedEvents(),
        native.ignoresMouseEvents(),
        area.options().bits(),
        view.bounds(),
    );
    TRACKING.with(|slot| *slot.borrow_mut() = Some(Tracking { view, area, owner }));
    Ok(())
}

pub(super) fn remove() {
    if MainThreadMarker::new().is_some() {
        TRACKING.with(|slot| {
            let old = slot.borrow_mut().take();
            drop(old);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_covers_inactive_windows_and_tracks_actual_resized_bounds() {
        let options = options();
        assert!(options.contains(NSTrackingAreaOptions::ActiveAlways));
        assert!(options.contains(NSTrackingAreaOptions::InVisibleRect));
        assert!(options.contains(NSTrackingAreaOptions::MouseEnteredAndExited));
        assert!(options.contains(NSTrackingAreaOptions::MouseMoved));
        assert!(!options.intersects(
            NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::AssumeInside
        ));
    }

    #[test]
    fn notifications_require_live_collapsed_surface_and_recover_missing_entry() {
        for view in [IslandView::Compact, IslandView::Peek, IslandView::Expanded] {
            assert!(!should_notify(false, view, true, true));
        }
        assert!(!should_notify(true, IslandView::Expanded, true, true));
        assert!(should_notify(true, IslandView::Compact, true, false));
        assert!(should_notify(true, IslandView::Peek, true, false));
        assert!(should_notify(true, IslandView::Compact, false, true));
        assert!(!should_notify(true, IslandView::Peek, false, true));
        assert!(!should_notify(true, IslandView::Compact, false, false));
    }
}
