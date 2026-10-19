use monoprint::{BundledFont, Error, InvalidOption, LineNumbers, Renderer, Scale, Snippet, Style};

fn dimensions(png: &[u8]) -> (u32, u32) {
    let reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    (reader.info().width, reader.info().height)
}

#[test]
fn font_size_bounds_are_validated_in_every_format() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("x");
    for size in [0, 7, 73, u32::MAX] {
        let style = Style::default().font_size(size);
        for result in [
            renderer.png(&snippet, &style, Scale::X1),
            renderer.svg(&snippet, &style).map(String::into_bytes),
            renderer.html(&snippet, &style).map(String::into_bytes),
        ] {
            assert!(matches!(
                result,
                Err(Error::InvalidOption(InvalidOption::FontSize(value))) if value == size
            ));
        }
    }
    for size in [8, 72] {
        let style = Style::default().font_size(size);
        assert!(renderer.png(&snippet, &style, Scale::X1).is_ok());
        assert!(renderer.svg(&snippet, &style).is_ok());
        assert!(renderer.html(&snippet, &style).is_ok());
    }
}

#[test]
fn explicit_default_preserves_every_format() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("// size\nlet n = 12;\n\n")
        .language("rust".parse().unwrap())
        .title("sample.rs")
        .highlight(2);
    for theme in ["monoprint", "thermal-receipt", "ground", "ayu"] {
        let style = Style::new(theme.parse().unwrap()).line_numbers(LineNumbers::Show);
        let explicit = style.clone().font_size(15);
        assert_eq!(
            renderer.png(&snippet, &style, Scale::X1).unwrap(),
            renderer.png(&snippet, &explicit, Scale::X1).unwrap(),
            "{theme} PNG"
        );
        assert_eq!(
            renderer.svg(&snippet, &style).unwrap(),
            renderer.svg(&snippet, &explicit).unwrap(),
            "{theme} SVG"
        );
        assert_eq!(
            renderer.html(&snippet, &style).unwrap(),
            renderer.html(&snippet, &explicit).unwrap(),
            "{theme} HTML"
        );
    }
}

#[test]
fn larger_text_grows_auto_width_and_wraps_at_fixed_width() {
    let renderer = Renderer::new();
    let snippet = Snippet::new(("alpha beta gamma delta ".repeat(5) + "\n").repeat(3));
    for theme in ["monoprint", "ayu"] {
        let style = Style::new(theme.parse().unwrap());
        let image = |size, fixed| {
            let style = style.clone().font_size(size);
            let style = if fixed { style.width(720) } else { style };
            dimensions(&renderer.png(&snippet, &style, Scale::X1).unwrap())
        };
        let small = image(10, false);
        let normal = image(15, false);
        let large = image(30, false);
        assert!(small.0 < normal.0 && normal.0 < large.0, "{theme}");
        assert!(small.1 < normal.1 && normal.1 < large.1, "{theme}");
        let small_fixed = image(10, true);
        let large_fixed = image(30, true);
        assert_eq!(small_fixed.0, 720);
        assert_eq!(large_fixed.0, 720);
        assert!(large_fixed.1 > large.1, "large code must wrap: {theme}");
        assert!(large_fixed.1 > small_fixed.1, "{theme}");
    }
}

#[test]
fn empty_lines_and_line_numbers_follow_leading_across_frame_variants() {
    let renderer = Renderer::new();
    for theme in ["monoprint", "ground", "ayu"] {
        let style = Style::new(theme.parse().unwrap())
            .font_size(24)
            .line_numbers(LineNumbers::Show);
        // Stay above the frames' minimum heights, which can absorb added
        // content without changing the outer dimensions of a tiny snippet.
        let one = Snippet::new("x\nx\nx\nx\nx\nx").language("rust".parse().unwrap());
        let two = Snippet::new("x\nx\nx\nx\nx\nx\n").language("rust".parse().unwrap());
        let filled = Snippet::new("x\nx\nx\nx\nx\nx\nx").language("rust".parse().unwrap());
        let one = dimensions(&renderer.png(&one, &style, Scale::X1).unwrap());
        let two = dimensions(&renderer.png(&two, &style, Scale::X1).unwrap());
        let filled = dimensions(&renderer.png(&filled, &style, Scale::X1).unwrap());
        assert_eq!(two.0, one.0, "{theme}");
        assert_eq!(two, filled, "{theme}: blank and filled rows share leading");
        assert!(
            two.1 - one.1 >= 24,
            "{theme}: an empty row must not collapse"
        );
    }
}

