use monoprint::{
    Appearance, BundledFont, HighlightBackend, Language, Renderer, Scale, Snippet, Style, TextRange,
};

#[test]
fn editor_uses_default_monoprint_syntax_colors() {
    let source = "// Comment\nlet value = \"hello\";";
    for backend in [HighlightBackend::Auto, HighlightBackend::Giallo] {
        let renderer = Renderer::new().highlight_backend(backend);
        for (appearance, comment, keyword, string) in [
            (Appearance::Light, "#6A6A65", "#A0452B", "#6E5A35"),
            (Appearance::Dark, "#9E9E98", "#E09A7E", "#D4BF92"),
        ] {
            // Rust uses the native lexer; Hem uses the fallback grammar.
            for language in ["rust", "hem"] {
                let spans = renderer
                    .highlight_source(source, language.parse().unwrap(), appearance)
                    .unwrap();
                for (token, expected) in
                    [("// Comment", comment), ("let", keyword), ("hello", string)]
                {
                    let offset = source.find(token).unwrap();
                    let span = spans
                        .iter()
                        .find(|span| span.range.contains(&offset))
                        .unwrap();
                    assert_eq!(span.foreground, expected, "{backend:?} {language} {token}");
                }
            }
        }
    }
}

#[test]
fn editor_ranges_follow_original_utf8_line_endings_for_both_backends() {
    let source = "// café 🦀\r\n\r\nlet café = \"日本語\";\r// fin\n";
    for backend in [HighlightBackend::Auto, HighlightBackend::Giallo] {
        let renderer = Renderer::new().highlight_backend(backend);
        for language in ["rust", "hem", "plain"] {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let runs = renderer
                    .highlight_source(source, language.parse().unwrap(), appearance)
                    .unwrap();
                let mut end = 0;
                for run in runs {
                    assert!(run.range.start >= end);
                    assert!(!run.range.is_empty());
                    assert!(source.get(run.range.clone()).is_some());
                    assert!(run.foreground.starts_with('#'));
                    end = run.range.end;
                }
            }
        }
    }
}

#[test]
fn fallback_keeps_exports_identical_with_selections_and_comment_fonts() {
    let auto = Renderer::new();
    let giallo = auto.clone().highlight_backend(HighlightBackend::Giallo);
    let source = "// café\neffect greeting(): string {\n  raise \"hello\"\n}";
    let snippet = Snippet::new(source)
        .language("hem".parse().unwrap())
        .highlight(2)
        .highlight_text(TextRange::new((3, 3), (3, 8)));
    let style = Style::default()
        .width(640)
        .font_size(15)
        .transparent(true)
        .comment_font("space-mono".parse::<BundledFont>().unwrap().font());
    assert_eq!(
        auto.html(&snippet, &style).unwrap(),
        giallo.html(&snippet, &style).unwrap()
    );
    assert_eq!(
        auto.svg(&snippet, &style).unwrap(),
        giallo.svg(&snippet, &style).unwrap()
    );
    assert_eq!(
        auto.png(&snippet, &style, Scale::X2).unwrap(),
        giallo.png(&snippet, &style, Scale::X2).unwrap()
    );
}

#[test]
fn concurrent_clones_keep_native_palettes_and_lexers_isolated() {
    let renderer = Renderer::new();
    let source = "// café\nfn main() { let value = \"日本語\"; }";
    std::thread::scope(|scope| {
        for language in ["rust", "typescript", "python", "hem"] {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let r = renderer.clone();
                let language: Language = language.parse().unwrap();
                let expected = r.highlight_source(source, language, appearance).unwrap();
                scope.spawn(move || {
                    for _ in 0..12 {
                        assert_eq!(
                            r.highlight_source(source, language, appearance).unwrap(),
                            expected
                        );
                    }
                });
            }
        }
    });
}
