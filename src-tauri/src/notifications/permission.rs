#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::{Command, Stdio};

use tauri::AppHandle;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPermission {
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Prompt,
    Granted,
    Denied,
    Unavailable,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    Unsupported,
}

#[cfg(target_os = "macos")]
mod platform {
    use super::NotificationPermission;
    use block2::RcBlock;
    use objc2_foundation::NSError;
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNAuthorizationStatus, UNNotificationSettings,
        UNUserNotificationCenter,
    };
    use tokio::sync::mpsc;

    pub(super) fn map_status(status: UNAuthorizationStatus) -> NotificationPermission {
        match status {
            UNAuthorizationStatus::NotDetermined => NotificationPermission::Prompt,
            UNAuthorizationStatus::Denied => NotificationPermission::Denied,
            UNAuthorizationStatus::Authorized
            | UNAuthorizationStatus::Provisional
            | UNAuthorizationStatus::Ephemeral => NotificationPermission::Granted,
            _ => NotificationPermission::Unavailable,
        }
    }

    pub fn get() -> mpsc::Receiver<NotificationPermission> {
        let (sender, receiver) = mpsc::channel(1);
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let block = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
            // SAFETY: UserNotifications supplies a non-null settings object valid during this callback.
            let settings = unsafe { settings.as_ref() };
            let status = settings.authorizationStatus();
            let _ = sender.try_send(map_status(status));
        });
        center.getNotificationSettingsWithCompletionHandler(&block);
        receiver
    }

    pub fn request() -> mpsc::Receiver<Result<(), super::AppError>> {
        let (sender, receiver) = mpsc::channel(1);
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let block = RcBlock::new(move |_granted: objc2::runtime::Bool, error: *mut NSError| {
            let has_error = !error.is_null();
            let result = if has_error {
                Err(super::AppError::configuration(
                    "Native notification authorization failed",
                ))
            } else {
                Ok(())
            };
            let _ = sender.try_send(result);
        });
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert,
            &block,
        );
        receiver
    }
}

pub async fn get(_app: &AppHandle) -> Result<NotificationPermission, AppError> {
    #[cfg(target_os = "macos")]
    {
        if !super::native::available(_app) {
            return Ok(NotificationPermission::Unavailable);
        }
        let mut receiver = platform::get();
        return tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv())
            .await
            .map_err(|_| AppError::configuration("Notification permission query timed out"))?
            .ok_or_else(|| {
                AppError::configuration("Notification permission callback disconnected")
            });
    }
    #[cfg(target_os = "windows")]
    {
        Ok(windows_permission(_app))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Ok(NotificationPermission::Unsupported)
}

pub async fn request(_app: &AppHandle) -> Result<NotificationPermission, AppError> {
    #[cfg(target_os = "macos")]
    {
        if !super::native::available(_app) {
            return Ok(NotificationPermission::Unavailable);
        }
        let mut receiver = platform::request();
        tokio::time::timeout(std::time::Duration::from_secs(120), receiver.recv())
            .await
            .map_err(|_| AppError::configuration("Notification authorization request timed out"))?
            .ok_or_else(|| {
                AppError::configuration("Notification authorization callback disconnected")
            })??;
        return get(_app).await;
    }
    #[cfg(target_os = "windows")]
    {
        let permission = windows_permission(_app);
        if !matches!(
            permission,
            NotificationPermission::Denied | NotificationPermission::Prompt
        ) {
            return Ok(permission);
        }
        tauri::async_runtime::spawn_blocking(open_settings)
            .await
            .map_err(|_| AppError::configuration("Open notification settings task failed"))??;
        Ok(windows_permission(_app))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Ok(NotificationPermission::Unsupported)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn open_settings() -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("/usr/bin/open");
        command.arg("x-apple.systempreferences:com.apple.Notifications-Settings");
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", "ms-settings:notifications"]);
        command
    };
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| AppError::configuration("Open notification settings failed"))?;
    if status.success() {
        Ok(())
    } else {
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("/usr/bin/open")
                .arg("x-apple.systempreferences:com.apple.preference.notifications")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|_| AppError::configuration("Open notification settings failed"))?;
            if status.success() {
                return Ok(());
            }
        }
        Err(AppError::configuration("Open notification settings failed"))
    }
}

