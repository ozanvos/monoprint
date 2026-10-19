use crate::{Snippet, assets, highlighted_lines};
use anyhow::Result;
use giallo::{FontStyle, HighlightOptions, Registry, ThemeVariant};
use std::{fmt::Write, ops::Range};

pub struct Highlighted {
    pub html: String,
    pub language: String,
    pub line_count: usize,
    pub comments: Vec<Range<usize>>,
}

pub fn code(
    registry: &Registry,
    snippet: &Snippet,
    source: &str,
    theme: &assets::Theme,
    mode: &str,
    separate_comments: bool,
) -> Result<Highlighted> {
    let language = snippet.language.id().to_owned();
    let theme_name = if theme.original && theme.syntax.len() > 1 {
        format!("original-{}-{mode}", theme.id)
    } else if theme.original {
        format!("original-{}", theme.id)
    } else {
        format!("editor-{}", theme.id)
    };
    // The second palette classifies grammar scopes, independent of theme colors.
    // Dual highlighting shares one tokenization pass and keeps comment boundaries.
    let themes = if separate_comments {
        ThemeVariant::Dual {
            light: &*theme_name,
            dark: "monoprint-comments",
        }
    } else {
        ThemeVariant::Single(&*theme_name)
    };
    let options = HighlightOptions::new(&language, themes).merge_whitespace(false);
    let result = registry.highlight(source, &options)?;
    let default_background = match result.theme {
        ThemeVariant::Single(theme) => theme.default_style.background,
        ThemeVariant::Dual { light, .. } => light.default_style.background,
    };
    let marked = highlighted_lines(snippet);
    let mut selections = snippet
        .text_highlights
        .iter()
        .map(|range| range.resolve(source))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    merge_ranges(&mut selections);
    let comment_marker = giallo::Color::from_hex("#ffffff")?;
    let mut comments = Vec::new();
    let mut html = String::new();
    let mut offset = 0;
    for (index, tokens) in result.tokens.iter().enumerate() {
        let selected = theme.study_id.is_some();
        let empty = selected && tokens.iter().all(|token| token.text.trim().is_empty());
        let class = if marked.contains(&(index + 1)) {
            "line highlighted-line"
        } else {
            "line"
        };
        let empty_class = if empty { " empty-line" } else { "" };
        write!(
            html,
            "<span class=\"{class}{empty_class}\" data-line=\"{}\">",
            index + 1
        )?;
        if selected {
            html.push_str("<span class=\"lc\">");
        }
        let mut runs: Vec<(Range<usize>, giallo::Style, bool)> = Vec::new();
        for token in tokens {
            let (style, comment) = match token.style {
                ThemeVariant::Single(style) => (style, false),
                ThemeVariant::Dual { light, dark } => (light, dark.foreground == comment_marker),
            };
            let end = offset + token.text.len();
            if comment {
                comments.push(offset..end);
            }
            // Dual-theme mode does not merge tokens itself. Rejoin identical runs
            // so ligatures can shape across grammar tokens with the same style.
            if let Some((range, previous_style, previous_comment)) = runs.last_mut()
                && *previous_style == style
                && *previous_comment == comment
            {
                range.end = end;
            } else {
                runs.push((offset..end, style, comment));
            }
            offset = end;
        }
        for (range, style, comment) in runs {
            let foreground = style.foreground.as_hex();
            let mut css = format!("color:{foreground};");
            if theme.editor.is_some() || theme.original {
                if style.background != default_background {
                    write!(css, "background-color:{};", style.background.as_hex())?;
                }
                if theme.id == "synthwave-84" {
                    css.push_str(crate::editor_theme::glow(&foreground));
                }
            }
            if comment {
                css.push_str(
                    "font-family:var(--monoprint-comment-font,CommentFont);font-weight:400;",
                );
            }
            if style.font_style.contains(FontStyle::BOLD) {
                // Monoprint uses a restrained function-name weight in each
                // appearance; grammar scopes determine which tokens receive it.
                if let Some(weight) = theme.syntax_weight {
                    write!(css, "font-weight:{weight};")?;
                } else {
                    css.push_str(match (theme.id.as_str(), mode) {
                        ("monoprint", "dark") => "font-weight:560;",
                        ("monoprint", _) => "font-weight:600;",
                        _ => "font-weight:bold;",
                    });
                }
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
            let mut position = range.start;
            let first = selections.partition_point(|selection| selection.end <= position);
            for selection in &selections[first..] {
                if selection.start >= range.end {
                    break;
                }
                let start = selection.start.max(position);
                let end = selection.end.min(range.end);
                if position < start {
                    span(&mut html, &css, &source[position..start], false)?;
                }
                span(&mut html, &css, &source[start..end], true)?;
                position = end;
            }
            if position < range.end {
                span(&mut html, &css, &source[position..range.end], false)?;
            }
        }
        if selected {
            html.push_str("</span>");
        }
        html.push_str("</span>");
        if index + 1 < result.tokens.len() {
            html.push('\n');
            offset += 1;
        }
    }
    merge_ranges(&mut comments);
    Ok(Highlighted {
        html,
        language,
        line_count: source.split('\n').count(),
        comments,
    })
}

fn span(html: &mut String, css: &str, text: &str, selected: bool) -> std::fmt::Result {
    let class = if selected {
        " class=\"highlighted-text\""
    } else {
        ""
    };
    write!(
        html,
        "<span{class} style=\"{css}\">{}</span>",
        assets::escape(text)
    )
}

fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_unstable_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges.drain(..) {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    *ranges = merged;
}
