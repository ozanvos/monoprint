use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};
use std::{borrow::Cow, sync::Arc};
pub const SURFACE: u32 = 0x1D2025;
pub const CANVAS: u32 = 0x15171B;
pub const BORDER: u32 = 0x30343C;
pub const TEXT: u32 = 0xE5E8ED;
pub const MUTED: u32 = 0x969DA8;
pub const ACCENT: u32 = 0xA6CEFF;

// AppKit supplies the frosting and contrast; only add a light slate tint here.
// Keep other platforms opaque until their native materials are validated.
pub fn chrome_background() -> Rgba {
    rgb(SURFACE).opacity(if cfg!(target_os = "macos") { 0.18 } else { 1.0 })
}

pub fn canvas_background() -> Rgba {
    // This veil sits above the chrome tint, keeping the image's surround quiet.
    rgb(CANVAS).opacity(if cfg!(target_os = "macos") { 0.32 } else { 1.0 })
}

pub fn init(cx: &mut App) {
    cx.set_window_appearance(Some(WindowAppearance::Dark));
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../../../assets/fonts/inter.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/jetbrains-mono.ttf")),
        ])
        .expect("load bundled interface fonts");
    Theme::change(ThemeMode::Dark, None, cx);
    let theme = Theme::global_mut(cx);
    theme.font_family = "Inter".into();
    theme.font_size = px(14.);
    theme.mono_font_family = "JetBrains Mono".into();
    theme.mono_font_size = px(12.);
    theme.radius = px(7.);
    theme.focus_ring = true;
    theme.colors.background = rgb(SURFACE).into();
    theme.colors.foreground = rgb(TEXT).into();
    theme.colors.border = rgb(BORDER).into();
    theme.colors.input = rgb(BORDER).into();
    theme.colors.muted_foreground = rgb(MUTED).into();
    theme.colors.primary = rgb(ACCENT).into();
    theme.colors.primary_foreground = rgb(0x16293F).into();
    theme.colors.button_primary = rgb(ACCENT).into();
    theme.colors.button_primary_foreground = rgb(0x16293F).into();
    theme.colors.button_primary_hover = rgb(0xBDDFFF).into();
    theme.colors.accent = rgb(0x303C4C).into();
    theme.colors.accent_foreground = rgb(TEXT).into();
    let highlight = Arc::make_mut(&mut theme.highlight_theme);
    highlight.style.editor_background = Some(rgb(SURFACE).into());
    highlight.style.editor_gutter_background = Some(rgb(SURFACE).into());
    highlight.style.editor_active_line = Some(rgb(0x242830).into());
    Theme::sync_base(cx);
}
pub fn thumbnails() -> Vec<(monoprint::Theme, Arc<Image>)> {
    let catalog: &[(&str, &[u8])] = include!("../assets/themes/catalog.rs");
    let mut thumbnails = catalog
        .iter()
        .map(|(id, bytes)| {
            (
                id.parse::<monoprint::Theme>().expect("bundled theme"),
                Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())),
            )
        })
        .collect::<Vec<_>>();
    // Keep related palettes together, with the original frame collection first.
    thumbnails.sort_by_key(|(theme, _)| {
        (
            theme.family() != "Monoprint",
            theme.family(),
            theme.name().to_lowercase(),
        )
    });
    thumbnails
}
