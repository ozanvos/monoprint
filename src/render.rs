use crate::{
    Error, LineNumbers, Renderer, Scale, Snippet, Style, TextRole, assets, background,
    editor_theme, effects, fallback::Fallbacks, highlight, originals, selected_styles,
};
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::sync::Arc;
use takumi::prelude::*;
use takumi_raster::RenderOptions;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Rgba,
    Bgra,
    Svg,
    Html,
}

impl Format {
    fn is_raster(self) -> bool {
        matches!(self, Self::Rgba | Self::Bgra)
    }
}

fn font_face(family: &str, bytes: &[u8]) -> String {
    format!(
        "@font-face{{font-family:'{family}';src:url(data:font/ttf;base64,{});font-weight:100 900;font-style:normal;}}",
        STANDARD.encode(bytes)
    )
}

// Substitute only the template itself. User text containing a marker is never
// interpreted as another marker or as an embedded asset URL.
fn substitute(template: &str, values: &[(&str, &str)]) -> String {
    let mut output = String::new();
    let mut rest = template;
    while let Some((position, key, value)) = values
        .iter()
        .filter_map(|(key, value)| rest.find(key).map(|p| (p, *key, *value)))
        .min_by_key(|x| x.0)
    {
        output.push_str(&rest[..position]);
        output.push_str(value);
        rest = &rest[position + key.len()..];
    }
    output.push_str(rest);
    output
}

fn code_width(
    font: &[u8],
    comment_font: Option<&[u8]>,
    comments: &[std::ops::Range<usize>],
    source: &str,
    font_size: u32,
) -> Result<f32> {
    let face = ttf_parser::Face::parse(font, 0)?;
    let comment_face = comment_font
        .map(|bytes| ttf_parser::Face::parse(bytes, 0))
        .transpose()?;
    let mut longest = 0.0_f32;
    let mut line_start = 0;
    let mut comment_index = 0;
    for line in source.split('\n') {
        let mut width = 0.0;
        let mut column = 0;
        for (offset, character) in line.char_indices() {
            let position = line_start + offset;
            while comments
                .get(comment_index)
                .is_some_and(|range| range.end <= position)
            {
                comment_index += 1;
            }
            let face = if comments
                .get(comment_index)
                .is_some_and(|range| range.contains(&position))
            {
                comment_face.as_ref().unwrap_or(&face)
            } else {
                &face
            };
            let unit = font_size as f32 / f32::from(face.units_per_em());
            let count = if character == '\t' { 2 - column % 2 } else { 1 };
            let glyph = face
                .glyph_index(if character == '\t' { ' ' } else { character })
                .context("missing glyph")?;
            width +=
                count as f32 * (f32::from(face.glyph_hor_advance(glyph).unwrap_or(0)) * unit + 0.1);
            column += count;
        }
        longest = longest.max(width);
        line_start += line.len() + 1;
    }
    Ok(longest.max(1.0))
}

// Measure complex text using the same shaper as the final image. Scalar glyph
// advances cannot account for emoji sequences, combining marks or script shaping.
fn shaped_code_width(
    code: &str,
    fonts: &Fonts,
    style: &Style,
    weight: u32,
    fallbacks: &str,
) -> Result<f32> {
    let tail = if fallbacks.is_empty() {
        String::new()
    } else {
        format!(",{fallbacks}")
    };
    let features = if style.ligatures {
        "'liga' 1,'clig' 1,'calt' 1"
    } else {
        "'liga' 0,'clig' 0,'calt' 0"
    };
    let css = format!(
        "pre{{display:inline-block;margin:0;padding:0;white-space:pre;tab-size:2;\
         font-family:CodeMono{tail};font-weight:{weight};font-size:{}px;line-height:{}px;\
         letter-spacing:0.1px;font-feature-settings:{features};\
         --monoprint-comment-font:CommentFont{tail}}}",
        style.font_size,
        f64::from(style.font_size) * 1.5,
    );
    let node = takumi::from_html(&format!("<pre>{code}</pre>"), FromHtmlOptions::default())?;
    Ok(takumi_raster::measure_size(
        RenderOptions::builder()
            .viewport(Viewport::new((None, None)))
            .node(node)
            .fonts(fonts)
            .stylesheet(Arc::new(StyleSheet::parse(&css)?))
            .build(),
    )?
    .width
    .max(1.0))
}

