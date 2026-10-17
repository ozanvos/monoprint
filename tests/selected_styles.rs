use monoprint::{Background, BundledFont, LineNumbers, Renderer, Scale, Snippet, Style, Theme};

macro_rules! selected_exports {
    ($($case:ident => $id:literal),* $(,)?) => {
        const IDS: &[&str] = &[$($id),*];
        $(#[test] fn $case() { assert_native_exports($id); })*
    };
}
selected_exports! {
    the_impression => "the-impression",
    carrier_sheet => "carrier-sheet",
    ultraviolet_drum => "ultraviolet-drum",
    die_cut => "die-cut",
    counterform => "counterform",
    gallery_recess => "gallery-recess",
    light_table => "light-table",
    ragged_block => "ragged-block",
    quiet_typesetter => "quiet-typesetter",
    spot_colour => "spot-colour",
    press_proof => "press-proof",
    segment_glass => "segment-glass",
    label_tape => "label-tape",
    two_colour_thermal => "two-colour-thermal",
    urushi => "urushi",
    one_bit => "one-bit",
    holofoil => "holofoil",
}
const SOURCE: &str = "// Make something worth sharing.\ntype Idea = { spark: string };\n\nexport async function create(idea: Idea) {\n  const colors = [\"ink\", \"light\", \"wonder\"];\n  const canvas = await render({\n    ...idea,\n    colors,\n    quality: 100,\n  });\n  return canvas.share();\n}";

fn assert_native_exports(id: &str) {
    let renderer = Renderer::new();
    let snippet = Snippet::new(SOURCE)
        .language("typescript".parse().unwrap())
        .title("create.ts");
    let theme: Theme = id.parse().unwrap();
    assert_eq!(theme.family(), "Monoprint Originals");
    let style = Style::new(theme).width(1000).padding(64).font_size(25);
    // SVG uses a 2x viewBox internally. Compare at that same resolution so a
    // dense pixel pattern is not testing two different downsampling filters.
    let rgba = renderer
        .rgba(&snippet, &style, Scale::X2)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(rgba.width(), 2000, "{id}");
    assert!(
        rgba.height() > 800 && rgba.height() < 3000,
        "{id}: unexpected height {}",
        rgba.height()
    );
    let bgra = renderer.bgra(&snippet, &style, Scale::X2).unwrap();
    assert_eq!(
        (rgba.width(), rgba.height()),
        (bgra.width(), bgra.height()),
        "{id}"
    );
    for (a, b) in rgba
        .pixels()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(bgra.pixels().as_chunks::<4>().0)
    {
        assert_eq!(*a, [b[2], b[1], b[0], b[3]], "{id}: native preview parity");
    }
    let svg = renderer
        .svg(&snippet, &style)
        .unwrap_or_else(|e| panic!("{id} SVG: {e}"));
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
        "{id}: portable vector"
    );
    let html = renderer.html(&snippet, &style).unwrap();
    assert!(
        !html.contains("__CODE_IMAGES_EDITOR__") && !html.contains("__LINE_COUNT__"),
        "{id}"
    );
    assert!(html.contains("data-line=\"12\""), "{id}: final source line");
    if let Some(dir) = std::env::var_os("MONOPRINT_SELECTED_SAMPLES") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{id}.png")),
            renderer.png(&snippet, &style, Scale::X1).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join(format!("{id}.svg")), svg).unwrap();
        std::fs::write(dir.join(format!("{id}.html")), html).unwrap();
    }
    println!("{id}: {} × {}", rgba.width(), rgba.height());
}

#[test]
fn selected_frames_grow_with_wrapped_code_and_allow_transparent_backgrounds() {
    let renderer = Renderer::new();
    let short =
        Snippet::new("const value = 1;\n\nreturn value;").language("typescript".parse().unwrap());
    let long = Snippet::new("const message = \"Every character in this long source line must remain readable when the image is narrow\";\n".repeat(5))
        .language("typescript".parse().unwrap());
    for &id in IDS {
        let style = Style::new(id.parse().unwrap())
            .width(360)
            .padding(24)
            .font_size(18)
            .line_numbers(LineNumbers::Show);
        let a = renderer
            .rgba(&short, &style, Scale::X1)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let b = renderer
            .rgba(&long, &style, Scale::X1)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!((a.width(), b.width()), (360, 360), "{id}");
        assert!(
            b.height() > a.height() + 200,
            "{id}: long lines must wrap and increase height"
        );
        let clear = renderer
            .rgba(
                &short,
                &style.clone().background(Background::None),
                Scale::X1,
            )
            .unwrap();
        // One Bit intentionally keeps its menu bar at the top of the image.
        let bottom_left = ((clear.height() - 1) * clear.width() * 4 + 3) as usize;
        assert!(
            clear.pixels()[bottom_left] <= 2,
            "{id}: transparent surroundings (including soft shadow fringe)"
        );
        assert_eq!(
            (a.width(), a.height()),
            (clear.width(), clear.height()),
            "{id}"
        );
        let auto = renderer
            .rgba(
                &short,
                &Style::new(id.parse().unwrap()).font_size(24),
                Scale::X1,
            )
            .unwrap();
        assert!(
            auto.width() >= 520 && auto.height() > 100,
            "{id}: automatic sizing"
        );
        if let Some(dir) = std::env::var_os("MONOPRINT_SELECTED_SAMPLES") {
            let dir = std::path::PathBuf::from(dir).join("wrapped");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("{id}.html")),
                renderer.html(&long, &style).unwrap(),
            )
            .unwrap();
        }
    }
}

