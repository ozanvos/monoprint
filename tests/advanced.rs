use monoprint::{BundledFont, Error, Font, Renderer, Scale, Snippet, Style, TextRange};

#[test]
fn bundled_typefaces_cover_the_shipped_manifest_and_render_in_both_roles() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/fonts/manifest.json")).unwrap();
    assert_eq!(
        BundledFont::all().len(),
        manifest.as_object().unwrap().len()
    );
    let renderer = Renderer::new();
    let snippet = Snippet::new("let x = 1; // comment").language("rust".parse().unwrap());
    for bundled in BundledFont::all() {
        assert_eq!(
            bundled.id().to_uppercase().parse::<BundledFont>().unwrap(),
            bundled
        );
        assert!(!bundled.name().is_empty());
        let file = manifest[bundled.id()]["file"].as_str().unwrap();
        let supplied = Font::from_bytes(
            std::fs::read(format!(
                "{}/assets/fonts/{file}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap();
        let named = Style::default()
            .font(bundled.font())
            .comment_font(bundled.font());
        let custom = Style::default()
            .font(supplied.clone())
            .comment_font(supplied);
        assert_eq!(
            renderer.png(&snippet, &named, Scale::X1).unwrap(),
            renderer.png(&snippet, &custom, Scale::X1).unwrap(),
            "{}",
            bundled.id()
        );
    }
    assert!(matches!(
        "missing".parse::<BundledFont>(),
        Err(Error::UnknownFont(_))
    ));
}

fn font(name: &str) -> Font {
    Font::from_bytes(
        std::fs::read(format!(
            "{}/assets/fonts/{name}.ttf",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn comment_typeface_follows_grammar_not_comment_like_strings() {
    let r = Renderer::new();
    let style = Style::default().comment_font(font("space-mono"));
    for (lang, source, comments) in [
        (
            "rust",
            "let s = \"// string\"; // comment\n/* block\n comment */",
            3,
        ),
        (
            "python",
            "s = '# string'\n# comment\n\"\"\"docstring\"\"\"",
            2,
        ),
    ] {
        let html = r
            .html(
                &Snippet::new(source).language(lang.parse().unwrap()),
                &style,
            )
            .unwrap();
        assert!(html.contains("font-family:'CommentFont'"));
        assert!(
            html.matches("font-family:var(--monoprint-comment-font,CommentFont);")
                .count()
                >= comments,
            "{lang}"
        );
        for span in html.split("<span style=\"").skip(1) {
            if ["// string", "# string"]
                .iter()
                .any(|text| span.split("</span>").next().unwrap().contains(text))
            {
                assert!(
                    !span
                        .split("</span>")
                        .next()
                        .unwrap()
                        .contains("font-family:var(--monoprint-comment-font,CommentFont);"),
                    "{lang}: {span}"
                );
            }
        }
    }
}

#[test]
fn comment_glyphs_use_the_comment_font_and_caches_do_not_leak() {
    let r = Renderer::new();
    let snippet = Snippet::new("let x = 1; // Привет").language("rust".parse().unwrap());
    let code = Style::default().font(font("space-mono"));
    let fallback = r.png(&snippet, &code, Scale::X1).unwrap();
    let mixed = code.clone().comment_font(font("jetbrains-mono"));
    let different = r.png(&snippet, &mixed, Scale::X1).unwrap();
    assert_ne!(fallback, different);
    assert!(r.svg(&snippet, &mixed).is_ok());
    assert_eq!(
        fallback,
        r.png(&snippet, &mixed.clone().comments_use_code_font(), Scale::X1)
            .unwrap()
    );
    let same = mixed.clone().comment_font(font("space-mono"));
    assert_eq!(fallback, r.png(&snippet, &same, Scale::X1).unwrap());
    assert_eq!(different, r.png(&snippet, &mixed, Scale::X1).unwrap());
}

#[test]
fn coding_ligatures_change_glyphs_and_can_be_disabled() {
    let r = Renderer::new();
    let snippet = Snippet::new("-> === != =>");
    let off = Style::default().font(font("fira-code")).ligatures(false);
    let on = off.clone().ligatures(true);
    let a = r.svg(&snippet, &off).unwrap();
    let b = r.svg(&snippet, &on).unwrap();
    assert_eq!(
        b,
        r.svg(&snippet, &Style::default().font(font("fira-code")))
            .unwrap()
    );
    // Fira Code combines === and != into fewer drawn glyphs. Comparing glyph
    // references, rather than output bytes, cannot pass on a spacing change alone.
    assert!(b.matches("<use href=\"#g").count() < a.matches("<use href=\"#g").count());
    let html = r.html(&snippet, &on).unwrap();
    assert!(html.contains("font-feature-settings:'liga' 1,'clig' 1,'calt' 1"));
    assert_ne!(
        r.png(&snippet, &off, Scale::X1).unwrap(),
        r.png(&snippet, &on, Scale::X1).unwrap()
    );
    assert_eq!(a, r.svg(&snippet, &on.ligatures(false)).unwrap());
}

#[test]
fn coding_ligatures_preserve_text_without_substitutions() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("A".repeat(100));
    for bundled in BundledFont::all() {
        let base = Style::default().font(bundled.font()).ligatures(false);
        // Cover both automatic sizing and a fixed width that wraps this line.
        for off in [base.clone(), base.width(720)] {
            let on = off.clone().ligatures(true);
            assert_eq!(
                renderer.svg(&snippet, &off).unwrap(),
                renderer.svg(&snippet, &on).unwrap(),
                "{}: ligatures must not move ordinary glyphs or change wrapping",
                bundled.id(),
            );
            assert_eq!(
                renderer.png(&snippet, &off, Scale::X1).unwrap(),
                renderer.png(&snippet, &on, Scale::X1).unwrap(),
                "{}: plain text pixels must stay identical",
                bundled.id(),
            );
        }
    }

    let snippet = Snippet::new(format!("{}\n# {}", "A".repeat(100), "A".repeat(100)))
        .language("python".parse().unwrap());
    let off = Style::default()
        .font(font("fira-code"))
        .comment_font(font("space-mono"))
        .ligatures(false);
    let on = off.clone().ligatures(true);
    assert_eq!(
        renderer.svg(&snippet, &off).unwrap(),
        renderer.svg(&snippet, &on).unwrap()
    );
    assert_eq!(
        renderer.png(&snippet, &off, Scale::X1).unwrap(),
        renderer.png(&snippet, &on, Scale::X1).unwrap(),
        "separate comment typefaces must retain their spacing too",
    );
}

#[test]
fn exact_selections_are_unicode_aware_merge_and_coexist_with_line_highlights() {
    let r = Renderer::new();
    let source = "let café = 1;\r\n// comment";
    let snippet = Snippet::new(source)
        .language("rust".parse().unwrap())
        .highlight(2)
        .highlight_text("1:5-1:9".parse().unwrap())
        .highlight_text(TextRange::new((1, 6), (1, 9)));
    let html = r.html(&snippet, &Style::default()).unwrap();
    assert!(html.contains(">café</span>"));
    assert_eq!(html.matches("class=\"highlighted-text\"").count(), 1);
    assert!(html.contains("line highlighted-line"));
    assert_eq!(
        r.html(&snippet.clear_highlights(), &Style::default())
            .unwrap(),
        r.html(
            &Snippet::new(source).language("rust".parse().unwrap()),
            &Style::default()
        )
        .unwrap()
    );
    let multiline = Snippet::new("é\t<\r\n// β\nend").highlight_text("1:2-2:4".parse().unwrap());
    let html = r.html(&multiline, &Style::default()).unwrap();
    assert_eq!(html.matches("class=\"highlighted-text\"").count(), 2);
    assert!(html.contains(">\t&lt;</span>"));
    assert!(html.contains(">// </span>"));
    let marked = Snippet::new("let x = 1;")
        .language("rust".parse().unwrap())
        .highlight_text("1:5-1:6".parse().unwrap());
    let normal = Snippet::new("let x = 1;").language("rust".parse().unwrap());
    for theme in ["monoprint", "ground", "dusk"] {
        let style = Style::new(theme.parse().unwrap());
        assert_ne!(
            r.png(&marked, &style, Scale::X1).unwrap(),
            r.png(&normal, &style, Scale::X1).unwrap(),
            "{theme}"
        );
        assert_ne!(
            r.svg(&marked, &style).unwrap(),
            r.svg(&normal, &style).unwrap(),
            "{theme}"
        );
    }
}

#[test]
fn text_selections_reject_invalid_coordinates_before_rendering() {
    let r = Renderer::new();
    for value in [
        "0:1-1:2",
        "1:0-1:2",
        "1:2-1:2",
        "2:1-1:1",
        "1:1-3:1",
        "1:1-1:999999999",
    ] {
        let snippet = Snippet::new("éx\n").highlight_text(value.parse().unwrap());
        assert!(
            matches!(
                r.html(&snippet, &Style::default()),
                Err(Error::InvalidOption(_))
            ),
            "{value}"
        );
    }
    for value in ["1-2", "1:1-2", "x:2-3:4"] {
        assert!(value.parse::<TextRange>().is_err());
    }
}
