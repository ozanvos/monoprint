use gpui_kit::{AssetSource, Result, SharedString};
use std::borrow::Cow;

// Keep the default component bundle and embed only the additional toolbar icon.
gpui_kit::assets::icon_assets!(ToolbarIcons, [ClipboardPaste]);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ToolbarIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ToolbarIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
