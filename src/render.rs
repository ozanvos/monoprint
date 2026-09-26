use crate::{
    assets,
    cli::{Args, Format},
    effects, highlight,
};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{borrow::Cow, fs, sync::Arc};
use takumi::prelude::*;

fn validate_glyphs(bytes: &[u8], text: &str, label: &str) -> Result<()> {
    let face = ttf_parser::Face::parse(bytes, 0).context("font must be a valid TTF or OTF file")?;
    for c in text.chars() {
        if c == '\n' || c == '\t' || c == '\r' {
            continue;
        }
        if face.glyph_index(c).is_none() {
            bail!(
                "{label} font cannot display {c:?} (U+{:04X}); use --font-file with a font containing this glyph",
                c as u32
            );
        }
    }
    Ok(())
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

fn code_width(font: &[u8], source: &str) -> Result<f32> {
    let face = ttf_parser::Face::parse(font, 0)?;
    let unit = 15.0 / f32::from(face.units_per_em());
    let mut longest = 0.0_f32;
    for line in source.split('\n') {
        let mut width = 0.0;
        let mut column = 0;
        for character in line.chars() {
            let count = if character == '\t' { 2 - column % 2 } else { 1 };
            let glyph = face
                .glyph_index(if character == '\t' { ' ' } else { character })
                .context("missing glyph")?;
            width +=
                count as f32 * (f32::from(face.glyph_hor_advance(glyph).unwrap_or(0)) * unit + 0.1);
            column += count;
        }
        longest = longest.max(width);
    }
    Ok(longest.max(1.0))
}

pub fn export(args: &Args, source: &str, format: Format) -> Result<Vec<u8>> {
    let theme = assets::themes()
        .get(&args.theme.to_ascii_lowercase())
        .with_context(|| format!("unknown theme {:?}; see --list-themes", args.theme))?;
    let mode = if args.light { "light" } else { "dark" };
    let palette = theme
        .syntax
        .get(mode)
        .with_context(|| format!("{} has no {mode} variant", theme.name))?;
    if args
        .width
        .is_some_and(|width| width <= args.padding * 2 + 48)
    {
        bail!(
            "width must leave room for the code after padding (at least {}px)",
            args.padding * 2 + 49
        );
    }
    if !args.subtitle.is_empty() && theme.id != "removed-frame" {
        bail!("--subtitle is supported by the Auth0 theme");
    }
    let font: Cow<'_, [u8]> = if let Some(path) = &args.font_file {
        Cow::Owned(fs::read(path).with_context(|| format!("reading font {}", path.display()))?)
    } else {
        Cow::Borrowed(assets::font(&theme.font))
    };
    validate_glyphs(&font, source, "code")?;
    validate_glyphs(
        assets::font("inter"),
        &format!("{}{}", args.title, args.subtitle),
        "title",
    )?;
    let code = highlight::code(args, source, theme, mode)?;
    let numbers = code.language != "plain"
        && if theme.partner && !theme.line_numbers_toggleable {
            theme.line_numbers
        } else if args.no_line_numbers {
            false
        } else {
            args.line_numbers || theme.line_numbers
        };
    let weight = if args.font_file.is_none()
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
        "<div class=\"Editor__editor{number_class}\" style=\"--editor-padding:16px;font-family:CodeMono;font-weight:{weight};{palette_css}\"><div class=\"Editor__formatted\"><pre><code>{}</code></pre></div></div>",
        code.html
    );
    let key = format!(
        "{}/{mode}/{}/{}/{}",
        theme.id,
        u8::from(!args.transparent),
        u8::from(!args.title.is_empty()),
        u8::from(!args.subtitle.is_empty())
    );
    let language_name = assets::grammars()
        .iter()
        .find(|g| g.name == code.language || g.aliases.contains(&code.language))
        .map(|g| g.display_name.as_str())
        .unwrap_or("Plain Text");
    let mut template = assets::templates()
        .get(&key)
        .context("missing embedded frame template")?
        .clone();
    if matches!(theme.id.as_str(), "removed-frame" | "removed-frame" | "removed-frame")
        && let Some(start) = template.find("background-image:linear-gradient(")
        && let Some(end) = template[start..].find(')')
    {
        template.replace_range(start..start + end + 1, "");
    }
    let width = args
        .width
        .map(|w| format!("width:{w}px"))
        .unwrap_or_else(|| "display:inline-block;min-width:520px".into());
    let base_css = &assets::styles()[&theme.id];
    let mut css = format!(
        "{base_css}\n#capture{{{width}}}.Editor__editor{{display:block}}.Editor__editor:after{{content:none}}.Editor__formatted{{font-family:CodeMono;font-variant-ligatures:none;font-feature-settings:'liga' 0,'calt' 0}}[data-export-input]{{display:block}}\n"
    );
    // All image URLs are resolved from the embedded asset bundle; no network access.
    for (name, data) in assets::images() {
        template = template.replace(&format!("asset:{name}"), data);
        css = css.replace(&format!("asset:{name}"), data);
    }
    if args.width.is_none() {
        css += &format!(
            ".Editor__formatted>pre{{width:{}px}}",
            code_width(&font, source)?.ceil()
        );
    }
    let frame = substitute(
        &template,
        &[
            ("__CODE_IMAGES_EDITOR__", &editor),
            ("__PADDING__", &args.padding.to_string()),
            ("__TITLE__", &assets::escape(&args.title)),
            ("__SUBTITLE__", &assets::escape(&args.subtitle)),
            ("__LANGUAGE__", &assets::escape(language_name)),
        ],
    );
    let mut html = format!("<div id=\"capture\" data-theme=\"{mode}\">{frame}</div>");
    css += "#capture{--font-space-mono:CodeMono;--font-soehne-mono:CodeMono}.TriggerdevFrame__fileName,.TriggerdevFrame__language{font-family:CodeMono}.Editor__editor .line{min-height:22.5px}";
    let mut fonts = Fonts::default();
    fonts.register(
        FontResource::new(font.as_ref()).override_info(FontOverride {
            family_name: Some(Arc::from("CodeMono")),
            ..Default::default()
        }),
    )?;
    fonts.register(
        FontResource::new(assets::font("inter")).override_info(FontOverride {
            family_name: Some(Arc::from("Inter")),
            ..Default::default()
        }),
    )?;
    let secondary_font = matches!(theme.id.as_str(), "removed-frame" | "removed-frame");
    if secondary_font {
        fonts.register(
            FontResource::new(assets::font("ibm-plex-mono")).override_info(FontOverride {
                family_name: Some(Arc::from("IBM Plex Mono")),
                ..Default::default()
            }),
        )?;
    }
    let parse_css = |css: &str| -> Result<Arc<StyleSheet>> {
        Ok(Arc::new(
            StyleSheet::parse(&effects::native_css(css)).map_err(|error| {
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
    if effects::needs_measurement(&theme.id) {
        let viewport =
            Viewport::new((args.width.map(|w| w * 2), None)).with_device_pixel_ratio(2.0);
        let size = takumi::measure(
            RenderOptions::builder()
                .viewport(viewport)
                .node(node.clone())
                .fonts(&fonts)
                .stylesheet(stylesheet.clone())
                .build(),
        )?;
        let effect = effects::prepare(&theme.id, args, size.width / 2.0, size.height / 2.0);
        css += &effect.css;
        let frame = effects::adapt_frame(&theme.id, frame, size.width / 2.0, size.height / 2.0);
        html = format!(
            "<div id=\"capture\" data-theme=\"{mode}\">{frame}{}</div>",
            effect.overlay
        );
        node = takumi::from_html(&html, FromHtmlOptions::default())?;
        stylesheet = parse_css(&css)?;
    }
    if format == Format::Html {
        let mut fonts = font_face("CodeMono", &font) + &font_face("Inter", assets::font("inter"));
        if secondary_font {
            fonts += &font_face("IBM Plex Mono", assets::font("ibm-plex-mono"));
        }
        let result = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{fonts}{css}</style></head><body>{html}</body></html>",
            assets::escape(&args.title)
        );
        return Ok(result.into_bytes());
    }
    // At 2x, the upstream 22.5px line height is integral. Keep the SVG's
    // viewBox at this precision and expose its normal CSS dimensions.
    let scale = if format == Format::Png {
        if args.scale.is_multiple_of(2) {
            args.scale
        } else {
            args.scale * 2
        }
    } else {
        2
    } as f32;
    let mut viewport = Viewport::new((args.width.map(|w| (w as f32 * scale) as u32), None))
        .with_device_pixel_ratio(scale);
    let size = takumi::measure(
        RenderOptions::builder()
            .viewport(viewport)
            .node(node.clone())
            .fonts(&fonts)
            .stylesheet(stylesheet.clone())
            .build(),
    )?;
    let width = (size.width / scale).ceil() * scale;
    let height = (size.height / scale).ceil() * scale;
    if width * height > 64_000_000.0 || width > 32768.0 || height > 32768.0 {
        bail!(
            "image is too large ({}x{} pixels); reduce --scale, --width, or snippet length",
            width,
            height
        );
    }
    viewport =
        Viewport::new((Some(width as u32), Some(height as u32))).with_device_pixel_ratio(scale);
    match format {
        Format::Png => {
            let bitmap = takumi::render(
                RenderOptions::builder()
                    .viewport(viewport)
                    .node(node)
                    .fonts(&fonts)
                    .stylesheet(stylesheet)
                    .build(),
            )
            .context("rendering PNG")?;
            let mut image =
                image::RgbaImage::from_raw(bitmap.width(), bitmap.height(), bitmap.into_raw())
                    .context("invalid rendered bitmap")?;
            if !args.scale.is_multiple_of(2) {
                image = image::imageops::resize(
                    &image,
                    image.width() / 2,
                    image.height() / 2,
                    image::imageops::FilterType::Lanczos3,
                );
            }
            let mut data = Vec::new();
            // Fast lossless PNG compression avoids spending most of a fresh
            // invocation in the renderer's default level-7/no-filter encoder.
            let mut encoder = png::Encoder::new(&mut data, image.width(), image.height());
            let opaque = image
                .as_raw()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255);
            let pixels = if opaque {
                let mut rgb = Vec::with_capacity(image.as_raw().len() / 4 * 3);
                for pixel in image.as_raw().as_chunks::<4>().0 {
                    rgb.extend_from_slice(&pixel[..3]);
                }
                Cow::Owned(rgb)
            } else {
                Cow::Borrowed(image.as_raw().as_slice())
            };
            encoder.set_color(if opaque {
                png::ColorType::Rgb
            } else {
                png::ColorType::Rgba
            });
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Fast);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&pixels)?;
            writer.finish()?;
            Ok(data)
        }
        Format::Svg => {
            let svg = takumi::render_svg(
                SvgOptions::builder()
                    .viewport(viewport)
                    .node(node)
                    .fonts(&fonts)
                    .stylesheet(stylesheet)
                    .build(),
            )
            .context("rendering SVG")?;
            Ok(svg
                .replacen(
                    &format!("width=\"{width}\""),
                    &format!("width=\"{}\"", width / 2.0),
                    1,
                )
                .replacen(
                    &format!("height=\"{height}\""),
                    &format!("height=\"{}\"", height / 2.0),
                    1,
                )
                .into_bytes())
        }
        Format::Html => unreachable!(),
    }
}