fn rasterize_svg(svg: &str, width: u32, height: u32) -> Vec<u8> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use takumi::prelude::*;
    let node = takumi::from_html(&format!(
        "<img style=\"width:{width}px;height:{height}px\" src=\"data:image/svg+xml;base64,{}\">",
        STANDARD.encode(svg)), FromHtmlOptions::default()).unwrap();
    takumi_raster::render(
        takumi_raster::RenderOptions::builder()
            .viewport(Viewport::new((width, height)))
            .node(node)
            .fonts(&Fonts::default())
            .build(),
    )
    .unwrap()
    .into_raw()
}

#[test]
fn material_strips_contain_wrapped_custom_font_text() {
    let renderer = Renderer::new();
    for id in ["label-tape", "ragged-block"] {
        let style = Style::new(id.parse().unwrap())
            .width(680)
            .padding(72)
            .font_size(20)
            .font("fira-code".parse::<BundledFont>().unwrap().font())
            .ligatures(true);
        let snippet =
            Snippet::new("type User = { name: string };").language("typescript".parse().unwrap());
        let image = renderer.rgba(&snippet, &style, Scale::X1).unwrap();
        // The former padded inline-block measured its glyphs without its
        // padding: the closing `};` wrapped below the strip onto the canvas.
        let mut bottom = 0;
        for (index, &[r, g, b, _]) in image.pixels().as_chunks::<4>().0.iter().enumerate() {
            let strip = if id == "label-tape" {
                r > 130 && g < 80 && b < 80
            } else {
                r < 50 && (50..80).contains(&g) && (60..90).contains(&b)
            };
            if strip {
                bottom = bottom.max(index as u32 / image.width());
            }
        }
        assert!(bottom > 0, "{id}: material strip present");
        let width = image.width();
        let stray_ink = (bottom + 3..(bottom + 23).min(image.height()))
            .flat_map(|y| (90..450).map(move |x| ((y * width + x) * 4) as usize))
            .filter(|&index| image.pixels()[index..index + 3].iter().all(|v| *v > 190))
            .count();
        assert_eq!(stray_ink, 0, "{id}: code must remain inside its strip");
        let source = "type User = { name: string };\n\nasync function getUser(id: string): Promise<User> {\n  const res = await fetch(`/api/users/${id}`);\n  return res.json();\n}";
        let compact = source.replace("getUser(id: string): Promise<User>", "getUser()");
        let snippet = Snippet::new(source)
            .language("typescript".parse().unwrap())
            .title("user.ts");
        let compact = Snippet::new(compact)
            .language("typescript".parse().unwrap())
            .title("user.ts");
        let wrapped_height = renderer.rgba(&snippet, &style, Scale::X1).unwrap().height();
        let compact_height = renderer.rgba(&compact, &style, Scale::X1).unwrap().height();
        assert!(
            wrapped_height >= compact_height + 25,
            "{id}: wrapped third line must push following lines down"
        );
        if let Some(dir) = std::env::var_os("MONOPRINT_SELECTED_SAMPLES") {
            std::fs::write(
                std::path::PathBuf::from(dir).join(format!("{id}-custom-font.png")),
                renderer.png(&snippet, &style, Scale::X2).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn selected_annotations_use_real_metadata_and_preserve_literal_text() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("print('__LINE_COUNT__')\n\n# final")
        .language("python".parse().unwrap())
        .title("<tag>__TITLE__</tag>")
        .highlight(3);
    for &id in IDS {
        let html = renderer
            .html(&snippet, &Style::new(id.parse().unwrap()))
            .unwrap();
        assert!(html.contains("&lt;tag&gt;__TITLE__&lt;/tag&gt;"), "{id}");
        assert!(
            html.contains("__LINE_COUNT__"),
            "{id}: literal source marker"
        );
        assert!(
            html.contains("class=\"line empty-line\" data-line=\"2\""),
            "{id}: blank lines remain blank"
        );
        assert!(
            html.contains("class=\"line highlighted-line\" data-line=\"3\""),
            "{id}"
        );
        if id == "segment-glass" {
            assert!(html.contains("LN 3"));
        }
        if id == "two-colour-thermal" {
            assert!(html.contains("3 LINES"));
        }
    }
}
