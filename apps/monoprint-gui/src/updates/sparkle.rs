//! Sparkle's standard native UI. Loaded only from a direct-download app bundle.
use anyhow::{Context, Result, ensure};
use objc2::{
    MainThreadMarker, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool},
};
use std::{ffi::CString, path::Path};

unsafe extern "C" {
    fn dlopen(path: *const std::ffi::c_char, mode: std::ffi::c_int) -> *mut std::ffi::c_void;
}

pub struct Sparkle(Retained<AnyObject>);

impl Sparkle {
    pub fn new() -> Result<Self> {
        ensure!(
            MainThreadMarker::new().is_some(),
            "Updates require the main thread"
        );
        let executable = std::env::current_exe()?;
        let contents = executable
            .parent()
            .and_then(Path::parent)
            .context("Missing app bundle")?;
        ensure!(
            contents.file_name().is_some_and(|name| name == "Contents"),
            "Updates are available in the packaged app"
        );
        let framework = contents.join("Frameworks/Sparkle.framework/Sparkle");
        ensure!(
            framework.is_file(),
            "Updates are not configured for this build"
        );
        let path = CString::new(framework.as_os_str().as_encoded_bytes())?;
        // SAFETY: The absolute path is inside our signed bundle. Keep the library
        // loaded for the process lifetime: registered Objective-C classes cannot
        // safely be unloaded. All controller calls run on GPUI's main thread.
        unsafe {
            ensure!(
                !dlopen(path.as_ptr(), 2).is_null(),
                "Could not load Sparkle"
            );
            let class = AnyClass::get(c"SPUStandardUpdaterController")
                .context("Missing Sparkle controller")?;
            let allocated: objc2::rc::Allocated<AnyObject> = msg_send![class, alloc];
            let controller: Retained<AnyObject> = msg_send![allocated,
                initWithStartingUpdater: Bool::YES,
                updaterDelegate: None::<&AnyObject>,
                userDriverDelegate: None::<&AnyObject>];
            Ok(Self(controller))
        }
    }

    pub fn check(&self) {
        // SAFETY: Sparkle documents these selectors and the updater is retained.
        unsafe {
            let updater: Retained<AnyObject> = msg_send![&*self.0, updater];
            let allowed: Bool = msg_send![&*updater, canCheckForUpdates];
            if allowed.as_bool() {
                let _: () = msg_send![&*self.0, checkForUpdates: None::<&AnyObject>];
            }
        }
    }

    pub fn automatic(&self) -> bool {
        // SAFETY: SPUUpdater owns the persisted automatic-check preference.
        unsafe {
            let updater: Retained<AnyObject> = msg_send![&*self.0, updater];
            let enabled: Bool = msg_send![&*updater, automaticallyChecksForUpdates];
            enabled.as_bool()
        }
    }

    pub fn set_automatic(&self, enabled: bool) {
        // SAFETY: Documented SPUUpdater setter; invoked only by a user action.
        unsafe {
            let updater: Retained<AnyObject> = msg_send![&*self.0, updater];
            let _: () = msg_send![&*updater, setAutomaticallyChecksForUpdates: Bool::new(enabled)];
        }
    }
}
