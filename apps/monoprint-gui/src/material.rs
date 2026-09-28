//! macOS's standard frosted material, underneath GPUI's transparent Metal surface.
//!
//! GPUI's `Blurred` mode uses Selection and removes native material layers. Keep
//! AppKit's background material intact so it supplies blur, tint and contrast.
use anyhow::{Context, Result, bail};
use gpui_kit::Window;
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub fn install(window: &Window) -> Result<()> {
    let main_thread =
        MainThreadMarker::new().context("AppKit material requires the main thread")?;
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("Native window handle is unavailable: {error}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        bail!("Expected an AppKit window");
    };
    // SAFETY: GPUI supplies a live NSView, borrowed for this Window's lifetime.
    // This synchronous call runs on the main thread and retains no raw pointer.
    let gpui_view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    // The effect must be a sibling below the Metal view, not a child covering it.
    // SAFETY: GPUI retains this content container for the live window; the
    // main thread cannot tear down its hierarchy during this synchronous call.
    let container = unsafe { gpui_view.superview() }.context("Missing native content container")?;
    attach(&container, main_thread);
    Ok(())
}

pub(crate) fn attach(host: &NSView, main_thread: MainThreadMarker) -> Retained<NSVisualEffectView> {
    let material =
        NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(main_thread), host.bounds());
    material.setMaterial(NSVisualEffectMaterial::UnderWindowBackground);
    material.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    material.setState(NSVisualEffectState::FollowsWindowActiveState);
    material.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    // The native view hierarchy retains the material after this local handle drops.
    // Insert beneath existing native subviews, leaving GPUI's input handling intact.
    host.addSubview_positioned_relativeTo(&material, NSWindowOrderingMode::Below, None);
    material
}
