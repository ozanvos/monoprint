use monoprint::{Appearance, Renderer, Scale, Snippet, Style, Theme};

#[test]
fn every_catalog_theme_exports_all_supported_appearances() {
    let renderer = Renderer::new();
    let snippet = Snippet::new("// A real source line\nconst answer = 42;\n\nanswer;")
        .language("typescript".parse().unwrap())
        .title("answer.ts");
    assert_eq!(Theme::all().len(), 69);
    for theme in Theme::all() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            if !theme.supports(appearance) {
                continue;
            }
            let style = Style::new(theme).appearance(appearance).width(520);
            let html = renderer
                .html(&snippet, &style)
                .unwrap_or_else(|e| panic!("{theme} {appearance} HTML: {e}"));
            assert!(html.contains("answer"), "{theme} {appearance}: source");
            assert!(
                !html.contains("__CODE_IMAGES_EDITOR__"),
                "{theme} {appearance}: frame substitution"
            );
            let png = renderer
                .png(&snippet, &style, Scale::X1)
                .unwrap_or_else(|e| panic!("{theme} {appearance} PNG: {e}"));
            assert!(
                png.starts_with(b"\x89PNG\r\n\x1a\n"),
                "{theme} {appearance}"
            );
            let svg = renderer
                .svg(&snippet, &style)
                .unwrap_or_else(|e| panic!("{theme} {appearance} SVG: {e}"));
            assert!(svg.contains("<svg"), "{theme} {appearance}");
        }
    }
}
