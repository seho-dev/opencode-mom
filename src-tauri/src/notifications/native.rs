use block2::{DynBlock, RcBlock};
use objc2::{define_class, msg_send, rc::Retained, runtime::ProtocolObject, AnyThread};
use objc2_foundation::{NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNMutableNotificationContent, UNNotification, UNNotificationPresentationOptions,
    UNNotificationRequest, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use tauri::AppHandle;

use crate::error::AppError;

pub(super) fn available(app: &AppHandle) -> bool {
    let bundle = NSBundle::mainBundle();
    bundle
        .bundleIdentifier()
        .is_some_and(|identifier| identifier.to_string() == app.config().identifier)
        && bundle
            .bundleURL()
            .path()
            .is_some_and(|path| path.to_string().ends_with(".app"))
}

define_class!(
    // SAFETY: NSObject subclass has no ivars or thread-affine state.
    #[unsafe(super(NSObject))]
    #[name = "MOMNotificationPresentationDelegate"]
    struct PresentationDelegate;

    // SAFETY: the class follows NSObject's protocol without overrides.
    unsafe impl NSObjectProtocol for PresentationDelegate {}

    // SAFETY: method signature matches the UserNotifications delegate protocol.
    unsafe impl UNUserNotificationCenterDelegate for PresentationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion
                .call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
        }
    }
);

thread_local! {
    // UNUserNotificationCenter only weakly retains its delegate.
    static DELEGATE: Retained<PresentationDelegate> = {
        // SAFETY: NSObject init initializes this stateless subclass.
        unsafe { msg_send![PresentationDelegate::alloc(), init] }
    };
}

pub(super) fn send(app: &AppHandle, title: &str, body: &str) -> Result<(), AppError> {
    if !available(app) {
        return Err(AppError::configuration(
            "Notification application identity is unavailable",
        ));
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    DELEGATE.with(|delegate| center.setDelegate(Some(ProtocolObject::from_ref(&**delegate))));
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(body));
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
        &NSString::from_str(&uuid::Uuid::new_v4().to_string()),
        &content,
        None,
    );
    let completion = RcBlock::new(|error: *mut NSError| {
        if !error.is_null() {
            crate::logging::record(
                crate::logging::LogType::App,
                crate::logging::LogStatus::Error,
                "notifications_show_failed",
            );
        }
    });
    center.addNotificationRequest_withCompletionHandler(&request, Some(&completion));
    Ok(())
}
