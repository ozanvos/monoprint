//! Adapt embedded frame CSS to the native raster and SVG backends.
// Takumi's raster backend paints shadow lists in forward order. CSS paints
// the first listed shadow on top. Adapt only our embedded CSS, keeping the SVG
// backend (which already corrects this order) and browser HTML unchanged.
fn raster_shadow_order(css: &str) -> String {
    css.split_inclusive(';')
        .map(|declaration| {
            let Some((before, value)) = declaration.split_once("box-shadow:") else {
                return declaration.to_owned();
            };
            let value = value.strip_suffix(';').unwrap_or(value);
            // A final declaration need not have a semicolon. Do not reverse
            // commas in the next rule (including font fallback lists).
            let (value, suffix) = value.split_at(value.find('}').unwrap_or(value.len()));
            let mut depth = 0;
            let mut start = 0;
            let mut shadows = Vec::new();
            for (i, ch) in value.char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    ',' if depth == 0 => {
                        shadows.push(value[start..i].trim());
                        start = i + 1;
                    }
                    _ => {}
                }
            }
            shadows.push(value[start..].trim());
            shadows.reverse();
            format!(
                "{before}box-shadow:{}{suffix}{}",
                shadows.join(","),
                if declaration.ends_with(';') { ";" } else { "" }
            )
        })
        .collect()
}

pub fn native_css(css: &str, raster: bool) -> String {
    // Block formatting gives each logical line its content-box wrap width.
    // Collapse only the separator newlines; preserve whitespace inside lines.
    let editor = ".Editor__formatted>pre>code{white-space:normal}\
        .Editor__editor .line{display:block;white-space:pre-wrap;font-weight:var(--code-weight)}\
        .Editor__formatted{font-weight:var(--code-weight)}\
        .Editor__editor .line:before{left:0;transform:translateX(6px)}";
    (if raster {
        raster_shadow_order(css)
    } else {
        css.to_owned()
    } + editor)
        .replace("!important", "")
        // Takumi ignores the font shorthand; expand inherited typography.
        .replace(
            "font: inherit",
            "font-family:inherit;font-size:inherit;font-weight:inherit;font-style:inherit;line-height:inherit",
        )
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

#[cfg(test)]
mod tests {
    #[test]
    fn shadow_order_stops_at_rule_boundaries_without_semicolons() {
        let css =
            ".a{box-shadow:inset 0 1px red,0 2px blue}.b{font-family:Code,Emoji;box-shadow:none}";
        let expected =
            ".a{box-shadow:0 2px blue,inset 0 1px red}.b{font-family:Code,Emoji;box-shadow:none}";
        assert_eq!(super::raster_shadow_order(css), expected);
    }
}