#[cfg(target_os = "windows")]
pub(super) fn windows_permission(app: &AppHandle) -> NotificationPermission {
    use windows::UI::Notifications::ToastNotificationManager;
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6432KEY,
        RRF_SUBKEY_WOW6464KEY,
    };

    // WinRT can report Enabled for an arbitrary, unregistered desktop identity.
    let Ok(executable) = std::env::current_exe() else {
        return NotificationPermission::Unavailable;
    };
    let Some(directory) = executable.parent() else {
        return NotificationPermission::Unavailable;
    };
    let directory_text = directory.to_string_lossy();
    if directory_text.ends_with("\\target\\debug") || directory_text.ends_with("\\target\\release")
    {
        return NotificationPermission::Unavailable;
    }
    let key: Vec<u16> = format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}\0",
        app.config()
            .product_name
            .as_deref()
            .unwrap_or(&app.package_info().name)
    )
    .encode_utf16()
    .collect();
    let value: Vec<u16> = "InstallLocation\0".encode_utf16().collect();
    let registered = [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
        .into_iter()
        .any(|root| {
            [RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY]
                .into_iter()
                .any(|view| {
                    // Windows extended paths are limited to 32,767 UTF-16 code units.
                    let mut buffer = vec![0u16; 32_768];
                    let mut bytes = (buffer.len() * 2) as u32;
                    // SAFETY: names are NUL-terminated; buffer and byte-count storage are valid.
                    let status = unsafe {
                        RegGetValueW(
                            root,
                            key.as_ptr(),
                            value.as_ptr(),
                            RRF_RT_REG_SZ | view,
                            std::ptr::null_mut(),
                            buffer.as_mut_ptr().cast(),
                            &mut bytes,
                        )
                    };
                    if status != 0
                        || bytes < 2
                        || !bytes.is_multiple_of(2)
                        || bytes as usize > buffer.len() * 2
                    {
                        return false;
                    }
                    let units = &buffer[..bytes as usize / 2];
                    let Some((&0, units)) = units.split_last() else {
                        return false;
                    };
                    if units.contains(&0) {
                        return false;
                    }
                    let Ok(location) = String::from_utf16(units) else {
                        return false;
                    };
                    let location = location
                        .strip_prefix('"')
                        .and_then(|s| s.strip_suffix('"'))
                        .unwrap_or(&location);
                    installed_directory_matches(std::path::Path::new(location), directory)
                })
        });
    if !registered {
        return NotificationPermission::Unavailable;
    }
    let setting = ToastNotificationManager::CreateToastNotifierWithId(
        &windows::core::HSTRING::from(&app.config().identifier),
    )
    .and_then(|notifier| notifier.Setting());
    setting
        .map(map_windows_setting)
        .unwrap_or(NotificationPermission::Unavailable)
}

#[cfg(target_os = "windows")]
fn installed_directory_matches(
    installed: &std::path::Path,
    executable_directory: &std::path::Path,
) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Globalization::{CompareStringOrdinal, CSTR_EQUAL};

    if !installed.is_absolute() {
        return false;
    }

    let (Ok(installed), Ok(executable_directory)) = (
        installed.canonicalize(),
        executable_directory.canonicalize(),
    ) else {
        return false;
    };
    let left: Vec<u16> = installed.as_os_str().encode_wide().collect();
    let right: Vec<u16> = executable_directory.as_os_str().encode_wide().collect();
    let (Ok(left_len), Ok(right_len)) = (i32::try_from(left.len()), i32::try_from(right.len()))
    else {
        return false;
    };
    // SAFETY: both UTF-16 buffers remain alive and their explicit lengths are valid.
    unsafe {
        CompareStringOrdinal(left.as_ptr(), left_len, right.as_ptr(), right_len, 1) == CSTR_EQUAL
    }
}

#[cfg(target_os = "windows")]
fn map_windows_setting(
    setting: windows::UI::Notifications::NotificationSetting,
) -> NotificationPermission {
    use windows::UI::Notifications::NotificationSetting;
    match setting {
        NotificationSetting::Enabled => NotificationPermission::Granted,
        NotificationSetting::DisabledForApplication
        | NotificationSetting::DisabledForUser
        | NotificationSetting::DisabledByGroupPolicy => NotificationPermission::Denied,
        NotificationSetting::DisabledByManifest => NotificationPermission::Unsupported,
        _ => NotificationPermission::Unavailable,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn open_settings() -> Result<(), AppError> {
    Err(AppError::configuration(
        "Notification settings are unavailable on this platform",
    ))
}

#[cfg(test)]
#[path = "permission/tests.rs"]
mod tests;
