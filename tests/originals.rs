use monoprint::{Appearance, Background, LineNumbers, Renderer, Scale, Snippet, Style, Theme};

const IDS: [&str; 10] = [
    "single-ink",
    "swatch-card",
    "wayfinding",
    "highlighter-pass",
    "gallery-wall",
    "departure-board",
    "dither-light",
    "cutting-mat",
    "thermal-receipt",
    "proof-marks",
];
const SOURCE: &str = "type User = { name: string };\n\nasync function getUser(\n  id: string,\n): Promise<User> {\n  const res = await fetch(\n    `/api/users/${id}`,\n  );\n\n  return res.json();\n}";

#[test]
fn originals_export_native_pixels_and_portable_vectors() {
    let renderer = Renderer::new();
    let snippet = Snippet::new(SOURCE)
        .language("typescript".parse().unwrap())
        .title("getUser");
    let original_count = Theme::all()
        .filter(|t| t.family() == "Monoprint Originals")
        .count();
    assert!(original_count >= IDS.len());
    for id in IDS {
        let theme: Theme = id.parse().unwrap();
        assert_eq!(
            theme.default_appearance() == Appearance::Dark,
            matches!(id, "swatch-card" | "wayfinding" | "departure-board")
        );
        let style = Style::new(theme).width(680).padding(72).font_size(20);
        let rgba = renderer
            .rgba(&snippet, &style, Scale::X1)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(rgba.width(), 680, "{id}");
        let bgra = renderer.bgra(&snippet, &style, Scale::X1).unwrap();
        assert_eq!((rgba.width(), rgba.height()), (bgra.width(), bgra.height()));
        for (a, b) in rgba
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(bgra.pixels().as_chunks::<4>().0)
        {
            assert_eq!(
                *a,
                [b[2], b[1], b[0], b[3]],
                "{id}: native preview/export agreement"
            );
        }
        let svg = renderer.svg(&snippet, &style).unwrap();
        let vector = rasterize_svg(&svg, rgba.width(), rgba.height());
        let mean_difference = rgba
            .pixels()
            .iter()
            .zip(&vector)
            .map(|(a, b)| u64::from(a.abs_diff(*b)))
            .sum::<u64>() as f64
            / vector.len() as f64;
        assert!(
            mean_difference < 4.0,
            "{id}: PNG/SVG mean channel difference {mean_difference}"
        );
        assert!(
            !svg.contains("<text") && !svg.contains("foreignObject"),
            "{id}"
        );
        let html = renderer.html(&snippet, &style).unwrap();
        assert!(
            !html.contains("__CODE_IMAGES_EDITOR__") && !html.contains("<script"),
            "{id}"
        );
        assert!(html.contains("res.json") || html.contains("json"), "{id}");
        if let Some(dir) = std::env::var_os("MONOPRINT_ORIGINAL_SAMPLES") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("{id}.png")),
                renderer.png(&snippet, &style, Scale::X2).unwrap(),
            )
            .unwrap();
            std::fs::write(dir.join(format!("{id}.svg")), svg).unwrap();
            std::fs::write(dir.join(format!("{id}.html")), html).unwrap();
        }
    }
}

fn rasterize_svg(svg: &str, width: u32, height: u32) -> Vec<u8> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use takumi::prelude::*;
    let html = format!(
        "<img style=\"width:{width}px;height:{height}px\" src=\"data:image/svg+xml;base64,{}\">",
        STANDARD.encode(svg)
    );
    let node = takumi::from_html(&html, FromHtmlOptions::default()).unwrap();
    let fonts = Fonts::default();
    takumi_raster::render(
        takumi_raster::RenderOptions::builder()
            .viewport(Viewport::new((width, height)))
            .node(node)
            .fonts(&fonts)
            .build(),
    )
    .unwrap()
    .into_raw()
}

#[test]
fn originals_wrap_grow_and_preserve_transparent_surroundings() {
    let renderer = Renderer::new();
    let short = Snippet::new("const greeting = \"Hello, 世界 👋\";\n")
        .language("typescript".parse().unwrap());
    let long = Snippet::new("const greeting = \"This longer line must wrap inside the reading surface instead of losing any code\";\n".repeat(14)).language("typescript".parse().unwrap());
    for id in IDS {
        let style = Style::new(id.parse().unwrap())
            .width(360)
            .padding(24)
            .line_numbers(LineNumbers::Show)
            .font_size(18);
        let a = renderer
            .rgba(&short, &style, Scale::X1)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let b = renderer.rgba(&long, &style, Scale::X1).unwrap();
        assert_eq!((a.width(), b.width()), (360, 360), "{id}");
        assert!(
            b.height() > a.height() + 14 * 27,
            "{id}: wrapped lines increase height"
        );
        let clear = renderer
            .rgba(&short, &style.background(Background::None), Scale::X1)
            .unwrap();
        assert_eq!(clear.pixels()[3], 0, "{id}: transparent corner");
        assert_eq!(
            (a.width(), a.height()),
            (clear.width(), clear.height()),
            "{id}"
        );
        let fitted = renderer
            .rgba(
                &short,
                &Style::new(id.parse().unwrap()).font_size(32),
                Scale::X1,
            )
            .unwrap();
        assert!(
            fitted.width() >= 520 && fitted.height() > 100,
            "{id}: automatic width"
        );
    }
}

#[test]
fn original_annotations_use_actual_selections_and_escape_labels() {
    let renderer = Renderer::new();
    for id in ["highlighter-pass", "proof-marks"] {
        let snippet = Snippet::new("first();\nsecond();\n}\n")
            .language("javascript".parse().unwrap())
            .title("<script>__TITLE__</script>");
        let style = Style::new(id.parse().unwrap());
        let html = renderer.html(&snippet, &style).unwrap();
        assert!(html.contains(".Original .line[data-line=\"2\"]"));
        assert!(html.contains("&lt;script&gt;__TITLE__&lt;/script&gt;"));
        let selected = renderer
            .html(&snippet.clone().highlight(1), &style)
            .unwrap();
        assert!(selected.contains(".Original .line[data-line=\"1\"]"));
        assert!(!selected.contains(".Original .line[data-line=\"2\"]"));
        let selected_text = renderer
            .html(
                &snippet.highlight_text(monoprint::TextRange::new((1, 1), (1, 6))),
                &style,
            )
            .unwrap();
        assert!(
            !selected_text.contains(".Original .line[data-line=\"2\"]"),
            "an exact text selection suppresses automatic line emphasis"
        );
    }
}
