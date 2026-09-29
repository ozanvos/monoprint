use monoprint::{
    Appearance, Error, Font, InvalidOption, Language, LineNumberPolicy, LineNumbers,
    MAX_FONT_BYTES, MAX_SOURCE_BYTES, PngCompression, PngOptions, Renderer, Scale, Snippet, Style,
    Theme,
};
use std::sync::Arc;

#[test]
fn png_compression_preserves_pixels_and_the_fast_default() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("// Hello 👋🏽\nlet answer = 42;\nlet ok = answer != 0;")
        .language("rust".parse().unwrap())
        .title("example.rs")
        .highlight(2);
    for (theme, transparent, scale) in [
        ("removed-frame", false, Scale::X1),
        ("removed-frame", true, Scale::X2),
        ("removed-frame", false, Scale::X3),
        ("removed-frame", true, Scale::X2),
    ] {
        let style = Style::new(theme.parse().unwrap())
            .width(480)
            .padding(32)
            .transparent(transparent);
        let fast = renderer.png(&snippet, &style, scale).unwrap();
        assert_eq!(
            fast,
            renderer
                .png(
                    &snippet,
                    &style,
                    PngOptions::new(scale).compression(PngCompression::Fast)
                )
                .unwrap(),
        );
        let balanced = renderer
            .png(
                &snippet,
                &style,
                PngOptions::new(scale).compression(PngCompression::Balanced),
            )
            .unwrap();
        assert_eq!(
            image::load_from_memory(&fast).unwrap().into_rgba8(),
            image::load_from_memory(&balanced).unwrap().into_rgba8(),
            "{theme}/{transparent}/{scale:?}",
        );
        assert_eq!(fast[25], balanced[25], "PNG color type must be preserved");
        assert!(
            balanced.len() < fast.len(),
            "{theme}: Balanced should reduce this fixture"
        );
    }
    assert_eq!(PngOptions::default(), PngOptions::new(Scale::X4));
    for mode in [PngCompression::Fast, PngCompression::Balanced] {
        assert_eq!(mode.to_string().parse::<PngCompression>().unwrap(), mode);
    }
    assert!(matches!(
        "smallest".parse::<PngCompression>(),
        Err(Error::InvalidOption(InvalidOption::PngCompression(value))) if value == "smallest"
    ));
}

#[test]
fn public_values_can_cross_thread_boundaries() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Renderer>();
    send_sync::<Font>();
    send_sync::<Style>();
    send_sync::<Snippet>();
    send_sync::<Error>();
    send_sync::<monoprint::RasterImage>();
    send_sync::<monoprint::BgraImage>();
}

#[test]
fn raw_pixels_match_png_at_odd_and_even_scales_and_retain_validation() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("// Hello 👋🏽\nlet answer = 42;").language("rust".parse().unwrap());
    for (scale, transparent) in [(Scale::X1, false), (Scale::X2, true), (Scale::X3, true)] {
        let style = Style::default()
            .width(400)
            .padding(16)
            .transparent(transparent);
        let raw = renderer.rgba(&snippet, &style, scale).unwrap();
        let png = renderer.png(&snippet, &style, scale).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!((raw.width(), raw.height()), decoded.dimensions());
        assert_eq!(raw.pixels(), decoded.as_raw());
        let bgra = renderer.rgba(&snippet, &style, scale).unwrap();
        let allocation = bgra.pixels().as_ptr();
        let bgra = bgra.into_bgra_pixels();
        assert_eq!(
            bgra.as_ptr(),
            allocation,
            "BGRA conversion must reuse the allocation"
        );
        for (actual, expected) in bgra
            .as_chunks::<4>()
            .0
            .iter()
            .zip(decoded.as_raw().as_chunks::<4>().0)
        {
            assert_eq!(
                *actual,
                [expected[2], expected[1], expected[0], expected[3]]
            );
        }
        let fused = renderer.bgra(&snippet, &style, scale).unwrap();
        assert_eq!((fused.width(), fused.height()), decoded.dimensions());
        assert_eq!(fused.pixels(), bgra);
        let allocation = fused.pixels().as_ptr();
        let fused = fused.into_pixels();
        assert_eq!(fused.as_ptr(), allocation);
        let allocation = raw.pixels().as_ptr();
        let pixels = raw.into_pixels();
        assert_eq!(
            pixels.as_ptr(),
            allocation,
            "ownership transfer must not copy pixels"
        );
    }
    assert!(matches!(
        renderer.rgba(&snippet, &Style::default().font_size(0), Scale::X2),
        Err(Error::InvalidOption(InvalidOption::FontSize(0)))
    ));
    assert!(matches!(
        renderer.rgba(&snippet, &Style::default().width(8192), Scale::X6),
        Err(Error::ImageTooLarge { .. })
    ));
}