#[test]
fn comments_line_numbers_and_highlights_share_scaled_layout() {
    let renderer = Renderer::new();
    // Keep the 72px auto-width fixture below the raster pixel budget with the
    // default Monoprint frame's masthead, footer and proportional inner margins.
    let source = format!("// {}\nlet total = 42;", "wide comment ".repeat(4));
    let snippet = Snippet::new(source).language("rust".parse().unwrap());
    let highlighted = snippet
        .clone()
        .highlight(2)
        .highlight_text("1:4-1:8".parse().unwrap());
    let base = Style::default()
        .font("fira-code".parse::<BundledFont>().unwrap().font())
        .comment_font("space-mono".parse::<BundledFont>().unwrap().font())
        .line_numbers(LineNumbers::Show);
    let mut widths = Vec::new();
    for size in [8, 24, 72] {
        let style = base.clone().font_size(size);
        let plain = renderer.png(&snippet, &style, Scale::X1).unwrap();
        let marked = renderer.png(&highlighted, &style, Scale::X1).unwrap();
        assert_eq!(dimensions(&plain), dimensions(&marked));
        assert_ne!(plain, marked);
        widths.push(dimensions(&plain).0);
        let svg = renderer.svg(&highlighted, &style).unwrap();
        let (width, height) = dimensions(&marked);
        assert!(svg.contains(&format!("width=\"{width}\" height=\"{height}\"")));
    }
    assert!(widths[0] < widths[1] && widths[1] < widths[2]);
}

#[test]
fn changing_code_size_preserves_header_glyphs() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("x").title("MMMM");
    // SVG emits the header before the source. Its first glyph definition should
    // have exactly the same outline at both sizes, including frame labels.
    for theme in ["ayu", "dracula", "github-default"] {
        let style = Style::new(theme.parse().unwrap()).width(720);
        let small = renderer.svg(&snippet, &style.clone().font_size(8)).unwrap();
        let large = renderer.svg(&snippet, &style.font_size(36)).unwrap();
        let glyph = |svg: &str, id| {
            svg.split(&format!("<path id=\"g{id}\""))
                .nth(1)
                .expect("glyph definition")
                .split("/>")
                .next()
                .unwrap()
                .to_owned()
        };
        assert_eq!(glyph(&small, 0), glyph(&large, 0), "{theme}: header");
        let glyphs = |svg: &str| {
            svg.split("<path id=\"g")
                .skip(1)
                .map(|definition| definition.split("/>").next().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        let small = glyphs(&small);
        let large = glyphs(&large);
        assert_eq!(small.len(), large.len(), "{theme}");
        assert_eq!(
            small.iter().zip(&large).filter(|(a, b)| a != b).count(),
            1,
            "{theme}: only the source's x glyph should change, preserving every label"
        );
    }
}

#[cfg(feature = "cli")]
#[test]
fn cli_font_size_matches_library_and_rejects_invalid_values() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let renderer = Renderer::new();
    let snippet = Snippet::new("let x = 1;").language("rust".parse().unwrap());
    let style = Style::default().font_size(24);
    for (format, expected) in [
        ("png", renderer.png(&snippet, &style, Scale::X1).unwrap()),
        ("svg", renderer.svg(&snippet, &style).unwrap().into_bytes()),
        (
            "html",
            renderer.html(&snippet, &style).unwrap().into_bytes(),
        ),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_monoprint"))
            .args([
                "--font-size",
                "24",
                "--language",
                "rust",
                "--scale",
                "1",
                "--format",
                format,
                "--output",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"let x = 1;")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected, "{format}");
    }
    for size in ["7", "73"] {
        let output = Command::new(env!("CARGO_BIN_EXE_monoprint"))
            .args(["--font-size", size, "--format", "html", "--output", "-"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--font-size"));
    }
}
