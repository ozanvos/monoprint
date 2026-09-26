use crate::{assets, cli::Args};
use anyhow::{Result, bail};
use giallo::{FontStyle, HighlightOptions, Registry, ThemeVariant};
use std::fmt::Write;

pub struct Highlighted {
    pub html: String,
    pub language: String,
    pub line_count: usize,
}

pub fn language(args: &Args, registry: &Registry) -> Result<String> {
    let candidate = if let Some(value) = &args.language {
        value.to_ascii_lowercase()
    } else {
        let path = args.input.as_deref().unwrap_or(std::path::Path::new(""));
        match path.file_name().and_then(|p| p.to_str()).unwrap_or("") {
            "Dockerfile" | "Containerfile" => "dockerfile".into(),
            "Makefile" | "GNUmakefile" => "make".into(),
            _ => path
                .extension()
                .and_then(|p| p.to_str())
                .unwrap_or("plain")
                .to_ascii_lowercase(),
        }
    };
    let candidate = match candidate.as_str() {
        "typescript" | "ts" => "tsx",
        "js" => "javascript",
        "rs" => "rust",
        "py" => "python",
        "sh" | "bash" | "zsh" => "shellscript",
        "yml" => "yaml",
        "md" => "markdown",
        "h" => "c",
        "cs" => "csharp",
        "kt" => "kotlin",
        "ex" | "exs" => "elixir",
        "rb" => "ruby",
        "tex" => "latex",
        "txt" | "text" | "plaintext" => "plain",
        _ => &candidate,
    };
    if registry.contains_grammar(candidate) {
        Ok(candidate.to_owned())
    } else if args.language.is_some() {
        bail!("unknown language {candidate:?}; see --list-languages")
    } else {
        Ok("plain".into())
    }
}

pub fn code(args: &Args, source: &str, theme: &assets::Theme, mode: &str) -> Result<Highlighted> {
    let registry = Registry::load(include_bytes!("../assets/registry.zst"))?;
    let language = language(args, &registry)?;
    let theme_name = format!("ink-{}-{mode}", theme.id);
    let options =
        HighlightOptions::new(&language, ThemeVariant::Single(&theme_name)).merge_whitespace(false);
    let result = registry.highlight(source, &options)?;
    let line_count = source.split('\n').count();
    let marked = args.highlighted_lines(line_count)?;
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
