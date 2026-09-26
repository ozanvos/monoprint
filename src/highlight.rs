use crate::{Snippet, assets, highlighted_lines};
use anyhow::Result;
use giallo::{FontStyle, HighlightOptions, Registry, ThemeVariant};
use std::fmt::Write;

pub struct Highlighted {
    pub html: String,
    pub language: String,
    pub line_count: usize,
}

pub fn code(
    registry: &Registry,
    snippet: &Snippet,
    source: &str,
    theme: &assets::Theme,
    mode: &str,
) -> Result<Highlighted> {
    let language = snippet.language.id().to_owned();
    let theme_name = format!("ink-{}-{mode}", theme.id);
    let options =
        HighlightOptions::new(&language, ThemeVariant::Single(&theme_name)).merge_whitespace(false);
    let result = registry.highlight(source, &options)?;
    let line_count = source.split('\n').count();
    let marked = highlighted_lines(snippet);
    let mut html = String::new();
    for (index, tokens) in result.tokens.iter().enumerate() {
        let class = if marked.contains(&(index + 1)) {
            "line highlighted-line"
        } else {
            "line"
        };
        write!(html, "<span class=\"{class}\" data-line=\"{}\">", index + 1)?;
        for token in tokens {
            let ThemeVariant::Single(style) = &token.style else {
                unreachable!()
            };
            let mut css = format!("color:{};", style.foreground.as_hex());
            if style.font_style.contains(FontStyle::BOLD) {
                css.push_str("font-weight:bold;");
            }
            if style.font_style.contains(FontStyle::ITALIC) {
                css.push_str("font-style:italic;");
            }
            if style.font_style.contains(FontStyle::UNDERLINE) {
                css.push_str("text-decoration:underline;");
            }
            if style.font_style.contains(FontStyle::STRIKETHROUGH) {
                css.push_str("text-decoration:line-through;");
            }
            write!(
                html,
                "<span style=\"{css}\">{}</span>",
                assets::escape(&token.text)
            )?;
        }
        html.push_str("</span>");
        if index + 1 < result.tokens.len() {
            html.push('\n');
        }
    }
    Ok(Highlighted {
        html,
        language,
        line_count,
    })
}
