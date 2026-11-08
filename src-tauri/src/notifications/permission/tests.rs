//! Table-driven coverage for the production permission mappers.
//!
//! These tests never touch the real `UNUserNotificationCenter` or system settings; they only
//! assert the pure enum mapping that the platform paths delegate to.

use super::NotificationPermission;

/// Asserts the serialized wire value for every permission state. Kept tiny on purpose: the
/// frontend adapter validates against exactly these lowercase strings.
#[test]
fn permission_serializes_as_lowercase_wire_states() {
    let cases = [
        (NotificationPermission::Prompt, "prompt"),
        (NotificationPermission::Granted, "granted"),
        (NotificationPermission::Denied, "denied"),
        (NotificationPermission::Unavailable, "unavailable"),
        (NotificationPermission::Unsupported, "unsupported"),
    ];
    for (permission, wire) in cases {
        assert_eq!(
            serde_json::to_value(permission).unwrap(),
            serde_json::Value::String(wire.to_owned()),
            "unexpected wire value for {permission:?}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_status_maps_every_status_and_falls_back_to_unavailable() {
    use objc2_user_notifications::UNAuthorizationStatus;

    let cases = [
        (
            UNAuthorizationStatus::NotDetermined,
            NotificationPermission::Prompt,
        ),
        (
            UNAuthorizationStatus::Denied,
            NotificationPermission::Denied,
        ),
        (
            UNAuthorizationStatus::Authorized,
            NotificationPermission::Granted,
        ),
        (
            UNAuthorizationStatus::Provisional,
            NotificationPermission::Granted,
        ),
        (
            UNAuthorizationStatus::Ephemeral,
            NotificationPermission::Granted,
        ),
        // Unknown/future statuses must never be reported as granted.
        (
            UNAuthorizationStatus(42),
            NotificationPermission::Unavailable,
        ),
    ];
    for (status, expected) in cases {
        assert_eq!(
            super::platform::map_status(status),
            expected,
            "unexpected mapping for {status:?}"
        );
    }
}

#[cfg(target_os = "windows")]
#[test]
fn windows_setting_maps_every_setting_and_falls_back_to_unavailable() {
    use windows::UI::Notifications::NotificationSetting;

    let cases = [
        (
            NotificationSetting::Enabled,
            NotificationPermission::Granted,
        ),
        (
            NotificationSetting::DisabledForApplication,
            NotificationPermission::Denied,
        ),
        (
            NotificationSetting::DisabledForUser,
            NotificationPermission::Denied,
        ),
        (
            NotificationSetting::DisabledByGroupPolicy,
            NotificationPermission::Denied,
        ),
        (
            NotificationSetting::DisabledByManifest,
            NotificationPermission::Unsupported,
        ),
        // Unknown/future settings must never be reported as granted.
        (NotificationSetting(42), NotificationPermission::Unavailable),
    ];
    for (setting, expected) in cases {
        assert_eq!(
            super::map_windows_setting(setting),
            expected,
            "unexpected mapping for {setting:?}"
        );
    }
}
