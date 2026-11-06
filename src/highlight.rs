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
    options: &crate::Style,
) -> Result<Highlighted> {
    let language = snippet.language.id().to_owned();
    let theme_name = if theme.original && theme.syntax.len() > 1 {
        format!("original-{}-{mode}", theme.id)
    } else if theme.original {
        format!("original-{}", theme.id)
    } else {
        format!("editor-{}", theme.id)
    };
    let result = if let Some(diff) = &snippet.diff {
        diff.highlight(renderer, snippet.language, &theme_name, separate_comments)?
    } else {
        syntax::highlight(
            renderer,
            snippet.language,
            if snippet.language.is_terminal() {
                &snippet.source
            } else {
                source
            },
            &theme_name,
            separate_comments,
        )?
    };
    let default_background = result.background;
    let marked = highlighted_lines(snippet);
    let mut selections = snippet
        .text_highlights
        .iter()
        .map(|range| range.resolve(source))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    merge_ranges(&mut selections);
    let diff_rows = snippet
        .diff
        .as_ref()
        .map(|d| d.display().map(|v| v.0))
        .transpose()?;
    let rows: Vec<_> = diff_rows
        .as_ref()
        .map(|rows| {
            rows.iter()
                .filter(|r| !matches!(r.kind, crate::RowKind::Fold { .. }))
                .collect()
        })
        .unwrap_or_default();
    let number_digits = options
        .diff_numbers
        .largest(rows.iter().copied())
        .to_string()
        .len();
    let words = if options.diff_words {
        snippet
            .diff
            .as_ref()
            .map(|d| d.words())
            .transpose()?
            .unwrap_or_default()
    } else {
        vec![]
    };
    let mut comments = Vec::new();
    let mut html = String::new();
    let mut offset = 0;
    let line_count = source.split('\n').count();
    let mut run_index = 0;
    for (index, line) in source.split('\n').enumerate() {
        let selected = theme.selected_frame;
        let empty = selected && line.trim().is_empty();
        let class = if marked.contains(&(index + 1)) {
            "line highlighted-line"
        } else {
            "line"
        };
        let empty_class = if empty { " empty-line" } else { "" };
        let (diff_class, number, sign) = if let Some(row) = rows.get(index) {
            let number = match options.diff_numbers {
                crate::DiffNumbers::New => row.new.map(|n| n.to_string()).unwrap_or_default(),
                crate::DiffNumbers::Old => row.old.map(|n| n.to_string()).unwrap_or_default(),
                crate::DiffNumbers::Both => format!(
                    "{:>width$}  {:>width$}",
                    row.old.map(|n| n.to_string()).unwrap_or_default(),
                    row.new.map(|n| n.to_string()).unwrap_or_default(),
                    width = number_digits
                )
                .replace(' ', "\u{a0}"),
            };
            let (class, sign) = match row.kind {
                crate::RowKind::Added => (" diff-added", "+"),
                crate::RowKind::Removed => (" diff-removed", "−"),
                _ => (" diff-context", " "),
            };
            (class, number, sign)
        } else {
            ("", (index + snippet.first_line).to_string(), "")
        };
        write!(
            html,
            "<span class=\"{class}{empty_class}{diff_class}\" data-line=\"{}\" data-number=\"{number}\" data-sign=\"{sign}\">",
            index + 1
        )?;
        if options.diff_numbers == crate::DiffNumbers::Both
            && let Some(row) = rows.get(index)
        {
            write!(
                html,
                "<span class=\"diff-numbers\" aria-hidden=\"true\" data-old=\"{}\" data-new=\"{}\"></span>",
                row.old.map(|n| n.to_string()).unwrap_or_default(),
                row.new.map(|n| n.to_string()).unwrap_or_default()
            )?;
        }
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
            if theme.editor.is_some() || theme.original || snippet.language.is_terminal() {
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
                css.push_str(
                    if snippet.language.is_terminal()
                        && style.font_style.contains(FontStyle::UNDERLINE)
                    {
                        "text-decoration:underline line-through;"
                    } else {
                        "text-decoration:line-through;"
                    },
                );
            }
            let mut boundaries = vec![range.start, range.end];
            let selected_start = selections.partition_point(|r| r.end <= range.start);
            let changed_start = words.partition_point(|r| r.end <= range.start);
            for ranges in [&selections[selected_start..], &words[changed_start..]] {
                for r in ranges.iter().take_while(|r| r.start < range.end) {
                    boundaries.push(r.start.max(range.start));
                    boundaries.push(r.end.min(range.end));
                }
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            for pair in boundaries.windows(2) {
                let selected = selections
                    .get(selections.partition_point(|r| r.end <= pair[0]))
                    .is_some_and(|r| r.contains(&pair[0]));
                let changed = words
                    .get(words.partition_point(|r| r.end <= pair[0]))
                    .is_some_and(|r| r.contains(&pair[0]));
                if changed {
                    html.push_str("<span class=\"diff-word\">");
                }
                span(
                    &mut html,
                    &css,
                    &source[pair[0]..pair[1]],
                    selected,
                    snippet.language.is_terminal(),
                )?;
                if changed {
                    html.push_str("</span>");
                }
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
    if let Some(rows) = &diff_rows {
        let code_lines: Vec<_> = html.split('\n').collect();
        let mut next = 0;
        let mut folded = Vec::new();
        for row in rows {
            if let crate::RowKind::Fold { hidden } = row.kind {
                folded.push(format!("<span class=\"line diff-fold\" data-fold=\"⋯ {hidden} unchanged lines\"></span>"));
            } else {
                folded.push(code_lines.get(next).copied().unwrap_or("").to_owned());
                next += 1;
            }
        }
        html = folded.join("\n");
    }
    merge_ranges(&mut comments);
    Ok(Highlighted {
        html,
        language,
        line_count: if diff_rows.is_some() {
            rows.len()
        } else {
            source.split('\n').count()
        },
        comments,
    })
}

fn span(
    html: &mut String,
    css: &str,
    text: &str,
    selected: bool,
    terminal: bool,
) -> std::fmt::Result {
    let class = if selected {
        " class=\"highlighted-text\""
    } else {
        ""
    };
    let selected_css;
    let css = if selected && terminal {
        selected_css = css
            .split_inclusive(';')
            .filter(|declaration| !declaration.starts_with("background-color:"))
            .collect::<String>();
        &selected_css
    } else {
        css
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
