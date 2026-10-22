use crate::{Renderer, Snippet, assets, highlighted_lines, syntax};
use anyhow::Result;
use giallo::FontStyle;
use std::{fmt::Write, ops::Range};

pub struct Highlighted {
    pub html: String,
    pub language: String,
    pub line_count: usize,
    pub comments: Vec<Range<usize>>,
}

pub fn code(
    renderer: &Renderer,
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
    let result = syntax::highlight(
        renderer,
        snippet.language,
        source,
        &theme_name,
        separate_comments,
    )?;
    let default_background = result.background;
    let marked = highlighted_lines(snippet);
    let mut selections = snippet
        .text_highlights
        .iter()
        .map(|range| range.resolve(source))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    merge_ranges(&mut selections);
    let mut comments = Vec::new();
    let mut html = String::new();
    let mut offset = 0;
    let line_count = source.split('\n').count();
    let mut run_index = 0;
    for (index, line) in source.split('\n').enumerate() {
        let selected = theme.study_id.is_some();
        let empty = selected && line.trim().is_empty();
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
        let line_end = offset + line.len();
        while let Some(run) = result.runs.get(run_index) {
            if run.range.start >= line_end {
                break;
            }
            let range = run.range.start.max(offset)..run.range.end.min(line_end);
            if !range.is_empty() {
                if run.comment {
                    comments.push(range.clone());
                }
                runs.push((range, run.style, run.comment));
            }
            if run.range.end > line_end {
                break;
            }
            run_index += 1;
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
        offset = line_end;
        if index + 1 < line_count {
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
