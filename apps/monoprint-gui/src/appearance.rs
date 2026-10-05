use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};
use std::{borrow::Cow, sync::Arc};

/// Interface tokens from the supplied design. Export colors belong to the renderer.
#[derive(Clone, Copy)]
pub struct Palette {
    pub light: bool,
    pub canvas: u32,
    pub stage: u32,
    pub raised: u32,
    pub surface: u32,
    pub surface3: u32,
    pub border: u32,
    pub strong: u32,
    pub ink: u32,
    pub ink2: u32,
    pub muted: u32,
    pub faint: u32,
    pub accent: u32,
    pub control: u32,
    pub control_hover: u32,
    pub control_on: u32,
    pub overlay: u32,
    pub overlay_ink: u32,
}
impl Palette {
    pub const DARK: Self = Self {
        light: false,
        canvas: 0x212120,
        stage: 0x1B1B1A,
        raised: 0x252524,
        surface: 0x2C2C2B,
        surface3: 0x3A3A38,
        border: 0x383836,
        strong: 0x4A4A47,
        ink: 0xECECE8,
        ink2: 0xC9C9C4,
        muted: 0x94948E,
        faint: 0x6A6A65,
        accent: 0x8298EA,
        control: 0x373735,
        control_hover: 0x41413F,
        control_on: 0x4E4E4B,
        overlay: 0xECECE8,
        overlay_ink: 0x1F1F1E,
    };
    pub const LIGHT: Self = Self {
        light: true,
        canvas: 0xF7F7F5,
        stage: 0xE8E8E6,
        raised: 0xFBFBF9,
        surface: 0xF0F0EE,
        surface3: 0xDEDEDB,
        border: 0xD0D0CC,
        strong: 0xB9B9B4,
        ink: 0x2A2A28,
        ink2: 0x4A4A47,
        muted: 0x7C7C77,
        faint: 0xA2A29C,
        accent: 0x526FD2,
        control: 0xFBFBF9,
        control_hover: 0xE8E8E6,
        control_on: 0xDEDEDB,
        overlay: 0x252524,
        overlay_ink: 0xF5F5F2,
    };
    pub fn chrome(self) -> Rgba {
        material_tint(self.surface, if self.light { 0.26 } else { 0.18 })
    }
    pub fn preview_background(self) -> Rgba {
        material_tint(self.stage, if self.light { 0.22 } else { 0.24 })
    }
    pub fn inspector_background(self) -> Rgba {
        rgb(self.raised)
    }
}

// AppKit supplies the preview's frosting and contrast. These light tints leave
// it visible; the source and inspector sidebars keep opaque reading surfaces.
// Platforms without the native material keep all surfaces opaque.
fn material_tint(color: u32, opacity: f32) -> Rgba {
    rgb(color).opacity(if cfg!(target_os = "macos") {
        opacity
    } else {
        1.
    })
}

pub fn palette(cx: &App) -> Palette {
    if Theme::global(cx).is_dark() {
        Palette::DARK
    } else {
        Palette::LIGHT
    }
}

pub fn init(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../../../assets/fonts/commit-mono.otf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/fira-code.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/geist-mono.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/google-sans-code.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/ibm-plex-mono.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/inter.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/jetbrains-mono.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/roboto-mono.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/source-code-pro.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/space-mono.ttf")),
        ])
        .expect("load bundled interface and specimen fonts");
    apply(false, None, cx);
}

pub fn apply(light: bool, window: Option<&mut Window>, cx: &mut App) {
    cx.set_window_appearance(Some(if light {
        WindowAppearance::Light
    } else {
        WindowAppearance::Dark
    }));
    Theme::change(
        if light {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        },
        window,
        cx,
    );
    let p = if light { Palette::LIGHT } else { Palette::DARK };
    let theme = Theme::global_mut(cx);
    theme.font_family = "JetBrains Mono".into();
    theme.font_size = px(12.);
    theme.mono_font_family = "JetBrains Mono".into();
    theme.mono_font_size = px(11.);
    theme.radius = px(4.);
    theme.focus_ring = true;
    theme.colors.background = rgb(p.canvas).into();
    theme.colors.foreground = rgb(p.ink).into();
    theme.colors.border = rgb(p.border).into();
    theme.colors.input = rgb(p.strong).into();
    theme.colors.ring = rgb(p.accent).into();
    theme.colors.muted_foreground = rgb(p.muted).into();
    theme.colors.primary = rgb(p.accent).into();
    theme.colors.primary_foreground = rgb(p.raised).into();
    theme.colors.button = rgb(p.control).into();
    theme.colors.button_foreground = rgb(p.ink2).into();
    theme.colors.button_hover = rgb(p.control_hover).into();
    theme.colors.button_active = rgb(p.control_on).into();
    theme.colors.button_primary = rgb(p.ink).into();
    theme.colors.button_primary_foreground = rgb(p.raised).into();
    theme.colors.button_primary_hover = rgb(if light { 0 } else { 0xFFFFFF }).into();
    theme.colors.button_primary_active = rgb(p.ink2).into();
    theme.colors.accent = rgb(p.surface3).into();
    theme.colors.accent_foreground = rgb(p.ink).into();
    theme.colors.secondary = rgb(p.control).into();
    theme.colors.secondary_foreground = rgb(p.ink2).into();
    theme.colors.secondary_hover = rgb(p.control_hover).into();
    theme.colors.secondary_active = rgb(p.control_on).into();
    theme.colors.popover = rgb(p.raised).into();
    theme.colors.popover_foreground = rgb(p.ink).into();
    theme.colors.list = rgb(p.raised).into();
    theme.colors.list_hover = rgb(p.surface).into();
    theme.colors.list_active = rgb(p.surface3).into();
    theme.colors.list_active_border = rgb(p.accent).into();
    theme.colors.caret = rgb(p.ink).into();
    theme.colors.switch = rgb(p.strong).into();
    theme.colors.switch_thumb = rgb(0xFFFFFF).into();
    theme.colors.selection = rgb(p.accent).opacity(0.28).into();
    let highlight = Arc::make_mut(&mut theme.highlight_theme);
    highlight.style.editor_background = Some(rgb(p.canvas).into());
    highlight.style.editor_gutter_background = Some(rgb(p.canvas).into());
    highlight.style.editor_active_line = Some(rgb(p.canvas).into());
    highlight.style.editor_line_number = Some(rgb(p.faint).into());
    highlight.style.editor_active_line_number = Some(rgb(p.ink2).into());
    theme.tokens = (&theme.colors).into();
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

pub fn light_thumbnails(
    thumbnails: &[(monoprint::Theme, Arc<Image>)],
) -> std::collections::HashMap<monoprint::Theme, Arc<Image>> {
    let catalog: &[(&str, &[u8])] = include!("../assets/themes/catalog-light.rs");
    catalog
        .iter()
        .map(|(id, bytes)| {
            let theme: monoprint::Theme = id.parse().expect("bundled light theme");
            // Share preferred light previews instead of decoding the same PNG twice.
            let image = if theme.default_appearance() == monoprint::Appearance::Light {
                thumbnails
                    .iter()
                    .find(|(item, _)| *item == theme)
                    .expect("bundled theme preview")
                    .1
                    .clone()
            } else {
                Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec()))
            };
            (theme, image)
        })
        .collect()
}
