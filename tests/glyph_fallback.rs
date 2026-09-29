use monoprint::{
    Background, BundledFont, Error, Font, LineNumbers, Renderer, Scale, Snippet, Style, TextRole,
};

fn dimensions(png: &[u8]) -> (u32, u32) {
    let info = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    (info.info().width, info.info().height)
}

#[test]
fn color_emoji_sequences_work_with_code_ligatures_disabled() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("👩🏽‍💻 🇳🇱 ❤️ 1️⃣ 🦀");
    let base = Style::default().background(Background::None).font_size(24);
    for enabled in [false, true] {
        let style = base.clone().ligatures(enabled);
        let svg = renderer.svg(&snippet, &style).unwrap();
        assert_eq!(
            svg.matches("<image").count(),
            5,
            "emoji clusters must stay intact"
        );
        assert!(!svg.contains("foreignObject"));
        let png = renderer.png(&snippet, &style, Scale::X2).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&png))
            .read_info()
            .unwrap();
        let mut bytes = vec![0; decoder.output_buffer_size().unwrap()];
        let info = decoder.next_frame(&mut bytes).unwrap();
        let channels = info.color_type.samples();
        let color_pixels = bytes[..info.buffer_size()]
            .chunks_exact(channels)
            .filter(|pixel| {
                pixel[0].max(pixel[1]).max(pixel[2]) - pixel[0].min(pixel[1]).min(pixel[2]) > 80
            })
            .count();
        assert!(
            color_pixels > 100,
            "emoji must contain color, not empty boxes"
        );
        let html = renderer.html(&snippet, &style).unwrap();
        assert!(html.contains("👩🏽‍💻 🇳🇱 ❤️ 1️⃣ 🦀"));
        assert!(html.contains("font-family:'MonoprintEmoji'"));
        assert!(!html.contains("font-family:'MonoprintCjk'"));
    }
}

#[test]
fn auto_width_measures_clusters_and_highlights_keep_geometry() {
    let renderer = Renderer::new();
    let joined = Snippet::new("👩🏽‍💻".repeat(32));
    let separate = Snippet::new("👩🏽 💻".repeat(32));
    let style = Style::default().font_size(24);
    let a = renderer.png(&joined, &style, Scale::X1).unwrap();
    let b = renderer.png(&separate, &style, Scale::X1).unwrap();
    assert!(
        dimensions(&a).0 * 3 / 2 < dimensions(&b).0,
        "joined sequence must be measured as one glyph"
    );
    assert_eq!(
        dimensions(&a).1,
        dimensions(&b).1,
        "auto width must avoid extra wrapping"
    );
    let marked = joined.clone().highlight_text("1:1-1:2".parse().unwrap());
    let svg = renderer.svg(&marked, &style).unwrap();
    assert_eq!(
        svg.matches("<image").count(),
        32,
        "selection inside a cluster must not split it"
    );
    assert_eq!(
        dimensions(&a),
        dimensions(&renderer.png(&marked, &style, Scale::X1).unwrap())
    );
    let wrapped = renderer.png(&joined, &style.width(400), Scale::X1).unwrap();
    assert!(dimensions(&wrapped).1 > dimensions(&a).1);
}

#[test]
fn scripts_comments_and_titles_use_embedded_fallbacks_in_every_format() {
    let renderer = Renderer::new();
    let snippet =
        Snippet::new("// 中文 日本語 한국어\n// مرحبا नमस्ते שלום สวัสดี\nlet message = \"👨‍👩‍👧‍👦\";")
            .language("rust".parse().unwrap())
            .title("你好 🦀.rs")
            .highlight(2);
    let font = "space-mono".parse::<BundledFont>().unwrap().font();
    let style = Style::default()
        .font(font.clone())
        .comment_font(font)
        .font_size(20)
        .line_numbers(LineNumbers::Show)
        .width(800);
    for theme in ["removed-frame", "removed-frame", "removed-frame", "ayu"] {
        let style = style.clone().theme(theme.parse().unwrap());
        let png = renderer.png(&snippet, &style, Scale::X1).unwrap();
        let svg = renderer.svg(&snippet, &style).unwrap();
        let html = renderer.html(&snippet, &style).unwrap();
        assert!(dimensions(&png).0 >= 800);
        assert!(svg.contains("<image"));
        for family in [
            "MonoprintCjk",
            "MonoprintEmoji",
            "MonoprintArabic",
            "MonoprintDevanagari",
            "MonoprintHebrew",
            "MonoprintThai",
        ] {
            assert!(
                html.contains(&format!("font-family:'{family}'")),
                "{theme}: {family}"
            );
        }
        assert!(html.contains("中文 日本語 한국어"));
        assert!(html.contains("你好 🦀.rs"));
    }
}

