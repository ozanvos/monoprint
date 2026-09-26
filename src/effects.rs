//! Geometry equivalents for the few upstream decorations that use browser JS
//! or CSS operations absent from Takumi. All dimensions are CSS pixels.
use crate::{assets, cli::Args};
use base64::{Engine, engine::general_purpose::STANDARD};

pub fn adapt_frame(theme: &str, frame: String, width: f32, height: f32) -> String {
    if theme == "removed-frame" && height >= 240.0 {
        return frame.replace("StripeFrame__isSmall", "");
    }
    if theme == "removed-frame" {
        // Chromium ignores the upstream SVG's invalid two-number viewBox.
        // Give resvg the same viewport instead of its 300x150 fallback.
        let url = &assets::images()["assets/clerk/pattern.svg"];
        let svg =
            String::from_utf8(STANDARD.decode(url.split_once(',').unwrap().1).unwrap()).unwrap();
        let svg = svg.replace(
            "viewBox=\"0 0\"",
            &format!(
                "width=\"{}\" height=\"128\" viewBox=\"0 0 {} 128\"",
                width + 64.0,
                width + 64.0
            ),
        );
        return frame.replace(
            url,
            &format!("data:image/svg+xml;base64,{}", STANDARD.encode(svg)),
        );
    }
    frame
}

pub fn needs_measurement(theme: &str) -> bool {
    matches!(
        theme,
        "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
            | "removed-frame"
    )
}

pub struct Effects {
    pub css: String,
    pub overlay: String,
}

