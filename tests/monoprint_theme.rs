use monoprint::{
    Appearance, Background, Font, LineNumbers, Renderer, Scale, Snippet, Style, Theme,
};

const SOURCE: &str = "// Make something worth sharing.\ntype Idea = { spark: string };\n\nexport async function create(idea: Idea) {\n  const colors = [\"ink\", \"light\", \"wonder\"];\n  const canvas = await render({\n    ...idea,\n    colors,\n    quality: 100,\n  });\n  return canvas.share();\n}";

#[test]
fn default_monoprint_exports_both_approved_appearances() {
    let renderer = Renderer::new();
    let theme = Theme::default();
    assert_eq!(theme.id(), "monoprint");
    assert_eq!(theme.name(), "Monoprint");
    assert_eq!(Theme::all().filter(|t| t.id() == "monoprint").count(), 1);
    let snippet = Snippet::new(SOURCE)
        .language("typescript".parse().unwrap())
        .title("create.ts");
    for (appearance, stage, paper, keyword, literal) in [
        (
            Appearance::Light,
            [232, 232, 229, 255],
            [247, 247, 245, 255],
            "#a0452b",
            "#6e5a35",
        ),
        (
            Appearance::Dark,
            [27, 27, 26, 255],
            [37, 37, 36, 255],
            "#e09a7e",
            "#d4bf92",
        ),
    ] {
        assert!(theme.supports(appearance));
        let style = Style::default()
            .appearance(appearance)
            .width(1000)
            .padding(44)
            .font_size(25);
        let image = renderer.rgba(&snippet, &style, Scale::X1).unwrap();
        assert_eq!((image.width(), image.height()), (1000, 750));
        assert_eq!(&image.pixels()[0..4], &stage);
        let paper_pixel = (140 * image.width() as usize + 50) * 4;
        assert_eq!(&image.pixels()[paper_pixel..paper_pixel + 4], &paper);
        let html = renderer.html(&snippet, &style).unwrap();
        for text in [
            "create.ts",
            "TypeScript",
            "<div class=\"Monoprint__foot\" aria-hidden=\"true\"></div>",
            keyword,
            literal,
        ] {
            assert!(html.contains(text), "{appearance}: missing {text}");
        }
        assert!(!html.contains("class=\"Editor__editor Editor__showLineNumbers"));
        assert!(!html.contains("__TITLE__") && !html.contains("__LINE_COUNT__"));
        let svg = renderer.svg(&snippet, &style).unwrap();
        assert!(!svg.contains("<text") && !svg.contains("foreignObject"));
        let bgra = renderer.bgra(&snippet, &style, Scale::X1).unwrap();
        for (rgba, bgra) in image
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(bgra.pixels().as_chunks::<4>().0)
        {
            assert_eq!(
                [rgba[0], rgba[1], rgba[2], rgba[3]],
                [bgra[2], bgra[1], bgra[0], bgra[3]]
            );
        }
        if let Some(dir) = std::env::var_os("MONOPRINT_THEME_SAMPLES") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("monoprint-{appearance}.png")),
                renderer.png(&snippet, &style, Scale::X1).unwrap(),
            )
            .unwrap();
            std::fs::write(dir.join(format!("monoprint-{appearance}.html")), html).unwrap();
            std::fs::write(dir.join(format!("monoprint-{appearance}.svg")), svg).unwrap();
        }
    }
}

#[test]
fn monoprint_export_scale_preserves_layout_dimensions() {
    let renderer = Renderer::new();
    let snippet =
        Snippet::new("const answer = 42;\n".repeat(6)).language("typescript".parse().unwrap());
    for appearance in [Appearance::Light, Appearance::Dark] {
        let style = Style::default().appearance(appearance).font_size(24);
        let base = renderer.rgba(&snippet, &style, Scale::X1).unwrap();
        for scale in [Scale::X2, Scale::X3, Scale::X4] {
            let image = renderer.rgba(&snippet, &style, scale).unwrap();
            let factor = u32::from(scale);
            assert_eq!(
                (image.width(), image.height()),
                (base.width() * factor, base.height() * factor),
                "{appearance}/{scale:?}"
            );
        }
    }
}

#[test]
fn monoprint_grows_with_wrapped_code_and_keeps_appearance_when_transparent() {
    let renderer = Renderer::new();
    let short = Snippet::new("const answer = 42;").language("typescript".parse().unwrap());
    let long = Snippet::new("const message = \"A longer line that wraps in the reading column without clipping any source\";\n".repeat(12))
        .language("typescript".parse().unwrap())
        .title("a-long-filename-with-世界-and-__TITLE__.ts")
        .highlight(2);
    for appearance in [Appearance::Light, Appearance::Dark] {
        let style = Style::default()
            .appearance(appearance)
            .width(360)
            .padding(24)
            .font_size(18)
            .font(
                Font::from_bytes(include_bytes!("../assets/fonts/ibm-plex-mono.ttf").to_vec())
                    .unwrap(),
            )
            .line_numbers(LineNumbers::Show);
        let a = renderer.rgba(&short, &style, Scale::X1).unwrap();
        let b = renderer.rgba(&long, &style, Scale::X1).unwrap();
        assert_eq!(b.width(), 360);
        assert!(b.height() > a.height() + 12 * 29);
        let html = renderer.html(&long, &style).unwrap();
        assert!(html.contains("<div class=\"Monoprint__foot\" aria-hidden=\"true\"></div>"));
        assert!(html.contains("世界-and-__TITLE__.ts"));
        assert!(html.contains("class=\"line highlighted-line\" data-line=\"2\""));
        let clear = renderer
            .rgba(
                &short,
                &style.clone().background(Background::None),
                Scale::X1,
            )
            .unwrap();
        assert_eq!(&clear.pixels()[0..4], &[0, 0, 0, 0]);
        assert_eq!((clear.width(), clear.height()), (a.width(), a.height()));
        let solid = renderer
            .rgba(&short, &style.background(Background::Solid), Scale::X1)
            .unwrap();
        assert_eq!(&solid.pixels()[0..4], &a.pixels()[0..4]);
    }
}
