use monoprint::{
    Appearance, Background, Error, InvalidOption, LineNumbers, Renderer, Scale, Snippet, Style,
    Theme,
};

fn snippet() -> Snippet {
    Snippet::new("// A small example\nconst answer = 42;\nconsole.log(answer);")
        .language("tsx".parse().unwrap())
        .title("example.ts")
        .highlight(2)
}

#[test]
fn light_palettes_and_explicit_appearance_are_unambiguous() {
    let theme: Theme = "catppuccin-latte".parse().unwrap();
    assert_eq!(theme.family(), "Catppuccin");
    assert_eq!(theme.default_appearance(), Appearance::Light);
    let renderer = Renderer::new();
    let style = Style::new(theme);
    assert!(
        renderer
            .html(&snippet(), &style)
            .unwrap()
            .contains("data-theme=\"light\"")
    );
    assert!(matches!(
        renderer.html(
            &snippet(),
            &Style::new("nord".parse().unwrap()).appearance(Appearance::Light)
        ),
        Err(Error::InvalidOption(InvalidOption::Appearance { .. }))
    ));
    let style = Style::default().theme(theme);
    assert!(
        renderer
            .html(&snippet(), &style)
            .unwrap()
            .contains("data-theme=\"light\"")
    );
    assert!(
        renderer
            .html(&snippet(), &style.appearance(Appearance::Dark))
            .unwrap()
            .contains("data-theme=\"dark\"")
    );
}

#[test]
fn curated_pairs_keep_legacy_palettes_and_switch_the_entire_frame() {
    let renderer = Renderer::new();
    let pairs = [
        ("ayu", "ayu-dark", "ayu-light"),
        ("catppuccin", "catppuccin-mocha", "catppuccin-latte"),
        (
            "github-default",
            "github-dark-default",
            "github-light-default",
        ),
        ("gruvbox-hard", "gruvbox-dark-hard", "gruvbox-light-hard"),
        (
            "gruvbox-medium",
            "gruvbox-dark-medium",
            "gruvbox-light-medium",
        ),
        ("gruvbox-soft", "gruvbox-dark-soft", "gruvbox-light-soft"),
        ("night-owl", "night-owl", "night-owl-light"),
        ("rose-pine", "rose-pine", "rose-pine-dawn"),
        ("solarized", "solarized-dark", "solarized-light"),
        ("tokyo-night", "tokyo-night", "tokyo-night-light"),
    ];
    for (id, dark, light) in pairs {
        let preset: Theme = id.parse().unwrap();
        assert_eq!(preset.canonical(), preset);
        assert!(Theme::all().any(|t| t == preset));
        for (mode, alias) in [(Appearance::Dark, dark), (Appearance::Light, light)] {
            let legacy: Theme = alias.parse().unwrap();
            assert_eq!(legacy.canonical(), preset);
            assert_eq!(legacy.default_appearance(), mode);
            assert!(legacy.supports(Appearance::Light));
            assert!(legacy.supports(Appearance::Dark));
            if alias != id {
                assert!(preset.aliases().any(|a| a == alias));
                assert!(!Theme::all().any(|t| t == legacy));
            }
            for background in Background::ALL {
                let paired = Style::new(preset).appearance(mode).background(background);
                let original = Style::new(legacy).background(background);
                // Compare syntax, frame, gutter, selection and companion backdrop.
                assert_eq!(
                    renderer.html(&snippet(), &paired).unwrap(),
                    renderer.html(&snippet(), &original).unwrap(),
                    "{id} {mode} {background}"
                );
            }
        }
    }
    for id in [
        "ayu-mirage",
        "catppuccin-frappe",
        "catppuccin-macchiato",
        "github-dark-dimmed",
        "rose-pine-moon",
        "tokyo-night-storm",
    ] {
        let theme: Theme = id.parse().unwrap();
        assert_eq!(theme.canonical(), theme);
        assert!(
            !theme.supports(Appearance::Light),
            "do not invent a palette for {id}"
        );
    }
}

#[test]
fn every_editor_palette_renders_in_all_formats() {
    let renderer = Renderer::new();
    let themes: Vec<_> = Theme::all()
        .filter(|t| !t.family().starts_with("Monoprint"))
        .collect();
    assert_eq!(themes.len(), 21);
    let variants: Vec<_> = themes
        .into_iter()
        .flat_map(|theme| {
            [Appearance::Dark, Appearance::Light]
                .into_iter()
                .filter(move |&mode| theme.supports(mode))
                .map(move |mode| (theme, mode))
        })
        .collect();
    assert_eq!(variants.len(), 31);
    for (theme, appearance) in variants {
        let style = Style::new(theme)
            .appearance(appearance)
            .padding(24)
            .width(400)
            .line_numbers(LineNumbers::Show);
        let html = renderer
            .html(&snippet(), &style)
            .unwrap_or_else(|e| panic!("{theme}: {e}"));
        assert!(html.contains("EditorFrame__window"), "{theme}");
        assert!(html.contains("highlighted-line"), "{theme}");
        assert!(!html.contains("<script"), "{theme}");
        let svg = renderer
            .svg(&snippet(), &style)
            .unwrap_or_else(|e| panic!("{theme}: {e}"));
        assert!(
            !svg.contains("<text") && !svg.contains("foreignObject"),
            "{theme}"
        );
        let png = renderer
            .png(&snippet(), &style, Scale::X1)
            .unwrap_or_else(|e| panic!("{theme}: {e}"));
        let image = image::load_from_memory(&png).unwrap();
        assert_eq!(image.width(), 400, "{theme}");
        if theme.id() == "synthwave-84" {
            assert!(html.contains("text-shadow:"));
            // Remove the frame shadow so this cannot pass on a box-shadow alone.
            assert!(
                renderer
                    .svg(&snippet(), &style.background(Background::None))
                    .unwrap()
                    .contains("feGaussianBlur")
            );
        }
    }
}

