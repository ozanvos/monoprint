use gpui::{App, Pixels, Point, Window};
use gpui_base::input::{NativeMenu, NativeMenuItem};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    Win32::{
        Foundation::{HWND, POINT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, DestroyMenu, MF_GRAYED, MF_SEPARATOR, MF_STRING,
            SetForegroundWindow, TPM_NONOTIFY, TPM_RETURNCMD, TrackPopupMenuEx,
        },
    },
    core::PCWSTR,
};

pub(super) fn show(menu: NativeMenu, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let Ok(raw) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(raw) = raw.as_raw() else {
        return;
    };
    let hwnd = raw.hwnd.get();
    let scale = window.scale_factor();
    let point = POINT {
        x: (f32::from(position.x) * scale).round() as i32,
        y: (f32::from(position.y) * scale).round() as i32,
    };
    let handle = Window::window_handle(window);
    cx.spawn(async move |cx| {
        let mut point = point;
        let hwnd = HWND(hwnd as *mut _);
        // This is GPUI's foreground thread. The tracking loop runs after the
        // originating event and owns its menu until the user selects or dismisses it.
        let selected = unsafe {
            let Ok(popup) = CreatePopupMenu() else { return };
            for (index, entry) in menu.items.iter().enumerate() {
                match entry {
                    NativeMenuItem::Separator => {
                        let _ = AppendMenuW(popup, MF_SEPARATOR, 0, PCWSTR::null());
                    }
                    NativeMenuItem::Action {
                        label, disabled, ..
                    } => {
                        let label: Vec<u16> = label.encode_utf16().chain([0]).collect();
                        let flags = if *disabled {
                            MF_STRING | MF_GRAYED
                        } else {
                            MF_STRING
                        };
                        let _ = AppendMenuW(popup, flags, index + 1, PCWSTR(label.as_ptr()));
                    }
                }
            }
            let _ = ClientToScreen(hwnd, &mut point);
            let _ = SetForegroundWindow(hwnd);
            let selected = TrackPopupMenuEx(
                popup,
                (TPM_RETURNCMD | TPM_NONOTIFY).0,
                point.x,
                point.y,
                hwnd,
                None,
            )
            .0;
            let _ = DestroyMenu(popup);
            selected
        };
        cx.update(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(NativeMenuItem::Action { action, .. }) = usize::try_from(selected - 1)
                    .ok()
                    .and_then(|index| menu.items.get(index))
                {
                    window.dispatch_action(action.boxed_clone(), cx);
                }
                window.refresh();
            });
        });
    })
    .detach();
}
