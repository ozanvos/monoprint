//! Monoprint's own compositions. Layout stays in normal flow so decorations
//! follow wrapped code, custom fonts and arbitrarily long snippets in every format.
use crate::{Background, Snippet, Style, assets};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::fmt::Write;

pub(crate) struct Frame {
    pub template: String,
    pub css: String,
}

pub(crate) fn frame(
    theme: &assets::Theme,
    style: &Style,
    snippet: &Snippet,
    source: &str,
) -> Frame {
    if theme.id == "monoprint" {
        return crate::monoprint_theme::frame(theme, style, source.split('\n').count());
    }
    let palette = &theme.syntax[style.appearance.as_str()];
    let surface = theme.surface.as_deref().expect("original reading surface");
    let outer = &theme.background.as_ref().expect("original backdrop").from;
    let available = style.width.map(|w| w - 2 * style.padding).unwrap_or(392);
    // Preserve a reading column even at the narrowest accepted canvas width.
    let inset = (style.font_size as f32 * 1.5).min(available as f32 * 0.06);
    let label = if snippet.title.is_empty() {
        String::new()
    } else {
        "<div class=\"Original__title\">__TITLE__</div>".into()
    };
    let editor = "__CODE_IMAGES_EDITOR__";
    let mut extra = String::new();
    let content = match theme.id.as_str() {
        "single-ink" => format!(
            "<div class=\"Original__paper\"><i class=\"Ink__mark\"></i><div class=\"Ink__body\">{editor}</div><div class=\"Ink__foot\">{label}</div></div>"
        ),
        "swatch-card" => {
            let mut swatches = String::new();
            for (role, key) in [
                ("ink", "--ink-foreground"),
                ("keyword", "--ink-token-keyword"),
                ("type", "--ink-token-type"),
                ("string", "--ink-token-string"),
                ("function", "--ink-token-function"),
                ("comment", "--ink-token-comment"),
                ("number", "--ink-token-number"),
                ("field", ""),
            ] {
                let color = if key.is_empty() {
                    surface
                } else {
                    &palette[key]
                };
                write!(swatches, "<div class=\"Swatch__item\"><i style=\"background:{color}\"></i><span>{role}</span><b>{}</b></div>", color.to_uppercase()).unwrap();
            }
            format!(
                "<div class=\"Swatch__stack\"><div class=\"Swatch__back\"></div><div class=\"Original__paper\"><div class=\"Swatch__field\"><i class=\"Swatch__hole\"></i>{label}{editor}</div><div class=\"Swatch__band\">{swatches}</div></div></div>"
            )
        }
        "wayfinding" => {
            // Stops identify real source-line groups, never invented declarations.
            let mut start = true;
            let mut group = 0;
            for (i, line) in source.split('\n').enumerate() {
                if line.trim().is_empty() {
                    start = true;
                    continue;
                }
                if start {
                    group += 1;
                }
                let color = if group % 2 == 1 { "#f4b400" } else { "#2ec4a0" };
                write!(
                    extra,
                    ".Original .line[data-line=\"{}\"]{{border-left-color:{color}}}",
                    i + 1
                )
                .unwrap();
                if start {
                    write!(extra, ".Original .line[data-line=\"{}\"]:after{{content:\"{}\";background:{color}}}", i+1, i+1).unwrap();
                }
                start = false;
            }
            format!(
                "<div class=\"Original__paper\">{label}<div class=\"Wayfinding__route\">{editor}</div></div>"
            )
        }
        "highlighter-pass" => format!(
            "<div class=\"Original__paper\"><div class=\"Highlighter__body\">{editor}</div>{label}</div>"
        ),
        "gallery-wall" => format!(
            "<div class=\"Gallery__lamp\"></div><div class=\"Gallery__frame\"><div class=\"Gallery__mat\"><div class=\"Original__paper\">{editor}</div></div></div><div class=\"Gallery__label\">{label}<div>__LANGUAGE__</div></div>"
        ),
        "departure-board" => format!(
            "<div class=\"Departure__housing\"><i class=\"Departure__screw tl\"></i><i class=\"Departure__screw tr\"></i><i class=\"Departure__screw bl\"></i><i class=\"Departure__screw br\"></i><div class=\"Departure__lamps\"><i></i><i></i><i></i></div>{label}<div class=\"Original__paper\">{editor}</div></div>"
        ),
        "dither-light" => format!("<div class=\"Original__paper\">{editor}{label}</div>"),
        "cutting-mat" => format!(
            "<div class=\"Cutting__card\"><i class=\"Cutting__top\"></i><i class=\"Cutting__left\"></i><div class=\"Original__paper\">{editor}{label}</div></div>"
        ),
        "thermal-receipt" => {
            let tail = svg_url(
                "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='10'><path d='M0 0H16L8 10Z' fill='#f5f2eb'/></svg>",
            );
            write!(
                extra,
                ".Receipt__tail{{background-image:url({tail});background-size:16px 10px}}"
            )
            .unwrap();
            format!(
                "<div class=\"Receipt__strip\"><div class=\"Receipt__slot\"></div><div class=\"Original__paper\"><div class=\"Receipt__head\">{label}</div><div class=\"Receipt__body\">{editor}</div><div class=\"Receipt__foot\">__LANGUAGE__</div></div><div class=\"Receipt__tail\"></div></div>"
            )
        }
        "proof-marks" => format!(
            "<div class=\"Original__paper\"><i class=\"Proof__crop tl\"></i><i class=\"Proof__crop tr\"></i><i class=\"Proof__crop bl\"></i><i class=\"Proof__crop br\"></i><div class=\"Proof__body\">{editor}</div>{label}</div>"
        ),
        _ => unreachable!("bundled original"),
    };
    if matches!(theme.id.as_str(), "highlighter-pass" | "proof-marks") {
        let marked = crate::highlighted_lines(snippet);
        // Explicit selections win. Otherwise emphasize the final substantive
        // source line, ignoring closing delimiters and trailing blank lines.
        let marked = if marked.is_empty() && snippet.text_highlights.is_empty() {
            source
                .split('\n')
                .enumerate()
                .filter(|(_, line)| {
                    let line = line.trim();
                    !line.is_empty() && !line.chars().all(|c| "{}[]();,".contains(c))
                })
                .last()
                .map(|(i, _)| i + 1)
                .into_iter()
                .collect()
        } else {
            marked
        };
        for line in marked {
            let selector = format!(".Original .line[data-line=\"{line}\"]");
            if theme.id == "highlighter-pass" {
                write!(extra, "{selector}{{background-color:transparent}}{selector}>span{{background-color:#d6ff26}}{selector}:after{{content:\"\u{200b}\";font-size:0;line-height:0;position:absolute;left:-12px;top:5px;width:0;height:0;border-top:6px solid transparent;border-bottom:6px solid transparent;border-left:8px solid #111}}" ).unwrap();
            } else {
                write!(extra, "{selector}{{box-shadow:inset 3px 0 #e2401b}}{selector}>span{{text-decoration:underline;text-decoration-color:#1f4fd1;text-decoration-thickness:1.5px;text-underline-offset:4px}}" ).unwrap();
            }
        }
    }
    let treatment = match theme.id.as_str() {
        "single-ink" => include_str!("../assets/originals/single-ink.css"),
        "swatch-card" => include_str!("../assets/originals/swatch-card.css"),
        "wayfinding" => include_str!("../assets/originals/wayfinding.css"),
        "highlighter-pass" => include_str!("../assets/originals/highlighter-pass.css"),
        "gallery-wall" => include_str!("../assets/originals/gallery-wall.css"),
        "departure-board" => include_str!("../assets/originals/departure-board.css"),
        "dither-light" => include_str!("../assets/originals/dither-light.css"),
        "cutting-mat" => include_str!("../assets/originals/cutting-mat.css"),
        "thermal-receipt" => include_str!("../assets/originals/thermal-receipt.css"),
        "proof-marks" => include_str!("../assets/originals/proof-marks.css"),
        _ => unreachable!(),
    };
    let mut css = format!(
        "{}\n#capture{{--editor-foreground:{};--editor-gutter:{};--surface:{surface};--outer:{outer};--inset:{inset}px}}{}\n{treatment}\n{extra}",
        include_str!("../assets/editor/frame.css"),
        palette["--ink-foreground"],
        palette["--ink-token-comment"],
        include_str!("../assets/originals/frame.css")
    );
    // Artwork is a backdrop. A transparent export retains its reading surface.
    if style.background != Background::Theme {
        css += ".Original{background-image:none;background-color:transparent}.Gallery__lamp,.Cutting__top,.Cutting__left{display:none}";
    }
    if style.background == Background::None {
        css += "#capture{background:transparent}.Original__paper,.Gallery__frame,.Swatch__stack,.Departure__housing,.Receipt__strip{box-shadow:none}";
    }
    Frame {
        template: format!(
            "<div class=\"Original {}\" style=\"padding:__PADDING__px\">{content}</div>",
            theme.id
        ),
        css,
    }
}