#[test]
fn inline_token_glow_paints_in_both_backends_and_none_overrides_it() {
    use std::sync::Arc;
    use takumi::prelude::*;
    let mut fonts = Fonts::default();
    let bytes: &'static [u8] = include_bytes!("../assets/fonts/jetbrains-mono.ttf");
    fonts
        .register(
            FontResource::new(FontSource::from_shared(Arc::new(bytes))).override_info(
                FontOverride {
                    family_name: Some(Arc::from("CodeMono")),
                    ..Default::default()
                },
            ),
        )
        .unwrap();
    let make_node = |red: &str, green: &str| {
        takumi::from_html(&format!(
        "<div style=\"width:360px;height:120px;padding:30px;font-family:CodeMono;font-size:28px;color:#fff;background:#000;text-shadow:0 0 8px #f00\"><span style=\"text-shadow:{red}\">MMMM</span><span style=\"text-shadow:{green}\"> NNNN</span></div>"
    ), FromHtmlOptions::default()).unwrap()
    };
    let viewport = Viewport::new((360, 120));
    let render = |node: Node| {
        let png = takumi_raster::render(
            takumi_raster::RenderOptions::builder()
                .viewport(viewport)
                .node(node.clone())
                .fonts(&fonts)
                .build(),
        )
        .unwrap();
        let svg = takumi_svg::render(
            takumi_svg::SvgOptions::builder()
                .viewport(viewport)
                .node(node)
                .fonts(&fonts)
                .build(),
        )
        .unwrap();
        [
            image::RgbaImage::from_raw(360, 120, png.into_raw()).unwrap(),
            rasterize_svg(svg.as_bytes(), 360, 120),
        ]
    };
    let count_halos = |image: &image::RgbaImage| {
        let red = image
            .pixels()
            .filter(|p| p[0] > p[1].saturating_add(8))
            .count();
        let green = image
            .pixels()
            .filter(|p| p[1] > p[0].saturating_add(8))
            .count();
        (red, green)
    };
    for image in render(make_node("0 0 8px #f00", "0 0 8px #0f0")) {
        let (red, green) = count_halos(&image);
        assert!(
            red > 100 && green > 100,
            "missing token halos: {red} red, {green} green"
        );
    }
    for image in render(make_node("none", "0 0 8px #0f0")) {
        let (red, green) = count_halos(&image);
        assert_eq!(red, 0);
        assert!(green > 100);
    }
}

#[test]
fn backgrounds_keep_geometry_and_transparency_contracts() {
    let renderer = Renderer::new();
    for id in ["removed-frame", "nord", "rose-pine-dawn", "removed-frame"] {
        let mut images = Vec::new();
        for background in Background::ALL {
            let style = Style::new(id.parse().unwrap())
                .padding(48)
                .width(440)
                .background(background);
            let png = renderer.png(&snippet(), &style, Scale::X1).unwrap();
            let image = image::load_from_memory(&png).unwrap().into_rgba8();
            assert_eq!(
                image[(0, 0)][3],
                if background == Background::None {
                    0
                } else {
                    255
                },
                "{id} {background}"
            );
            assert!(
                image[(220, 100)][3] > 0,
                "preserved code panel for {id} {background}"
            );
            if let Some(previous) = images.last() {
                let previous: &image::RgbaImage = previous;
                assert_eq!(
                    image.dimensions(),
                    previous.dimensions(),
                    "{id} {background}"
                );
            }
            images.push(image);
        }
        // Ensure patterns actually paint; accepting CSS without rendering it is insufficient.
        assert_ne!(images[1], images[2], "solid/lattice {id}");
        assert_ne!(images[1], images[3], "solid/halftone {id}");
        assert_ne!(images[2], images[3], "lattice/halftone {id}");
        let style = Style::new(id.parse().unwrap()).padding(48).width(440);
        assert_eq!(
            renderer
                .png(&snippet(), &style.clone().transparent(true), Scale::X1)
                .unwrap(),
            renderer
                .png(&snippet(), &style.background(Background::None), Scale::X1)
                .unwrap()
        );
    }
    assert!("noise".parse::<Background>().is_err());
}