// Only transform trusted frame CSS, never user markup or source text. Insert
// named fallbacks before generic browser families to keep HTML deterministic.
fn with_fallback_families(css: &str, families: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("font-family:") {
        let start = start + "font-family:".len();
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest.find([';', '}']).unwrap_or(rest.len());
        let value = &rest[..end];
        if matches!(value.trim(), "inherit" | "initial" | "unset" | "revert") {
            result.push_str(value);
        } else if let Some((named, generic)) = value
            .rsplit_once(',')
            .filter(|(_, generic)| matches!(generic.trim(), "sans-serif" | "monospace" | "serif"))
        {
            result.push_str(&format!("{named},{families},{generic}"));
        } else {
            result.push_str(&format!("{value},{families}"));
        }
        rest = &rest[end..];
    }
    result.push_str(rest);
    result.push_str(&format!(
        "#capture{{--monoprint-comment-font:CommentFont,{families}}}"
    ));
    result
}

fn typography_css(font: &[u8], font_size: u32, line_count: usize) -> Result<String> {
    // Keep the original CSS (and exported bytes) untouched at the legacy frame default.
    if font_size == 15 {
        return Ok(String::new());
    }
    let ratio = f64::from(font_size) / 15.0;
    let rendered_size = f64::from(font_size) * 149.0 / 150.0;
    let line_height = f64::from(font_size) * 1.5;
    // the frame's gutter is specified in rem, so merely increasing font-size makes
    // large line numbers overlap code or extend outside the window. Keep its
    // proportions while also reserving room for every digit in long snippets.
    let face = ttf_parser::Face::parse(font, 0)?;
    let digit_advance = ('0'..='9')
        .filter_map(|digit| face.glyph_index(digit))
        .filter_map(|glyph| face.glyph_hor_advance(glyph))
        .max()
        .unwrap_or(0);
    let digit_width =
        f64::from(digit_advance) / f64::from(face.units_per_em()) * rendered_size + 0.1;
    let number_width = (16.0 * ratio).max(line_count.to_string().len() as f64 * digit_width);
    let number_gap = 24.0 * ratio;
    let number_margin = number_width + number_gap;
    let number_padding = number_margin + 8.0 * ratio;
    let number_offset = 6.0 * ratio;
    Ok(format!(
        ".Editor__formatted{{font-size:{rendered_size}px;line-height:{line_height}px}}\
         .Editor__editor .line{{min-height:{line_height}px}}\
         .Editor__editor.Editor__showLineNumbers{{--line-number-padding:{number_padding}px}}\
         .Editor__editor.Editor__showLineNumbers .line:before{{width:{number_width}px;margin-right:{number_gap}px;margin-left:-{number_margin}px;left:0;transform:translateX({number_offset}px)}}"
    ))
}

pub(crate) enum Output {
    Raster(image::RgbaImage),
    Text(String),
}

