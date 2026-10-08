use gpui_kit::App;

/// Supplement Kit's Command/Option bindings with AppKit's Control shortcuts.
pub fn init(cx: &mut App) {
    #[cfg(target_os = "macos")]
    {
        use gpui_kit::{
            KeyBinding,
            base::actions::{SelectDown, SelectLeft, SelectRight, SelectUp},
            component::input::{Backspace, Delete, MoveDown, MoveLeft, MoveRight, MoveUp},
        };

        cx.bind_keys([
            KeyBinding::new("ctrl-b", MoveLeft, Some("Input")),
            KeyBinding::new("ctrl-f", MoveRight, Some("Input")),
            KeyBinding::new("ctrl-p", MoveUp, Some("Input")),
            KeyBinding::new("ctrl-n", MoveDown, Some("Input")),
            KeyBinding::new("ctrl-shift-b", SelectLeft, Some("Input")),
            KeyBinding::new("ctrl-shift-f", SelectRight, Some("Input")),
            KeyBinding::new("ctrl-shift-p", SelectUp, Some("Input")),
            KeyBinding::new("ctrl-shift-n", SelectDown, Some("Input")),
            KeyBinding::new("ctrl-h", Backspace, Some("Input")),
            KeyBinding::new("ctrl-d", Delete, Some("Input")),
        ]);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = cx;
}
