use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;

fn native_error(error: tauri::Error) -> AppError {
    AppError::configuration(format!("native window operation failed: {error}"))
}

fn section_route(section: Option<&str>) -> Result<Option<&'static str>, AppError> {
    match section {
        None => Ok(None),
        Some("settings") => Ok(Some("/settings")),
        Some(_) => Err(AppError::validation("unsupported main window section")),
    }
}

pub(crate) fn open_main(app: &AppHandle, section: Option<&str>) -> Result<(), AppError> {
    let route = section_route(section)?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::not_found("main window not found"))?;
    hide_tray_window(app.clone())?;
    window.unminimize().map_err(native_error)?;
    window.show().map_err(native_error)?;
    window.set_focus().map_err(native_error)?;
    if let Some(route) = route {
        app.emit_to("main", "tray:navigate", route)
            .map_err(native_error)?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_main_window(app: AppHandle, section: Option<String>) -> Result<(), AppError> {
    open_main(&app, section.as_deref())
}

#[tauri::command]
pub fn hide_tray_window(app: AppHandle) -> Result<(), AppError> {
    if let Some(window) = app.get_webview_window("tray") {
        window.hide().map_err(native_error)?;
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if let Some(state) = app.try_state::<native::TrayState>() {
        if let Ok(mut toggle) = state.0.lock() {
            *toggle = geometry::Toggle::default();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) use native::{on_window_event, setup};

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native {
    use std::sync::Mutex;
    use std::time::Instant;

    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::{
        App, LogicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent,
    };

    use super::geometry::{popup_position, Toggle, POPUP_HEIGHT, POPUP_WIDTH};
    use super::*;

    pub(super) struct TrayState(pub(super) Mutex<Toggle>);

    pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
        app.manage(TrayState(Mutex::new(Toggle::default())));
        let popup = WebviewWindowBuilder::new(app, "tray", WebviewUrl::App("/tray".into()))
            .title("opencode-mom")
            .inner_size(POPUP_WIDTH, POPUP_HEIGHT)
            .decorations(false)
            .resizable(false)
            .visible(false)
            .focused(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .shadow(true)
            .background_color(tauri::window::Color(18, 18, 18, 255));
        // macOS is opaque by default; transparent() requires the private API feature.
        #[cfg(target_os = "windows")]
        let popup = popup.transparent(false);
        popup.build()?;

        let open = MenuItem::with_id(app, "tray-open", "Open", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "tray-quit", "Quit", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&open, &quit])?;
        #[cfg(target_os = "macos")]
        let icon = tauri::include_image!("icons/tray-macos.png");
        #[cfg(target_os = "windows")]
        let icon = tauri::include_image!("icons/tray-windows.png");
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let icon = tauri::include_image!("icons/32x32.png");
        TrayIconBuilder::with_id("opencode-mom-tray")
            .icon(icon)
            .icon_as_template(cfg!(target_os = "macos"))
            .tooltip("opencode-mom")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "tray-open" => {
                    if let Err(error) = open_main(app, None) {
                        eprintln!("failed to open main window: {error}");
                    }
                }
                "tray-quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    position,
                    rect,
                    button: MouseButton::Left,
                    button_state,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    let Some(window) = app.get_webview_window("tray") else {
                        return;
                    };
                    let visible = window.is_visible().unwrap_or(false);
                    let state = app.state::<TrayState>();
                    let Ok(mut toggle) = state.0.lock() else {
                        return;
                    };
                    let now = Instant::now();
                    if button_state == MouseButtonState::Down {
                        toggle.press(visible, now);
                        return;
                    }
                    let hide = toggle.release(visible, now);
                    drop(toggle);
                    let result = if hide {
                        window.hide().map_err(native_error)
                    } else {
                        show_popup(app, &window, position, rect)
                    };
                    if let Err(error) = result {
                        eprintln!("failed to toggle tray popup: {error}");
                    }
                }
            })
            .build(app)?;
        Ok(())
    }

    fn show_popup(
        app: &AppHandle,
        window: &WebviewWindow,
        click: tauri::PhysicalPosition<f64>,
        rect: tauri::Rect,
    ) -> Result<(), AppError> {
        let monitor = match app
            .monitor_from_point(click.x, click.y)
            .map_err(native_error)?
        {
            Some(monitor) => monitor,
            None => app
                .primary_monitor()
                .map_err(native_error)?
                .ok_or_else(|| AppError::configuration("no monitor available for tray popup"))?,
        };
        let scale = monitor.scale_factor();
        let (position, size) = popup_position(
            rect,
            click,
            *monitor.work_area(),
            scale,
            cfg!(target_os = "macos"),
        );
        #[cfg(target_os = "macos")]
        {
            // Cocoa converts physical coordinates using the window's previous DPI.
            window
                .set_position(position.to_logical::<f64>(scale))
                .map_err(native_error)?;
            window
                .set_size(LogicalSize::new(POPUP_WIDTH, POPUP_HEIGHT))
                .map_err(native_error)?;
            let _ = size;
        }
        #[cfg(target_os = "windows")]
        {
            window.set_position(position).map_err(native_error)?;
            window.set_size(size).map_err(native_error)?;
        }
        window.show().map_err(native_error)?;
        window.set_focus().map_err(native_error)?;
        app.emit_to("tray", "tray:shown", ()).map_err(native_error)
    }

    pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested { api, .. }
                if matches!(window.label(), "main" | "tray") =>
            {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("failed to hide window: {error}");
                }
            }
            WindowEvent::Focused(false) if window.label() == "tray" => {
                if window.is_visible().unwrap_or(false) {
                    if let Some(state) = window.app_handle().try_state::<TrayState>() {
                        if let Ok(mut toggle) = state.0.lock() {
                            toggle.blur(Instant::now());
                        }
                    }
                    if let Err(error) = window.hide() {
                        eprintln!("failed to hide tray popup on blur: {error}");
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } if window.label() == "tray" => {
                if let Err(error) = window.set_size(LogicalSize::new(POPUP_WIDTH, POPUP_HEIGHT)) {
                    eprintln!("failed to resize tray popup after DPI change: {error}");
                }
            }
            _ => {}
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
mod geometry {
    use std::time::{Duration, Instant};

    use tauri::{LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Rect};

    pub(super) const POPUP_WIDTH: f64 = 340.0;
    pub(super) const POPUP_HEIGHT: f64 = 370.0;
    const BLUR_CLICK_GRACE: Duration = Duration::from_millis(250);

    #[derive(Default)]
    pub(super) struct Toggle {
        pressed_visible: Option<bool>,
        last_blur: Option<Instant>,
    }

    impl Toggle {
        fn recently_blurred(&self, now: Instant) -> bool {
            self.last_blur
                .is_some_and(|blur| now.saturating_duration_since(blur) < BLUR_CLICK_GRACE)
        }

        pub(super) fn blur(&mut self, now: Instant) {
            self.last_blur = Some(now);
        }

        pub(super) fn press(&mut self, visible: bool, now: Instant) {
            self.pressed_visible = Some(visible || self.recently_blurred(now));
        }

        pub(super) fn release(&mut self, visible: bool, now: Instant) -> bool {
            let hide = self
                .pressed_visible
                .take()
                .unwrap_or(visible || self.recently_blurred(now));
            self.last_blur = None;
            hide
        }
    }

    pub(super) fn popup_position(
        rect: Rect,
        click: PhysicalPosition<f64>,
        work: PhysicalRect<i32, u32>,
        scale: f64,
        below_menu_bar: bool,
    ) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
        let mut origin = rect.position.to_physical::<f64>(scale);
        let mut icon = rect.size.to_physical::<f64>(scale);
        if icon.width <= 0.0 || icon.height <= 0.0 {
            origin = click;
            icon = PhysicalSize::new(0.0, 0.0);
        }
        let size = LogicalSize::new(POPUP_WIDTH, POPUP_HEIGHT).to_physical::<u32>(scale);
        let width = f64::from(size.width);
        let height = f64::from(size.height);
        let left = f64::from(work.position.x);
        let top = f64::from(work.position.y);
        let right = left + f64::from(work.size.width);
        let bottom = top + f64::from(work.size.height);
        let center_x = origin.x + icon.width / 2.0;
        let center_y = origin.y + icon.height / 2.0;
        let gap = 6.0 * scale;
        let edge = if below_menu_bar {
            0
        } else {
            [
                (center_y - top).abs(),
                (center_y - bottom).abs(),
                (center_x - left).abs(),
                (center_x - right).abs(),
            ]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(edge, _)| edge)
            .unwrap_or(0)
        };
        let (x, y) = match edge {
            0 => (center_x - width / 2.0, origin.y + icon.height + gap),
            1 => (center_x - width / 2.0, origin.y - height - gap),
            2 => (origin.x + icon.width + gap, center_y - height / 2.0),
            _ => (origin.x - width - gap, center_y - height / 2.0),
        };
        // Keep the fixed panel size; oversized panels pin to the work area's origin.
        let position = PhysicalPosition::new(
            x.round().clamp(left, (right - width).max(left)),
            y.round().clamp(top, (bottom - height).max(top)),
        )
        .cast();
        (position, size)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Rect};

    use super::geometry::{popup_position, Toggle};

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            position: PhysicalPosition::new(x, y).into(),
            size: PhysicalSize::new(width, height).into(),
        }
    }

    #[test]
    fn tray_geometry_handles_monitor_dpi_and_logical_rects() {
        let work = PhysicalRect {
            position: PhysicalPosition::new(-1920, -120),
            size: PhysicalSize::new(1920, 1080),
        };
        let logical = Rect {
            position: LogicalPosition::new(-160.0, -120.0).into(),
            size: LogicalSize::new(24.0, 24.0).into(),
        };
        let (position, size) = popup_position(
            logical,
            PhysicalPosition::new(-180.0, -130.0),
            work,
            1.25,
            true,
        );
        assert_eq!(size, PhysicalSize::new(425, 463));
        assert_eq!(position, PhysicalPosition::new(-425, -113));
        assert_eq!(
            (position, size),
            popup_position(
                rect(-200.0, -150.0, 30.0, 30.0),
                PhysicalPosition::new(-180.0, -130.0),
                work,
                1.25,
                true
            )
        );
    }

    #[test]
    fn tray_geometry_anchors_all_windows_taskbar_edges() {
        let work = PhysicalRect {
            position: PhysicalPosition::new(0, 0),
            size: PhysicalSize::new(1200, 900),
        };
        for (icon, expected) in [
            (rect(590.0, -30.0, 20.0, 20.0), (430, 0)),
            (rect(590.0, 910.0, 20.0, 20.0), (430, 530)),
            (rect(-30.0, 440.0, 20.0, 20.0), (0, 265)),
            (rect(1210.0, 440.0, 20.0, 20.0), (860, 265)),
        ] {
            let (position, _) =
                popup_position(icon, PhysicalPosition::new(600.0, 450.0), work, 1.0, false);
            assert_eq!(position, PhysicalPosition::from(expected));
        }
    }

    #[test]
    fn tray_geometry_handles_empty_rect_and_tiny_work_area() {
        let work = PhysicalRect {
            position: PhysicalPosition::new(-700, 40),
            size: PhysicalSize::new(200, 150),
        };
        let (position, size) = popup_position(
            rect(999.0, 999.0, 0.0, 20.0),
            PhysicalPosition::new(-600.0, 50.0),
            work,
            1.5,
            true,
        );
        assert_eq!(position, work.position);
        assert_eq!(size, PhysicalSize::new(510, 555));
        let work = PhysicalRect {
            position: PhysicalPosition::new(0, 0),
            size: PhysicalSize::new(1200, 900),
        };
        assert_eq!(
            popup_position(
                rect(999.0, 999.0, 0.0, 0.0),
                PhysicalPosition::new(600.0, 20.0),
                work,
                1.0,
                true
            )
            .0,
            PhysicalPosition::new(430, 26)
        );
    }

    #[test]
    fn tray_toggle_does_not_reopen_after_click_blur() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        assert!(!toggle.release(false, now));
        assert!(toggle.release(true, now));
        toggle.press(true, now);
        toggle.blur(now);
        assert!(toggle.release(false, now + Duration::from_millis(20)));
        toggle.blur(now);
        toggle.press(false, now + Duration::from_millis(20));
        assert!(toggle.release(false, now + Duration::from_millis(30)));
        toggle.blur(now);
        assert!(toggle.release(false, now + Duration::from_millis(20)));
        toggle.blur(now);
        assert!(!toggle.release(false, now + Duration::from_millis(251)));
        assert!(!toggle.release(false, now + Duration::from_millis(252)));
    }

    #[test]
    fn tray_navigation_only_allows_settings() {
        assert_eq!(super::section_route(None).unwrap(), None);
        assert_eq!(
            super::section_route(Some("settings")).unwrap(),
            Some("/settings")
        );
        assert!(super::section_route(Some("/settings")).is_err());
        assert!(super::section_route(Some("providers")).is_err());
    }
}
