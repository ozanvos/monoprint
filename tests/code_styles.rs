use monoprint::{Background, LineNumbers, Renderer, Scale, Snippet, Style, Theme};

macro_rules! selected_exports {
    ($($case:ident => $id:literal),* $(,)?) => {
        const IDS: &[&str] = &[$($id),*];
        $(#[test] fn $case() { assert_native_exports($id); })*
    };
}
selected_exports! {
    ground => "ground",
    galley => "galley",
    traverse => "traverse",
    plumb => "plumb",
    contour => "contour",
    dusk => "dusk",
    cantilever => "cantilever",
    ordinal => "ordinal",
    ballast => "ballast",
    aside => "aside",
    jetty => "jetty",
    column => "column",
    course => "course",
    well => "well",
    quay => "quay",
    pendant => "pendant",
    capsule => "capsule",
    flange => "flange",
    inlay => "inlay",
    trace => "trace",
}
const SOURCE: &str = "// Make something worth sharing.\ntype Idea = { spark: string };\n\nexport async function create(idea: Idea) {\n  const colors = [\"ink\", \"light\", \"wonder\"];\n  const canvas = await render({\n    ...idea,\n    colors,\n    quality: 100,\n  });\n  return canvas.share();\n}";

fn assert_native_exports(id: &str) {
    let renderer = Renderer::new();
    let snippet = Snippet::new(SOURCE)
        .language("typescript".parse().unwrap())
        .title("create.ts");
    let theme: Theme = id.parse().unwrap();
    assert_eq!(theme.family(), "Monoprint Originals");
    let style = Style::new(theme).width(800).padding(32).font_size(20);
    // SVG uses a 2x viewBox internally. Compare at that same resolution so a
    // dense pixel pattern is not testing two different downsampling filters.
    let rgba = renderer
        .rgba(&snippet, &style, Scale::X2)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(rgba.width(), 1600, "{id}");
    assert!(
        rgba.height() > 600 && rgba.height() < 3000,
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
    let bare = renderer
        .html(
            &Snippet::new(SOURCE).language("typescript".parse().unwrap()),
            &style,
        )
        .unwrap();
    assert_eq!(
        html.split("<body>").nth(1),
        bare.split("<body>").nth(1),
        "{id}: artwork ignores metadata"
    );
    assert!(
        !html.contains("__CODE_IMAGES_EDITOR__") && !html.contains("__LINE_COUNT__"),
        "{id}"
    );
    assert!(html.contains("data-line=\"12\""), "{id}: final source line");
    if let Some(dir) = std::env::var_os("MONOPRINT_CODE_SAMPLES") {
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
fn code_only_frames_grow_with_wrapped_code_and_allow_transparent_backgrounds() {
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
        // Jetty joins its left spine to the canvas edge. Check the opposite corner.
        let bottom_left = if id == "jetty" {
            (clear.width() * 4 - 1) as usize
        } else {
            3
        };
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
        if let Some(dir) = std::env::var_os("MONOPRINT_CODE_SAMPLES") {
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
fn aside_uses_inter_for_comments_and_respects_explicit_typeface_choices() {
    let renderer = Renderer::new();
    let snippet = Snippet::new(
        "// A proportional comment
const value = \"a string, not a comment\";",
    )
    .language("typescript".parse().unwrap());
    let style = Style::new("aside".parse().unwrap());
    let html = renderer.html(&snippet, &style).unwrap();
    assert!(html.contains("font-family:var(--monoprint-comment-font,CommentFont)"));
    let inherited = renderer
        .html(&snippet, &style.clone().comments_use_code_font())
        .unwrap();
    assert!(!inherited.contains("font-family:var(--monoprint-comment-font,CommentFont)"));
    let other = renderer
        .html(&snippet, &style.clone().theme("ground".parse().unwrap()))
        .unwrap();
    assert!(!other.contains("font-family:var(--monoprint-comment-font,CommentFont)"));
    let overridden = style.comment_font(
        "space-mono"
            .parse::<monoprint::BundledFont>()
            .unwrap()
            .font(),
    );
    let a = renderer.rgba(&snippet, &overridden, Scale::X1).unwrap();
    let b = renderer
        .rgba(&snippet, &Style::new("aside".parse().unwrap()), Scale::X1)
        .unwrap();
    assert_ne!(a.pixels(), b.pixels());
}

#[test]
fn contour_surface_follows_each_source_line() {
    let renderer = Renderer::new();
    let snippet =
        Snippet::new("const longer = 12345;\n\nx").language("typescript".parse().unwrap());
    let image = renderer
        .rgba(
            &snippet,
            &Style::new("contour".parse().unwrap())
                .width(600)
                .padding(32)
                .font_size(26),
            Scale::X1,
        )
        .unwrap();
    let pixel = |x, y| &image.pixels()[((y * image.width() + x) * 4) as usize..][..3];
    let near = |actual: &[u8], expected: [u8; 3]| {
        actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 4)
    };
    assert!(near(pixel(200, 33), [250, 247, 239]), "long source strip");
    assert!(
        near(pixel(200, 73), [191, 143, 63]),
        "blank line leaves a short stub"
    );
    assert!(
        near(pixel(200, 113), [191, 143, 63]),
        "short source strip ends before long strip"
    );
    assert!(
        near(pixel(0, 113), [250, 247, 239]),
        "strips reach the canvas edge"
    );
}