pub fn prepare(theme: &str, args: &Args, width: f32, height: f32) -> Effects {
    let padding = args.padding as f32;
    let w = width - 2.0 * padding;
    let h = height - 2.0 * padding;
    let mut css = format!(
        "#capture{{--circle-diameter:{}px;--export-window-width:{w}px}}",
        w.hypot(h)
    );
    let mut paths = String::new();
    if args.transparent && theme != "removed-frame" {
        return Effects {
            css,
            overlay: paths,
        };
    }
    match theme {
        "removed-frame" | "removed-frame" => {
            let (prefix, margin, offset, alpha) = if theme == "removed-frame" {
                ("Auth0Frame", 48.0, 0.5, 0.12)
            } else {
                ("TailwindFrame", 64.0, 23.5, 0.1)
            };
            css += &format!(
                ".{prefix}__gridlinesHorizontal,.{prefix}__gridlinesVertical{{display:none}}"
            );
            let color = if args.light {
                if theme == "removed-frame" {
                    "#2a2c36"
                } else {
                    "#020617"
                }
            } else {
                "#ffffff"
            };
            let left = padding - margin;
            let top = padding - margin;
            let right = width - padding + margin;
            let bottom = height - padding + margin;
            for (id, x1, y1, x2, y2, length) in [
                ("h", left, 0.0, right, 0.0, w + 2.0 * margin),
                ("v", 0.0, top, 0.0, bottom, h + 2.0 * margin),
            ] {
                let fade = margin / length * 100.0;
                paths += &format!(
                    "<defs><linearGradient id=\"grid-{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\"><stop stop-color=\"{color}\" stop-opacity=\"0\"/><stop offset=\"{fade}%\" stop-color=\"{color}\" stop-opacity=\"{alpha}\"/><stop offset=\"{}%\" stop-color=\"{color}\" stop-opacity=\"{alpha}\"/><stop offset=\"100%\" stop-color=\"{color}\" stop-opacity=\"0\"/></linearGradient></defs>",
                    100.0 - fade
                );
            }
            paths += &format!(
                "<path d=\"M{left} {}H{right}M{left} {}H{right}\" stroke=\"url(#grid-h)\" fill=\"none\"/><path d=\"M{} {top}V{bottom}M{} {top}V{bottom}\" stroke=\"url(#grid-v)\" fill=\"none\"/>",
                padding - offset,
                height - padding + offset,
                padding - offset,
                width - padding + offset
            );
        }
        "removed-frame" | "removed-frame" | "removed-frame" | "removed-frame" | "removed-frame" => {
            let prefix = if theme == "removed-frame" {
                "removed_frameFrame"
            } else if theme == "removed-frame" {
                "TriggerdevFrame"
            } else {
                "removed_frameFrame"
            };
            css += &format!(
                ".{prefix}__gridlinesHorizontal,.{prefix}__gridlinesVertical,.{prefix}__bracketLeft,.{prefix}__bracketRight{{display:none}}"
            );
            if theme == "removed-frame" {
                css += ".AwsFrame__awsGridlinesHorizontal,.AwsFrame__awsGridlinesVertical{display:none}";
            }
            let color = if theme == "removed-frame" {
                if args.light { "#d9d7d7" } else { "#272a2e" }
            } else if theme == "removed-frame" {
                if args.light { "#d2d2d2" } else { "#233242" }
            } else if theme == "removed-frame" {
                if args.light { "#e5e5e5" } else { "#262626" }
            } else if args.light {
                "#ebebeb"
            } else {
                "#1a1a1a"
            };
            let right = width - padding;
            let bottom = height - padding;
            paths += &format!(
                "<path d=\"M0 {padding}H{width}M0 {bottom}H{width}M{padding} 0V{height}M{right} 0V{height}\" fill=\"none\" stroke=\"{color}\"/>"
            );
            if matches!(theme, "removed-frame" | "removed-frame") {
                let color = if args.light { "#a8a8a8" } else { "#515356" };
                for (x, y) in [(padding, padding), (right, bottom)] {
                    paths += &format!(
                        "<path d=\"M{} {y}h25M{x} {}v25\" stroke=\"{color}\" fill=\"none\"/>",
                        x - 12.0,
                        y - 12.0
                    );
                }
            }
        }
        "removed-frame" => {
            css += ".NuxtFrame__window [data-frameborder]{display:none}";
            for (i, (inset, radius, opacity, edge, fade)) in [
                (0.0, 10.0, 1.0, 8, 20),
                (7.0, 18.0, 0.5, 10, 26),
                (13.0, 25.0, 0.25, 12, 28),
            ]
            .into_iter()
            .enumerate()
            {
                let rw = w + 2.0 * inset;
                let rh = h + 2.0 * inset;
                let x = padding - inset;
                let y = padding - inset;
                let half_projection = (rw + rh) * 0.25;
                paths += &format!(
                    "<defs><linearGradient id=\"ring{i}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"><stop stop-color=\"#00dc82\"/><stop offset=\"{edge}%\" stop-color=\"#00dc82\"/><stop offset=\"{fade}%\" stop-color=\"#00dc82\" stop-opacity=\"0\"/><stop offset=\"{}%\" stop-color=\"#00dc82\" stop-opacity=\"0\"/><stop offset=\"{}%\" stop-color=\"#00dc82\"/><stop offset=\"100%\" stop-color=\"#00dc82\"/></linearGradient></defs><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" fill=\"none\" stroke=\"url(#ring{i})\" opacity=\"{opacity}\"/>",
                    x + rw / 2.0 - half_projection,
                    y + rh / 2.0 - half_projection,
                    x + rw / 2.0 + half_projection,
                    y + rh / 2.0 + half_projection,
                    100 - fade,
                    100 - edge,
                    x + 0.5,
                    y + 0.5,
                    rw - 1.0,
                    rh - 1.0
                );
            }
        }
        "removed-frame" => {
            css += &format!(
                ".FirecrawlFrame__asciiArt{{font-size:{}px;font-family:CodeMono}}.FirecrawlFrame__frameOverlay{{display:none}}",
                w * 0.013
            );
            let color = if args.light { "#ededed" } else { "#444" };
            let right = width - padding;
            let bottom = height - padding;
            paths += &format!(
                "<path d=\"M0 {padding}H{width}M0 {bottom}H{width}M{padding} 0V{height}M{right} 0V{height}\" stroke=\"{color}\" fill=\"none\"/>"
            );
            for (x, y) in [
                (padding, padding),
                (right, padding),
                (padding, bottom),
                (right, bottom),
            ] {
                paths += &format!(
                    "<path transform=\"translate({} {})\" fill=\"{color}\" d=\"M10.5 4C10.5 7.31371 7.81371 10 4.5 10H0.5V11H4.5C7.81371 11 10.5 13.6863 10.5 17V21H11.5V17C11.5 13.6863 14.1863 11 17.5 11H21.5V10H17.5C14.1863 10 11.5 7.31371 11.5 4V0H10.5V4Z\"/>",
                    x - 11.0,
                    y - 10.5
                );
            }
        }
        _ => {}
    }
    let overlay = if paths.is_empty() {
        String::new()
    } else {
        format!(
            "<svg aria-hidden=\"true\" xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" style=\"position:absolute;left:0;top:0;pointer-events:none;z-index:20\">{paths}</svg>"
        )
    };
    Effects { css, overlay }
}

pub fn native_css(css: &str) -> String {
    css.replace("!important", "")
        // Chromium quantizes these fractional CSS borders to a CSS pixel in
        // the pinned reference. Match their layout contribution at every DPR.
        .replace("border: 0.5px", "border: 1px")
        .replace(
            "width: clamp(14rem, 80%, 40rem)",
            "width:80%;min-width:14rem;max-width:40rem",
        )
        // Empty generated content is omitted by Takumi. Keep the styled box
        // using invisible content; width/height still come from the upstream CSS.
        .replace(
            "content: \"\"",
            "content: \"\u{200b}\";font-size:0;line-height:0",
        )
}