#[test]
fn fallback_does_not_leak_into_later_ascii_renders_and_unknown_glyphs_fail() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("plain code");
    let style = Style::default().font("space-mono".parse::<BundledFont>().unwrap().font());
    let before = renderer.png(&snippet, &style, Scale::X1).unwrap();
    let html_before = renderer.html(&snippet, &style).unwrap();
    for source in ["🦀", "Привет", "中文", "مرحبا", "สวัสดี"] {
        assert!(
            renderer
                .png(&Snippet::new(source), &style, Scale::X1)
                .is_ok()
        );
    }
    assert_eq!(before, renderer.png(&snippet, &style, Scale::X1).unwrap());
    assert_eq!(html_before, renderer.html(&snippet, &style).unwrap());
    assert!(!html_before.contains("MonoprintEmoji"));
    assert!(!html_before.contains("MonoprintCjk"));
    for snippet in [
        Snippet::new("\u{10ffff}"),
        Snippet::new("x").title("\u{10ffff}"),
    ] {
        assert!(matches!(
            renderer.html(&snippet, &style),
            Err(Error::UnsupportedGlyph {
                character: '\u{10ffff}',
                ..
            })
        ));
    }
    let comments = Snippet::new("// \u{10ffff}").language("rust".parse().unwrap());
    assert!(matches!(
        renderer.html(
            &comments,
            &style.comment_font("space-mono".parse::<BundledFont>().unwrap().font())
        ),
        Err(Error::UnsupportedGlyph {
            text: TextRole::Comment,
            ..
        })
    ));
}

#[test]
fn selected_font_wins_and_explicit_emoji_presentation_is_respected() {
    let renderer = Renderer::new();
    // This font has no heart outline. VS15 must still find a monochrome fallback.
    let code = Font::from_bytes(&include_bytes!("../assets/fonts/space-mono.ttf")[..]).unwrap();
    let style = Style::default().font(code).background(Background::None);
    let text = renderer.svg(&Snippet::new("❤\u{fe0e}"), &style).unwrap();
    let emoji = renderer.svg(&Snippet::new("❤\u{fe0f}"), &style).unwrap();
    assert_eq!(text.matches("<image").count(), 0);
    assert_eq!(emoji.matches("<image").count(), 1);
    let mixed = renderer
        .svg(&Snippet::new("❤\u{fe0e} ❤\u{fe0f}"), &style)
        .unwrap();
    assert_eq!(mixed.matches("<image").count(), 1);
}

#[test]
fn covered_combining_text_uses_shaped_width_without_changing_source() {
    let renderer = Renderer::new();
    let composed = Snippet::new("é".repeat(50));
    let decomposed = Snippet::new("e\u{301}".repeat(50));
    let style = Style::default().font("jetbrains-mono".parse::<BundledFont>().unwrap().font());
    let a = dimensions(&renderer.png(&composed, &style, Scale::X1).unwrap());
    let b = dimensions(&renderer.png(&decomposed, &style, Scale::X1).unwrap());
    assert_eq!(a.1, b.1, "combining marks must not cause extra wrapping");
    // The shaper retains 0.1px tracking for each additional source scalar.
    // Fifty combining marks can add five pixels, not fifty glyph advances.
    assert!(a.0.abs_diff(b.0) <= 6);
    assert!(
        renderer
            .html(&decomposed, &style)
            .unwrap()
            .contains(&"e\u{301}".repeat(50))
    );
    for control in ['\u{ad}', '\u{61c}', '\u{200d}', '\u{fe0e}'] {
        assert!(
            renderer
                .svg(&Snippet::new(format!("a{control}b")), &style)
                .is_ok()
        );
    }
}