#[test]
fn catalog_parsing_discovery_and_filename_inference() {
    assert_eq!(Theme::all().len(), 54);
    assert_eq!(
        Theme::all()
            .filter(|t| t.supports(Appearance::Light))
            .count(),
        42
    );
    assert_eq!("CANDY".parse::<Theme>().unwrap(), Theme::default());
    assert_eq!(Theme::default().to_string(), "removed-frame");
    assert!(matches!("missing".parse::<Theme>(), Err(Error::UnknownTheme(id)) if id == "missing"));
    assert!(matches!(
        "missing".parse::<Language>(),
        Err(Error::UnknownLanguage(_))
    ));
    assert_eq!(Language::all().count(), 230);
    for language in Language::all() {
        assert!(!language.name().is_empty());
        assert_eq!(language.id().parse::<Language>().unwrap(), language);
        for alias in language.aliases() {
            assert_eq!(alias.parse::<Language>().unwrap(), language, "{alias}");
        }
    }
    for (file, expected) in [
        ("src/MAIN.RS", "rust"),
        ("index.mts", "tsx"),
        ("index.cts", "tsx"),
        ("Containerfile", "docker"),
        ("Dockerfile", "docker"),
        ("GNUmakefile", "make"),
        ("a.txt", "plain"),
    ] {
        assert_eq!(Language::for_file_name(file).unwrap().id(), expected);
    }
    assert_eq!(Language::for_file_name("no.extensionknown"), None);
    assert_eq!(Language::for_file_name("unknown"), None);
    assert_eq!("rs".parse::<Language>().unwrap().name(), "Rust");
    assert!(matches!(
        "removed-frame".parse::<Theme>().unwrap().line_numbers(),
        LineNumberPolicy::Toggleable { default: true }
    ));
}