pub(crate) fn render(
    renderer: &Renderer,
    snippet: &Snippet,
    style: &Style,
    source: &str,
    format: Format,
    scale: Scale,
) -> Result<Output> {
    let theme = style
        .theme
        .data_for(style.appearance)
        .expect("validated theme appearance");
    let mode = style.appearance.as_str();
    let palette = &theme.syntax[mode];
    let font = style
        .font
        .as_ref()
        .map(|font| font.bytes())
        .unwrap_or_else(|| assets::font(&theme.font));
    let code = highlight::code(
        renderer.registry()?,
        snippet,
        source,
        theme,
        mode,
        style.comment_font.is_some(),
    )?;
    let mut fallbacks = Fallbacks::default();
    if let Some(comment_font) = &style.comment_font {
        let mut offset = 0;
        for range in &code.comments {
            fallbacks.include(font, &source[offset..range.start], TextRole::Code, true)?;
            fallbacks.include(
                comment_font.bytes(),
                &source[range.clone()],
                TextRole::Comment,
                false,
            )?;
            offset = range.end;
        }
        fallbacks.include(font, &source[offset..], TextRole::Code, true)?;
    } else {
        fallbacks.include(font, source, TextRole::Code, true)?;
    }
    let numbers = code.language != "plain"
        && match style.line_numbers {
            LineNumbers::Auto => theme.line_numbers,
            LineNumbers::Show => true,
            LineNumbers::Hide => false,
        };
    let weight = if theme.editor.is_none()
        && !theme.original
        && style.font.is_none()
        && matches!(theme.font.as_str(), "jetbrains-mono" | "ibm-plex-mono")
    {
        500
    } else {
        400
    };
    let palette_css = palette
        .iter()
        .map(|(k, v)| format!("{k}:{v};"))
        .collect::<String>();
    let number_class = if numbers {
        if code.line_count > 8 {
            " Editor__showLineNumbers Editor__showLineNumbersLarge"
        } else {
            " Editor__showLineNumbers"
        }
    } else {
        ""
    };
    let editor = format!(
        "<div class=\"Editor__editor{number_class}\" style=\"--editor-padding:16px;--code-weight:{weight};font-family:CodeMono;font-weight:{weight};{palette_css}\"><div class=\"Editor__formatted\"><pre><code>{}</code></pre></div></div>",
        code.html
    );
    let key = format!(
        "{}/{mode}/{}/{}/{}",
        theme.id,
        u8::from(style.background == crate::Background::Theme),
        u8::from(!snippet.title.is_empty()),
        u8::from(!snippet.subtitle.is_empty())
    );
    let language_name = assets::grammars()
        .iter()
        .find(|g| g.name == code.language || g.aliases.contains(&code.language))
        .map(|g| g.display_name.as_str())
        .unwrap_or("Plain Text");
    let original = if theme.study_id.is_some() {
        Some(selected_styles::frame(theme, style, code.line_count))
    } else {
        theme
            .original
            .then(|| originals::frame(theme, style, snippet, source))
    };
    let mut template = if let Some(frame) = &original {
        frame.template.clone()
    } else if theme.editor.is_some() {
        editor_theme::template(!snippet.title.is_empty(), style.background)
    } else {
        assets::templates()
            .get(&key)
            .context("missing embedded frame template")?
            .clone()
    };
    if template.contains("__TITLE__") {
        let title_uses_code_font = matches!(
            theme.id.as_str(),
            "removed-frame" | "removed-frame" | "thermal-receipt" | "monoprint"
        );
        let title_font = match theme.id.as_str() {
            "removed-frame" | "removed-frame" | "thermal-receipt" | "monoprint" => font,
            "removed-frame" => assets::font("ibm-plex-mono"),
            _ => assets::font("inter"),
        };
        fallbacks.include(
            title_font,
            &snippet.title,
            TextRole::Title,
            title_uses_code_font,
        )?;
    }
    // The pinned Auth0 frame keeps subtitle only in data-value metadata; its
    // visible span is the language label. No subtitle glyphs are drawn.
    if matches!(theme.id.as_str(), "removed-frame" | "removed-frame" | "removed-frame")
        && let Some(start) = template.find("background-image:linear-gradient(")
        && let Some(end) = template[start..].find(')')
    {
        template.replace_range(start..start + end + 1, "");
    }
    let width = style
        .width
        .map(|w| format!("width:{w}px"))
        .unwrap_or_else(|| "display:inline-block;min-width:520px".into());
    let editor_css;
    let base_css = if let Some(frame) = &original {
        &frame.css
    } else if theme.editor.is_some() {
        editor_css = editor_theme::css(theme);
        &editor_css
    } else {
        &assets::styles()[&theme.id]
    };
    // html-to-image 1.11.13 reduces 15px code to 14.9px on export.
    // Match that in every format, retaining 22.5px leading and the original
    // pre-export fit-width measurement below.
    let mut css = format!(
        "{base_css}\n#capture{{{width}}}.Editor__editor{{display:block}}.Editor__editor:after{{content:none}}.Editor__formatted{{font-family:CodeMono;font-size:14.9px;font-variant-ligatures:none;font-feature-settings:'liga' 0,'calt' 0}}[data-export-input]{{display:block}}.DefaultFrame__header .DefaultFrame__fileName>[data-export-input]{{display:flex;align-items:center;justify-content:center}}\n"
    );
    css += &background::css(theme, style.appearance, style.background);
    if style.ligatures {
        // Toggle shaping features without changing the theme's tracking. Plain
        // text must retain its glyph positions, wrapping and automatic width.
        css += ".Editor__formatted{font-variant-ligatures:common-ligatures contextual;font-feature-settings:'liga' 1,'clig' 1,'calt' 1}";
    }
    if !snippet.text_highlights.is_empty() {
        css += ".Editor__editor .highlighted-text{background-color:var(--ink-highlight,rgba(128,128,128,0.28))}";
    }
    // All image URLs are resolved from the embedded asset bundle; no network access.
    for (name, data) in assets::images() {
        template = template.replace(&format!("asset:{name}"), data);
        css = css.replace(&format!("asset:{name}"), data);
    }
    let secondary_font = matches!(theme.id.as_str(), "removed-frame" | "removed-frame");
    let fallback_families = fallbacks.families();
    if style.width.is_none() {
        let measured_width = if !fallbacks.is_empty() || !source.is_ascii() {
            let fonts = renderer.fonts(style, secondary_font, fallbacks)?;
            shaped_code_width(&code.html, &fonts, style, weight, &fallback_families)?
        } else {
            code_width(
                font,
                style.comment_font.as_ref().map(|font| font.bytes()),
                &code.comments,
                source,
                style.font_size,
            )?
        };
        css += &format!(
            ".Editor__formatted>pre{{width:{}px}}",
            measured_width.ceil()
        );
    }
    let fill_template = |template: &str| {
        substitute(
            template,
            &[
                ("__CODE_IMAGES_EDITOR__", &editor),
                ("__PADDING__", &style.padding.to_string()),
                ("__TITLE__", &assets::escape(&snippet.title)),
                ("__SUBTITLE__", &assets::escape(&snippet.subtitle)),
                ("__LANGUAGE__", &assets::escape(language_name)),
                (
                    "__LANGUAGE_SHORT__",
                    &assets::escape(&code.language.to_uppercase()),
                ),
                ("__LINE_COUNT__", &code.line_count.to_string()),
                (
                    "__LINE_UNIT__",
                    if code.line_count == 1 {
                        "line"
                    } else {
                        "lines"
                    },
                ),
            ],
        )
    };
    let frame = fill_template(&template);
    let mut html = format!("<div id=\"capture\" data-theme=\"{mode}\">{frame}</div>");
    css += "#capture{--font-space-mono:CodeMono;--font-soehne-mono:CodeMono}.TriggerdevFrame__fileName,.TriggerdevFrame__language{font-family:CodeMono}.Editor__editor .line{min-height:22.5px}";
    css += &typography_css(font, style.font_size, code.line_count)?;
    if !fallbacks.is_empty() {
        css = with_fallback_families(&css, &fallback_families);
    }
    let parse_css = |css: &str| -> Result<Arc<StyleSheet>> {
        Ok(Arc::new(
            StyleSheet::parse(&effects::native_css(css, format.is_raster())).map_err(|error| {
                let message = error.to_string();
                let tail: String = message
                    .chars()
                    .rev()
                    .take(500)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                anyhow::anyhow!("parsing {} frame styles: {tail}", theme.id)
            })?,
        ))
    };
    let mut stylesheet = parse_css(&css)?;
    let mut node =
        takumi::from_html(&html, FromHtmlOptions::default()).context("parsing frame markup")?;
    if effects::needs_measurement(&theme.id) || theme.id == "dither-light" {
        let fonts = renderer.fonts(style, secondary_font, fallbacks)?;
        let viewport =
            Viewport::new((style.width.map(|w| w * 2), None)).with_device_pixel_ratio(2.0);
        let size = takumi_raster::measure_size(
            RenderOptions::builder()
                .viewport(viewport)
                .node(node.clone())
                .fonts(&fonts)
                .stylesheet(stylesheet.clone())
                .build(),
        )?;
        let effect = effects::prepare(&theme.id, style, size.width / 2.0, size.height / 2.0);
        css += &effect.css;
        css += &originals::measured_css(style, size.width / 2.0, size.height / 2.0);
        // Adapt only trusted template markup, before inserting user text.
        let template =
            effects::adapt_template(&theme.id, template, size.width / 2.0, size.height / 2.0);
        let frame = fill_template(&template);
        html = format!(
            "<div id=\"capture\" data-theme=\"{mode}\">{frame}{}</div>",
            effect.overlay
        );
        node = takumi::from_html(&html, FromHtmlOptions::default())?;
        stylesheet = parse_css(&css)?;
    }
    if format == Format::Html {
        let mut fonts = font_face("CodeMono", font) + &font_face("Inter", assets::font("inter"));
        if let Some(font) = &style.comment_font {
            fonts += &font_face("CommentFont", font.bytes());
        }
        if secondary_font {
            fonts += &font_face("IBM Plex Mono", assets::font("ibm-plex-mono"));
        }
        for (family, bytes) in fallbacks.entries() {
            fonts += &font_face(family, bytes);
        }
        let result = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{fonts}{css}</style></head><body>{html}</body></html>",
            assets::escape(&snippet.title)
        );
        return Ok(Output::Text(result));
    }
    let fonts = renderer.fonts(style, secondary_font, fallbacks)?;
    // At 2x, the upstream 22.5px line height is integral. Keep the SVG's
    // viewBox at this precision and expose its normal CSS dimensions.
    let raster_scale = scale;
    let scale = if format.is_raster() {
        if u32::from(scale).is_multiple_of(2) {
            u32::from(scale)
        } else {
            u32::from(scale) * 2
        }
    } else {
        2
    } as f32;
    let mut viewport = Viewport::new((style.width.map(|w| (w as f32 * scale) as u32), None))
        .with_device_pixel_ratio(scale);
    let size = takumi_raster::measure_size(
        RenderOptions::builder()
            .viewport(viewport)
            .node(node.clone())
            .fonts(&fonts)
            .stylesheet(stylesheet.clone())
            .build(),
    )?;
    let width = (size.width / scale).ceil() * scale;
    let height = (size.height / scale).ceil() * scale;
    // Takumi 2.14's raster canvas and SVG-source rasterizer each cap pixmaps
    // at 16 Mi pixels. Check here so callers get our actionable typed error
    // rather than an opaque backend failure. Vector output keeps its budget.
    let pixel_budget = if format.is_raster() {
        16 << 20
    } else {
        64_000_000
    };
    if u64::from(width as u32) * u64::from(height as u32) > pixel_budget
        || width > 32768.0
        || height > 32768.0
    {
        return Err(Error::ImageTooLarge {
            width: width as u32,
            height: height as u32,
            supersampled: format.is_raster() && !u32::from(raster_scale).is_multiple_of(2),
        }
        .into());
    }
    viewport =
        Viewport::new((Some(width as u32), Some(height as u32))).with_device_pixel_ratio(scale);
    match format {
        Format::Rgba | Format::Bgra => {
            let options = RenderOptions::builder()
                .viewport(viewport)
                .node(node)
                .fonts(&fonts)
                .stylesheet(stylesheet)
                .build();
            let (width, height, pixels) = if format == Format::Bgra {
                let bitmap = takumi_raster::render_bgra(options).context("rendering BGRA image")?;
                (bitmap.width, bitmap.height, bitmap.pixels)
            } else {
                let bitmap = takumi_raster::render(options).context("rendering RGBA image")?;
                (bitmap.width(), bitmap.height(), bitmap.into_raw())
            };
            // Internal four-channel container; BGRA stays BGRA. The resampler
            // applies identical arithmetic independently to each channel.
            let mut image = image::RgbaImage::from_raw(width, height, pixels)
                .context("invalid rendered bitmap")?;
            if !u32::from(raster_scale).is_multiple_of(2) {
                image = crate::resample::half(&image);
            }
            Ok(Output::Raster(image))
        }
        Format::Svg => {
            let svg = takumi_svg::render(
                takumi_svg::SvgOptions::builder()
                    .viewport(viewport)
                    .node(node)
                    .fonts(&fonts)
                    .stylesheet(stylesheet)
                    .build(),
            )
            .context("rendering SVG")?;
            Ok(Output::Text(
                svg.replacen(
                    &format!("width=\"{width}\""),
                    &format!("width=\"{}\"", width / 2.0),
                    1,
                )
                .replacen(
                    &format!("height=\"{height}\""),
                    &format!("height=\"{}\"", height / 2.0),
                    1,
                ),
            ))
        }
        Format::Html => unreachable!(),
    }
}