#[test]
fn frameless_light_themes_keep_text_readable_on_custom_backgrounds() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("const answer = 42;\nconsole.log(answer);")
        .language("javascript".parse().unwrap());
    // These frames paint text directly on the canvas, with no panel fill to
    // protect light-mode text from a dark companion background.
    for id in ["removed-frame", "removed-frame", "removed-frame", "removed-frame", "removed-frame"] {
        let mut images = Vec::new();
        for background in [Background::Solid, Background::Lattice, Background::Halftone] {
            let style = Style::new(id.parse().unwrap())
                .appearance(Appearance::Light)
                .padding(32)
                .width(600)
                .line_numbers(LineNumbers::Show)
                .background(background);
            let png = renderer.png(&snippet, &style, Scale::X2).unwrap();
            let image = image::load_from_memory(&png).unwrap().into_rgba8();
            // Check both outside and inside the code area, away from glyphs
            // and pattern dots/lines. This catches dark canvases, not just
            // successfully parsed CSS or light syntax colors.
            for (x, y) in [(12, 12), (900, 100)] {
                assert_eq!(image[(x, y)].0, [255; 4], "{id} {background} at {x},{y}");
            }
            let dark_pixels = image
                .enumerate_pixels()
                .filter(|(x, y, p)| (64..700).contains(x) && (64..155).contains(y) && p[0] < 180)
                .count();
            assert!(dark_pixels > 100, "visible code: {id} {background}");

            // Studio consumes BGRA directly; exported PNG and SVG must agree.
            let preview = renderer.bgra(&snippet, &style, Scale::X2).unwrap();
            for (actual, expected) in preview
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .zip(image.pixels())
            {
                assert_eq!(
                    *actual,
                    [expected[2], expected[1], expected[0], expected[3]]
                );
            }
            let svg = renderer.svg(&snippet, &style).unwrap();
            let vector = rasterize_svg(svg.as_bytes(), image.width(), image.height());
            for (x, y) in [(12, 12), (900, 100)] {
                assert_eq!(vector[(x, y)].0, [255; 4], "SVG {id} {background}");
            }
            images.push(image);
        }
        assert_ne!(images[0], images[1], "lattice is visible: {id}");
        assert_ne!(images[0], images[2], "halftone is visible: {id}");
    }
}

#[test]
fn patterned_svg_backgrounds_match_png_pixels() {
    let renderer = Renderer::new();
    for id in ["nord", "catppuccin-latte"] {
        for background in [Background::Solid, Background::Lattice, Background::Halftone] {
            let style = Style::new(id.parse().unwrap())
                .padding(48)
                .width(440)
                .background(background);
            let png =
                image::load_from_memory(&renderer.png(&snippet(), &style, Scale::X2).unwrap())
                    .unwrap()
                    .into_rgba8();
            let svg = renderer.svg(&snippet(), &style).unwrap();
            let pixels = rasterize_svg(svg.as_bytes(), png.width(), png.height());
            let mut difference = 0_u64;
            // Compare only the outer pattern, independent of glyph antialiasing.
            for y in 0..64 {
                for x in 0..64 {
                    let actual = pixels[(x, y)];
                    let expected = png[(x, y)];
                    for (a, b) in actual.0.into_iter().zip(expected.0) {
                        difference += u64::from(a.abs_diff(b));
                    }
                }
            }
            assert!(
                (difference as f64 / (64. * 64. * 4.)) < 2.,
                "{id} {background}: {difference}"
            );
        }
    }
}

fn rasterize_svg(svg: &[u8], width: u32, height: u32) -> image::RgbaImage {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use takumi::prelude::*;
    let html = format!(
        "<img style=\"width:{width}px;height:{height}px\" src=\"data:image/svg+xml;base64,{}\">",
        STANDARD.encode(svg)
    );
    let node = takumi::from_html(&html, FromHtmlOptions::default()).unwrap();
    let fonts = Fonts::default();
    let actual = takumi_raster::render(
        takumi_raster::RenderOptions::builder()
            .viewport(Viewport::new((width, height)))
            .node(node)
            .fonts(&fonts)
            .build(),
    )
    .unwrap();
    image::RgbaImage::from_raw(width, height, actual.into_raw()).unwrap()
}

#[cfg(feature = "cli")]
#[test]
fn cli_selects_light_palette_and_rejects_conflicting_background_flags() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_monoprint"))
        .args([
            "--theme",
            "catppuccin-latte",
            "--background",
            "halftone",
            "-o",
            "-",
            "--format",
            "html",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"hello").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let expected = Renderer::new()
        .html(
            &Snippet::new("hello"),
            &Style::new("catppuccin-latte".parse().unwrap()).background(Background::Halftone),
        )
        .unwrap();
    assert_eq!(output.stdout, expected.as_bytes());
    let output = Command::new(env!("CARGO_BIN_EXE_monoprint"))
        .args(["--background", "solid", "--transparent", "--list-themes"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