#[test]
fn invalid_requests_report_structured_errors_before_rendering() {
    let r = Renderer::new();
    let snippet = Snippet::new("x");
    assert!(
        matches!(r.html(&Snippet::new("\r\n".repeat(MAX_SOURCE_BYTES / 2 + 1)), &Style::default()),
        Err(Error::InvalidOption(InvalidOption::SourceTooLarge { len })) if len == MAX_SOURCE_BYTES + 2)
    );
    assert!(matches!(
        r.html(&snippet, &Style::default().padding(u32::MAX)),
        Err(Error::InvalidOption(InvalidOption::Padding(u32::MAX)))
    ));
    for width in [0, 79, 8193, u32::MAX] {
        assert!(matches!(
            r.html(&snippet, &Style::default().padding(0).width(width)),
            Err(Error::InvalidOption(InvalidOption::Width {
                min: 80,
                max: 8192,
                ..
            }))
        ));
    }
    assert!(matches!(
        r.html(&snippet, &Style::default().width(100)),
        Err(Error::InvalidOption(InvalidOption::Width { min: 177, .. }))
    ));
    assert!(matches!(
        r.html(
            &snippet,
            &Style::new("removed-frame".parse().unwrap()).appearance(Appearance::Light)
        ),
        Err(Error::InvalidOption(InvalidOption::Appearance {
            appearance: Appearance::Light,
            ..
        }))
    ));
    assert!(matches!(
        r.html(&snippet.clone().subtitle("no"), &Style::default()),
        Err(Error::InvalidOption(InvalidOption::Subtitle { .. }))
    ));
    for range in [0..=0, std::ops::RangeInclusive::new(2, 1), 1..=usize::MAX] {
        assert!(matches!(
            r.html(&snippet.clone().highlight(range), &Style::default()),
            Err(Error::InvalidOption(InvalidOption::HighlightedLines {
                line_count: 1,
                ..
            }))
        ));
    }
    let mut exhausted = 1..=1;
    assert_eq!(exhausted.next(), Some(1));
    assert!(matches!(
        r.html(&snippet.clone().highlight(exhausted), &Style::default()),
        Err(Error::InvalidOption(InvalidOption::HighlightedLines { .. }))
    ));
    assert!(matches!(
        Scale::try_from(0),
        Err(Error::InvalidOption(InvalidOption::Scale(0)))
    ));
    assert!(matches!(
        Scale::try_from(7),
        Err(Error::InvalidOption(InvalidOption::Scale(7)))
    ));
    assert_eq!(u32::from(Scale::try_from(3).unwrap()), 3);
    assert!(matches!(
        Font::from_bytes(&b"not a font"[..]),
        Err(Error::InvalidFont)
    ));
    assert!(matches!(
        Font::from_bytes(vec![0; MAX_FONT_BYTES + 1]),
        Err(Error::InvalidOption(InvalidOption::FontTooLarge { .. }))
    ));
}

#[test]
fn normalized_source_and_inclusive_ranges_preserve_user_text() {
    let r = Renderer::new();
    let raw = "<a>__TITLE__&\r\nb\rc\n";
    let snippet = Snippet::new(raw)
        .title("<script>x</script>")
        .highlight(2)
        .highlight(2..=4);
    let html = r.html(&snippet, &Style::default()).unwrap();
    let normalized = r
        .html(
            &Snippet::new(raw.replace("\r\n", "\n").replace('\r', "\n"))
                .title("<script>x</script>")
                .highlight(2..=4),
            &Style::default(),
        )
        .unwrap();
    assert_eq!(html, normalized);
    assert!(html.contains("&lt;a&gt;__TITLE__&amp;"));
    assert!(!html.contains("<script>"));
    assert_eq!(html.matches("class=\"line highlighted-line\"").count(), 3);
    assert!(
        r.html(&Snippet::new("").highlight(1), &Style::default())
            .is_ok()
    );
    assert!(matches!(
        r.html(&Snippet::new("x\n").highlight(3), &Style::default()),
        Err(Error::InvalidOption(InvalidOption::HighlightedLines {
            line_count: 2,
            ..
        }))
    ));
}

#[test]
fn output_types_and_dimensions_are_format_specific() {
    let r = Renderer::new();
    let snippet = Snippet::new("let x = 1;").language("rust".parse().unwrap());
    let style = Style::default().width(400);
    let png: Vec<u8> = r.png(&snippet, &style, Scale::X2).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 800);
    let svg: String = r.svg(&snippet, &style).unwrap();
    assert!(svg.contains("width=\"400\""));
    assert!(!svg.contains("<text"));
    assert!(!svg.contains("<foreignObject"));
    let html: String = r.html(&snippet, &style).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("@font-face"));
    assert!(html.contains("data:font/ttf;base64,"));
    assert_eq!(
        r.svg(&snippet, &style.fit_width()).unwrap(),
        r.svg(&snippet, &Style::default()).unwrap()
    );
}

