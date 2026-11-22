use gpui_base::input::{self, InputBaseState, InputModeKind};
use std::rc::Rc;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[path = "editor_menu/fallback.rs"]
mod fallback;
#[cfg(target_os = "macos")]
#[path = "editor_menu/macos.rs"]
mod macos;
#[cfg(target_os = "windows")]
#[path = "editor_menu/windows.rs"]
mod windows;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use fallback::show;
#[cfg(target_os = "macos")]
use macos::show;
#[cfg(target_os = "windows")]
use windows::show;

pub fn init(cx: &mut gpui::App) {
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fallback::init(cx);
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let _ = cx;
}

pub fn install<M: InputModeKind>(editor: &mut InputBaseState<M>) {
    editor.on_context_menu(Rc::new(|menu, capabilities, position, window, cx| {
        let menu = menu
            .menu_with_disabled(
                "Cut",
                !(capabilities.is_editable() && capabilities.is_copyable()),
                Box::new(input::Cut),
            )
            .menu_with_disabled("Copy", !capabilities.is_copyable(), Box::new(input::Copy))
            .menu_with_disabled("Paste", !capabilities.is_editable(), Box::new(input::Paste))
            .separator()
            .menu("Select All", Box::new(input::SelectAll));
        show(menu, position, window, cx);
    }));
}