fn svg_url(svg: &str) -> String {
    format!("data:image/svg+xml;base64,{}", STANDARD.encode(svg))
}

/// Dots use explicit vector geometry instead of browser-only contrast filters.
/// Scale the pitch on very large exports to keep the decorative work bounded.
pub(crate) fn measured_css(style: &Style, width: f32, height: f32) -> String {
    if style.theme.id() != "dither-light" || style.background != Background::Theme {
        return String::new();
    }
    let pitch = 7.0_f32.max((width * height / 12_000.0).sqrt());
    let mut dots = String::new();
    for y in 0..(height / pitch).ceil() as usize {
        for x in 0..(width / pitch).ceil() as usize {
            let px = (x as f32 + 0.5) * pitch;
            let py = (y as f32 + 0.5) * pitch;
            let a = (px / width).hypot(py / height);
            let b = (1.0 - px / width).hypot(1.0 - py / height);
            let tone = (1.0 - a.min(b) * 1.8).max(0.0);
            let radius = pitch * 0.74 * tone;
            if radius > 0.2 {
                write!(dots, "<circle cx='{px:.2}' cy='{py:.2}' r='{radius:.2}'/>").unwrap();
            }
        }
    }
    let url = svg_url(&format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' viewBox='0 0 {width} {height}'><rect width='100%' height='100%' fill='white'/><g fill='black'>{dots}</g></svg>"
    ));
    format!(".dither-light{{background-image:url({url});background-size:{width}px {height}px}}")
}