#[test]
fn custom_fonts_are_reused_without_leaking_between_renders() {
    let r = Renderer::new();
    let snippet = Snippet::new("let width = 42;").language("rust".parse().unwrap());
    let style = Style::default();
    let font = Font::from_bytes(&include_bytes!("../assets/fonts/space-mono.ttf")[..]).unwrap();
    let custom = style.clone().font(font);
    let before = r.png(&snippet, &style, Scale::X2).unwrap();
    let custom_png = r.png(&snippet, &custom, Scale::X2).unwrap();
    assert_ne!(before, custom_png);
    assert_eq!(
        custom_png,
        r.clone().png(&snippet, &custom.clone(), Scale::X2).unwrap()
    );
    assert_eq!(before, r.png(&snippet, &style, Scale::X2).unwrap());
    assert_eq!(
        before,
        r.png(&snippet, &custom.clone().theme_font(), Scale::X2)
            .unwrap()
    );
    let additional = Snippet::new("Ж");
    assert!(
        r.html(&additional, &custom)
            .unwrap()
            .contains("MonoprintSans")
    );
    assert!(r.html(&additional, &style).is_ok());
    let title = Snippet::new("x").title("Ж");
    assert!(
        r.html(&title, &custom.clone().theme("removed-frame".parse().unwrap()))
            .is_ok()
    );
    assert!(r.html(&title, &custom).is_ok());
    assert!(r.html(&Snippet::new("x").title("🦀"), &style).is_ok());
}

