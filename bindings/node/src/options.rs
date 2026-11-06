use crate::{Output, errors};
use monoprint::{Appearance, Font, LineNumbers, PngOptions, Renderer, Scale, Snippet, Style};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Options {
    format: Format,
    diff: Option<DiffInput>,
    added: Option<Vec<Selection>>,
    removed: Option<Vec<Selection>>,
    first_line: Option<usize>,
    diff_numbers: Option<String>,
    diff_words: Option<bool>,
    language: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    highlight: Option<Vec<Selection>>,
    theme: Option<String>,
    appearance: Option<ColorMode>,
    padding: Option<u32>,
    font_size: Option<u32>,
    width: Option<u32>,
    transparent: Option<bool>,
    background: Option<String>,
    line_numbers: Option<Numbers>,
    scale: Option<u32>,
    compression: Option<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum DiffInput {
    Patch(String),
    Compare {
        base: String,
        context: Option<serde_json::Value>,
    },
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Format {
    Png,
    Svg,
    Html,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ColorMode {
    Dark,
    Light,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Selection {
    Line(u32),
    Range([u32; 2]),
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Numbers {
    Toggle(bool),
    Auto(Auto),
}
#[derive(Deserialize)]
enum Auto {
    #[serde(rename = "auto")]
    Auto,
}

pub fn render(
    renderer: &Renderer,
    source: &str,
    options: &str,
    font: Option<Font>,
) -> Result<Output, String> {
    let options: Options = serde_json::from_str(options)
        .map_err(|e| errors::internal(&format!("invalid native options: {e}")))?;
    let invalid =
        |message: &str| errors::encode(monoprint::InvalidOption::Diff(message.into()).into());
    if options.diff.is_some() && (options.added.is_some() || options.removed.is_some()) {
        return Err(invalid("conflicting diff forms"));
    }
    let mut inferred = None;
    let mut snippet = if let Some(diff) = options.diff {
        let diff = match diff {
            DiffInput::Patch(kind) if kind == "patch" => {
                monoprint::Diff::from_patch(source).map_err(errors::encode)?
            }
            DiffInput::Patch(_) => return Err(invalid("diff must be patch or a comparison")),
            DiffInput::Compare { base, context } => {
                let context = match context {
                    None => monoprint::Context::Lines(3),
                    Some(v) if v == "all" => monoprint::Context::All,
                    Some(v) => monoprint::Context::Lines(
                        v.as_u64()
                            .and_then(|n| usize::try_from(n).ok())
                            .ok_or_else(|| invalid("invalid context"))?,
                    ),
                };
                monoprint::Diff::compare(&base, source)
                    .map_err(errors::encode)?
                    .context(context)
            }
        };
        inferred = diff.path().and_then(monoprint::Language::for_file_name);
        Snippet::diff(diff)
    } else if options.added.is_some() || options.removed.is_some() {
        let mut diff = monoprint::Diff::marked(source).first_line(options.first_line.unwrap_or(1));
        for (marks, added) in [(options.added, true), (options.removed, false)] {
            for mark in marks.unwrap_or_default() {
                let range: monoprint::LineRange = match mark {
                    Selection::Line(n) => (n as usize).into(),
                    Selection::Range([a, b]) => (a as usize..=b as usize).into(),
                };
                diff = if added {
                    diff.added(range)
                } else {
                    diff.removed(range)
                };
            }
        }
        Snippet::diff(diff)
    } else {
        Snippet::new(source).first_line(options.first_line.unwrap_or(1))
    };
    if let Some(language) = inferred {
        snippet = snippet.language(language);
    }
    if let Some(language) = options.language {
        snippet = snippet.language(language.parse().map_err(errors::encode)?);
    }
    if let Some(title) = options.title {
        snippet = snippet.title(title);
    }
    if let Some(subtitle) = options.subtitle {
        snippet = snippet.subtitle(subtitle);
    }
    for selection in options.highlight.unwrap_or_default() {
        snippet = match selection {
            Selection::Line(line) => snippet.highlight(line as usize),
            Selection::Range([start, end]) => snippet.highlight(start as usize..=end as usize),
        };
    }
    let mut style = Style::default();
    if let Some(numbers) = options.diff_numbers {
        style = style.diff_numbers(numbers.parse().map_err(errors::encode)?);
    }
    if let Some(words) = options.diff_words {
        style = style.diff_words(words);
    }
    if let Some(theme) = options.theme {
        style = style.theme(theme.parse().map_err(errors::encode)?);
    }
    if let Some(appearance) = options.appearance {
        style = style.appearance(match appearance {
            ColorMode::Dark => Appearance::Dark,
            ColorMode::Light => Appearance::Light,
        });
    }
    if let Some(padding) = options.padding {
        style = style.padding(padding);
    }
    if let Some(font_size) = options.font_size {
        style = style.font_size(font_size);
    }
    if let Some(width) = options.width {
        style = style.width(width);
    }
    if let Some(transparent) = options.transparent {
        style = style.transparent(transparent);
    }
    if let Some(background) = options.background {
        style = style.background(background.parse().map_err(errors::encode)?);
    }
    if let Some(numbers) = options.line_numbers {
        style = style.line_numbers(match numbers {
            Numbers::Toggle(true) => LineNumbers::Show,
            Numbers::Toggle(false) => LineNumbers::Hide,
            Numbers::Auto(_) => LineNumbers::Auto,
        });
    }
    if let Some(font) = font {
        style = style.font(font);
    }
    match options.format {
        Format::Png => renderer
            .png(
                &snippet,
                &style,
                PngOptions::new(
                    Scale::try_from(options.scale.unwrap_or(4)).map_err(errors::encode)?,
                )
                .compression(
                    options
                        .compression
                        .as_deref()
                        .unwrap_or("fast")
                        .parse()
                        .map_err(errors::encode)?,
                ),
            )
            .map(Output::Png),
        Format::Svg => renderer.svg(&snippet, &style).map(Output::Text),
        Format::Html => renderer.html(&snippet, &style).map(Output::Text),
    }
    .map_err(errors::encode)
}
