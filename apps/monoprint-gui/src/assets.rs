use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

macro_rules! icons {
    ($($name:ident => $path:literal),* $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum IconName { $($name),* }
        impl IconName {
            pub fn path(self) -> &'static str {
                match self { $(Self::$name => concat!("icons/", $path, ".svg")),* }
            }
        }
        impl AssetSource for Assets {
            fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
                Ok(match path {
                    $(concat!("icons/", $path, ".svg") =>
                        Some(Cow::Borrowed(include_bytes!(concat!("../assets/icons/", $path, ".svg")))),)*
                    _ => None,
                })
            }
            fn list(&self, prefix: &str) -> Result<Vec<SharedString>> {
                Ok([$(concat!("icons/", $path, ".svg")),*].into_iter()
                    .filter(|path| path.starts_with(prefix)).map(Into::into).collect())
            }
        }
    };
}

pub struct Assets;

icons! {
    ArrowLeft => "arrow-left",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronRight => "chevron-right",
    ClipboardPaste => "clipboard-paste",
    Copy => "copy",
    Eraser => "eraser",
    File => "file",
    FolderOpen => "folder-open",
    Image => "image",
    LoaderCircle => "loader-circle",
    Minus => "minus",
    Moon => "moon",
    Plus => "plus",
    RotateCcw => "rotate-ccw",
    Search => "search",
    SlidersHorizontal => "sliders-horizontal",
    SquareTerminal => "square-terminal",
    Sun => "sun",
    TextWrap => "text-wrap",
    X => "x",
}