#[test]
fn concurrent_first_use_matches_sequential_outputs() {
    let r = Renderer::new();
    let font = Font::from_bytes(Arc::<[u8]>::from(
        &include_bytes!("../assets/fonts/space-mono.ttf")[..],
    ))
    .unwrap();
    let style = Style::new("removed-frame".parse().unwrap()).font(font);
    let snippet = Snippet::new("const answer = 42;")
        .language("ts".parse().unwrap())
        .highlight(1);
    let outputs = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let (r, style, snippet) = (r.clone(), style.clone(), snippet.clone());
                scope.spawn(move || {
                    (
                        r.svg(&snippet, &style).unwrap(),
                        r.png(&snippet, &style, Scale::X2).unwrap(),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    let sequential = (
        r.svg(&snippet, &style).unwrap(),
        r.png(&snippet, &style, Scale::X2).unwrap(),
    );
    for result in outputs {
        assert_eq!(result, sequential);
    }
}

#[test]
fn fixed_width_tailwind_reports_png_budget_and_recovers() {
    let source = (0..30)
        .map(|i| {
            format!(
                "{:.<80}",
                format!("let value_{i:03} = \"hello {i}\"; // benchmark code image ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let renderer = Renderer::new();
    let snippet = Snippet::new(source).language("rust".parse().unwrap());
    let style = Style::new("removed-frame".parse().unwrap())
        .width(896)
        .line_numbers(LineNumbers::Show);
    // The frame's borders and gutter wrap these lines into a 22 MP canvas at 4x.
    // Reject before entering Takumi, whose root and SVG-source caps are 16 MP.
    for style in [
        style.clone(),
        style.font(
            Font::from_bytes(&include_bytes!("../assets/fonts/jetbrains-mono.ttf")[..]).unwrap(),
        ),
    ] {
        assert!(matches!(
            renderer.png(&snippet, &style, Scale::X4),
            Err(Error::ImageTooLarge {
                width: 3584,
                height: 6184,
                supersampled: false
            })
        ));
        let png = renderer.png(&snippet, &style, Scale::X2).unwrap();
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 1792);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 3092);
        let svg = renderer.svg(&snippet, &style).unwrap();
        assert!(svg.contains("width=\"896\" height=\"1546\""));
    }
}

#[test]
fn image_budget_is_a_typed_error_and_renderer_recovers() {
    let r = Renderer::new();
    let error = r
        .png(
            &Snippet::new("x".repeat(5000)),
            &Style::default(),
            Scale::X3,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        Error::ImageTooLarge {
            supersampled: true,
            ..
        }
    ));
    assert!(error.to_string().contains("internal render pixels"));
    assert!(
        r.png(&Snippet::new("x"), &Style::default(), Scale::X2)
            .is_ok()
    );
}

#[test]
fn every_theme_preserves_default_line_numbers_and_allows_overrides() {
    let r = Renderer::new();
    let plain = Snippet::new("a\nb");
    let code = plain.clone().language("rust".parse().unwrap());
    let style = Style::default().line_numbers(LineNumbers::Show);
    let editor_has_numbers =
        |html: String| html.contains("class=\"Editor__editor Editor__showLineNumbers");
    assert!(!editor_has_numbers(r.html(&plain, &style).unwrap()));
    assert!(editor_has_numbers(r.html(&code, &style).unwrap()));
    for theme in Theme::all() {
        let default = matches!(
            theme.id(),
            "removed-frame"
                | "removed-frame"
                | "removed-frame"
                | "removed-frame"
                | "removed-frame"
                | "removed-frame"
                | "removed-frame"
                | "removed-frame"
                | "removed-frame"
        );
        assert_eq!(
            theme.line_numbers(),
            LineNumberPolicy::Toggleable { default }
        );
        for appearance in [Appearance::Light, Appearance::Dark] {
            if !theme.supports(appearance) {
                continue;
            }
            for (preference, expected) in [
                (LineNumbers::Auto, default),
                (LineNumbers::Show, true),
                (LineNumbers::Hide, false),
            ] {
                let style = Style::new(theme)
                    .appearance(appearance)
                    .line_numbers(preference);
                assert_eq!(
                    editor_has_numbers(r.html(&code, &style).unwrap()),
                    expected,
                    "{theme} {appearance} {preference:?}"
                );
                assert!(!editor_has_numbers(r.html(&plain, &style).unwrap()));
            }
        }
    }
}

#[cfg(feature = "cli")]
#[test]
fn cli_and_library_export_identical_bytes() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let source = "const answer = 42;\r\n";
    let r = Renderer::new();
    let snippet = Snippet::new(source)
        .language("ts".parse().unwrap())
        .title("sample.ts")
        .highlight(1);
    let style = Style::new("removed-frame".parse().unwrap())
        .padding(32)
        .line_numbers(LineNumbers::Show);
    for (format, expected) in [
        ("png", r.png(&snippet, &style, Scale::X2).unwrap()),
        ("svg", r.svg(&snippet, &style).unwrap().into_bytes()),
        ("html", r.html(&snippet, &style).unwrap().into_bytes()),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_monoprint"))
            .args([
                "-o",
                "-",
                "--format",
                format,
                "-l",
                "ts",
                "--theme",
                "removed-frame",
                "--padding",
                "32",
                "--line-numbers",
                "--title",
                "sample.ts",
                "--highlight",
                "1",
                "--scale",
                "2",
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
            .write_all(source.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(expected, output.stdout, "{format}");
    }
}

#[test]
fn font_collections_are_rejected_even_when_the_first_face_is_valid() {
    let font = include_bytes!("../assets/fonts/space-mono.ttf");
    let mut collection = b"ttcf\0\x01\0\0\0\0\0\x01\0\0\0\x10".to_vec();
    collection.extend_from_slice(font);
    let tables = u16::from_be_bytes(font[4..6].try_into().unwrap());
    for table in 0..usize::from(tables) {
        let offset = 16 + 12 + table * 16 + 8;
        let value = u32::from_be_bytes(collection[offset..offset + 4].try_into().unwrap());
        collection[offset..offset + 4].copy_from_slice(&(value + 16).to_be_bytes());
    }
    assert!(ttf_parser::Face::parse(&collection, 0).is_ok());
    assert!(matches!(
        Font::from_bytes(collection),
        Err(Error::InvalidFont)
    ));
}

#[test]
fn preview_selections_can_be_replaced_without_rebuilding_content() {
    let renderer = Renderer::new();
    let original = Snippet::new("a\nb").title("example").highlight(1);
    let updated = original.clear_highlights().highlight(2);
    let expected = Snippet::new("a\nb").title("example").highlight(2);
    assert_eq!(
        renderer.html(&updated, &Style::default()).unwrap(),
        renderer.html(&expected, &Style::default()).unwrap()
    );
}
