use gpui::*;
use gpui_base::{ColorTokens, Theme, ThemeAppearance};
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
        muted: 0xA3A39D,
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
        muted: 0x6B6B66,
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
    if Theme::global(cx).appearance == ThemeAppearance::Dark {
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
    let p = if light { Palette::LIGHT } else { Palette::DARK };
    let theme = Theme::global_mut(cx);
    theme.appearance = if light {
        ThemeAppearance::Light
    } else {
        ThemeAppearance::Dark
    };
    theme.tokens.typography.mono = "JetBrains Mono".into();
    theme.tokens.typography.sans = "JetBrains Mono".into();
    theme.tokens.radius.md = px(4.);
    theme.tokens.colors = ColorTokens {
        background: rgb(p.canvas).into(),
        foreground: rgb(p.ink).into(),
        surface: rgb(p.raised).into(),
        surface_foreground: rgb(p.ink).into(),
        border: rgb(p.border).into(),
        input: rgb(p.strong).into(),
        ring: rgb(p.accent).into(),
        muted: rgb(p.surface).into(),
        muted_foreground: rgb(p.muted).into(),
        primary: rgb(p.accent).into(),
        primary_foreground: rgb(p.raised).into(),
        accent: rgb(p.surface3).into(),
        accent_foreground: rgb(p.ink).into(),
        secondary: rgb(p.control).into(),
        secondary_foreground: rgb(p.ink2).into(),
        selection: rgb(p.accent).opacity(0.28).into(),
        ..if light {
            ColorTokens::light()
        } else {
            ColorTokens::dark()
        }
    };
    if let Some(window) = window {
        window.refresh();
    }
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
    // Lead with the curated Originals in group order, then the editor palettes.
    thumbnails.sort_by_key(|(theme, _)| {
        (
            *theme != monoprint::Theme::default(),
            theme.curated_rank().unwrap_or(usize::MAX),
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

pub fn editor_style(p: Palette) -> gpui_base::input::InputEditorStyle {
    gpui_base::input::InputEditorStyle {
        foreground: rgb(p.ink).into(),
        muted_foreground: rgb(p.faint).into(),
        background: rgb(p.canvas).into(),
        editor_gutter_background: Some(rgb(p.canvas).into()),
        editor_active_line: Some(rgb(p.canvas).into()),
        caret: rgb(p.ink).into(),
        selection: rgb(p.accent).opacity(0.28).into(),
        ..Default::default()
    }
}
