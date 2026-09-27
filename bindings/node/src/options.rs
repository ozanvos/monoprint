use crate::{Output, errors};
use monoprint::{Appearance, Font, LineNumbers, Renderer, Scale, Snippet, Style};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Options {
    format: Format,
    language: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    highlight: Option<Vec<Selection>>,
    theme: Option<String>,
    appearance: Option<ColorMode>,
    padding: Option<u32>,
    width: Option<u32>,
    transparent: Option<bool>,
    line_numbers: Option<Numbers>,
    scale: Option<u32>,
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
    let mut snippet = Snippet::new(source);
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
    if let Some(width) = options.width {
        style = style.width(width);
    }
    if let Some(transparent) = options.transparent {
        style = style.transparent(transparent);
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
                Scale::try_from(options.scale.unwrap_or(4)).map_err(errors::encode)?,
            )
            .map(Output::Png),
        Format::Svg => renderer.svg(&snippet, &style).map(Output::Text),
        Format::Html => renderer.html(&snippet, &style).map(Output::Text),
    }
    .map_err(errors::encode)
}
