//! Lenient terminal capture preprocessing (not a terminal emulator).
use crate::syntax::{StyledSource, push_run};
use giallo::{Color, FontStyle, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Ink {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Attributes {
    fg: Ink,
    bg: Ink,
    font: FontStyle,
    dim: bool,
    inverse: bool,
    hidden: bool,
}
#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    style: Attributes,
}

/// Return visible text after escape removal, carriage-return redraws and overstrikes.
pub fn strip(source: &str) -> String {
    parse(source).0
}

/// Whether a capture contains a complete SGR color/style sequence.
pub fn contains_sgr(source: &str) -> bool {
    source.split("\x1b[").skip(1).any(|rest| {
        rest.find(|c: char| !c.is_ascii_digit() && c != ';' && c != ':')
            .is_some_and(|i| rest.as_bytes()[i] == b'm')
    })
}

fn parse(source: &str) -> (String, Vec<Cell>) {
    let mut chars = source.chars().peekable();
    let mut cells: Vec<Cell> = Vec::new();
    let (mut start, mut cursor) = (0, 0);
    let mut style = Attributes::default();
    let mut overstrike = false;
    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => match chars.next() {
                Some('[') => {
                    let mut parameters = String::new();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            if c == 'm' {
                                sgr(&parameters, &mut style);
                            }
                            if c == 'K' && (parameters.is_empty() || parameters == "0") {
                                cells.truncate(cursor);
                            }
                            break;
                        }
                        if parameters.len() < 256 {
                            parameters.push(c);
                        }
                    }
                }
                Some(kind @ (']' | 'P' | '_' | '^' | 'X')) => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' && kind == ']' {
                            break;
                        }
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(c) if (' '..='/').contains(&c) => {
                    for c in chars.by_ref() {
                        if ('0'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\r' => {
                cursor = start;
                overstrike = false;
            }
            '\n' => {
                cells.push(Cell { ch, style });
                start = cells.len();
                cursor = start;
                overstrike = false;
            }
            '\x08' => {
                overstrike = cursor > start;
                cursor = cursor.saturating_sub(1).max(start);
            }
            c if (c.is_control() && c != '\t') => {}
            mut ch => {
                let mut attributes = style;
                if let Some(previous) = cells.get(cursor).filter(|_| overstrike) {
                    if previous.ch == ch || previous.ch == '_' || ch == '_' {
                        // Overstrikes decorate the same glyph; retain decorations
                        // accumulated by earlier passes (for example _\bX\bX).
                        for flag in [FontStyle::BOLD, FontStyle::UNDERLINE] {
                            if previous.style.font.contains(flag) {
                                set_font(&mut attributes.font, flag, true);
                            }
                        }
                    }
                    if previous.ch == ch {
                        set_font(&mut attributes.font, FontStyle::BOLD, true);
                    }
                    if previous.ch == '_' || ch == '_' {
                        if ch == '_' {
                            ch = previous.ch;
                        }
                        set_font(&mut attributes.font, FontStyle::UNDERLINE, true);
                    }
                }
                let cell = Cell {
                    ch,
                    style: attributes,
                };
                if cursor < cells.len() {
                    cells[cursor] = cell;
                } else {
                    cells.push(cell);
                }
                cursor += 1;
                overstrike = false;
            }
        }
    }
    (cells.iter().map(|cell| cell.ch).collect(), cells)
}
fn sgr(parameters: &str, style: &mut Attributes) {
    let groups: Vec<&str> = parameters.split(';').collect();
    let mut i = 0;
    while i < groups.len() {
        let group = groups[i];
        i += 1;
        let colon: Vec<_> = group.split(':').collect();
        let code = colon[0]
            .parse::<u16>()
            .unwrap_or(if colon[0].is_empty() { 0 } else { 999 });
        if matches!(code, 38 | 48 | 58) {
            let values: Vec<Option<u16>>;
            if colon.len() > 1 {
                values = colon[1..].iter().map(|s| s.parse().ok()).collect();
            } else {
                let kind = groups.get(i).and_then(|s| s.parse::<u16>().ok());
                let count = match kind {
                    Some(2) => 4,
                    Some(5) => 2,
                    _ => 1,
                };
                values = groups[i..(i + count).min(groups.len())]
                    .iter()
                    .map(|s| s.parse().ok())
                    .collect();
                i = (i + count).min(groups.len());
            }
            let color = match values.as_slice() {
                [Some(5), Some(n)] if *n <= 255 => Some(Ink::Indexed(*n as u8)),
                [Some(2), rest @ ..] => {
                    let rgb = if rest.len() == 4 { &rest[1..] } else { rest };
                    match rgb {
                        [Some(r), Some(g), Some(b)] if *r <= 255 && *g <= 255 && *b <= 255 => {
                            Some(Ink::Rgb(*r as u8, *g as u8, *b as u8))
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(color) = color {
                if code == 38 {
                    style.fg = color;
                } else if code == 48 {
                    style.bg = color;
                }
            }
            continue;
        }
        match code {
            0 => *style = Attributes::default(),
            1 => set_font(&mut style.font, FontStyle::BOLD, true),
            2 => style.dim = true,
            3 => set_font(&mut style.font, FontStyle::ITALIC, true),
            4 | 21 => set_font(&mut style.font, FontStyle::UNDERLINE, true),
            7 => style.inverse = true,
            8 => style.hidden = true,
            9 => set_font(&mut style.font, FontStyle::STRIKETHROUGH, true),
            22 => {
                set_font(&mut style.font, FontStyle::BOLD, false);
                style.dim = false;
            }
            23 => set_font(&mut style.font, FontStyle::ITALIC, false),
            24 => set_font(&mut style.font, FontStyle::UNDERLINE, false),
            27 => style.inverse = false,
            28 => style.hidden = false,
            29 => set_font(&mut style.font, FontStyle::STRIKETHROUGH, false),
            30..=37 => style.fg = Ink::Indexed((code - 30) as u8),
            90..=97 => style.fg = Ink::Indexed((code - 90 + 8) as u8),
            40..=47 => style.bg = Ink::Indexed((code - 40) as u8),
            100..=107 => style.bg = Ink::Indexed((code - 100 + 8) as u8),
            39 => style.fg = Ink::Default,
            49 => style.bg = Ink::Default,
            _ => {}
        }
    }
}
pub(crate) fn highlight(
    source: &str,
    foreground: Color,
    background: Color,
    palette: &[Color; 16],
) -> StyledSource {
    fn resolve(ink: Ink, default: Color, palette: &[Color; 16]) -> Color {
        match ink {
            Ink::Default => default,
            Ink::Indexed(n @ 0..=15) => palette[n as usize],
            Ink::Indexed(n) => {
                let (r, g, b) = if n >= 232 {
                    let v = 8 + (n - 232) * 10;
                    (v, v, v)
                } else {
                    let n = n - 16;
                    let c = |v| if v == 0 { 0 } else { 55 + 40 * v };
                    (c(n / 36), c(n / 6 % 6), c(n % 6))
                };
                rgb(r, g, b)
            }
            Ink::Rgb(r, g, b) => rgb(r, g, b),
        }
    }
    let (_, cells) = parse(source);
    // Default terminal backgrounds are transparent. Keep this distinct from
    // an explicit RGB background that happens to match the theme surface.
    let transparent = Color::from_hex("#00000000").unwrap();
    let mut runs = Vec::new();
    let mut offset = 0;
    for cell in cells {
        let a = cell.style;
        let mut fg = resolve(a.fg, foreground, palette);
        let mut bg = resolve(a.bg, background, palette);
        if a.inverse {
            std::mem::swap(&mut fg, &mut bg);
        }
        if a.dim {
            let f = fg.as_hex();
            let b = background.as_hex();
            let channel = |s: &str, i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0) as u16;
            fg = rgb(
                ((channel(&f, 1) + channel(&b, 1)) / 2) as u8,
                ((channel(&f, 3) + channel(&b, 3)) / 2) as u8,
                ((channel(&f, 5) + channel(&b, 5)) / 2) as u8,
            );
        }
        if a.hidden {
            fg = bg;
        }
        let end = offset + cell.ch.len_utf8();
        push_run(
            &mut runs,
            offset..end,
            Style {
                foreground: fg,
                background: if a.bg == Ink::Default && !a.inverse {
                    transparent
                } else {
                    bg
                },
                font_style: a.font,
            },
            false,
        );
        offset = end;
    }
    StyledSource {
        runs,
        background: transparent,
    }
}
fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_hex(&format!("#{r:02x}{g:02x}{b:02x}")).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discipline() {
        assert_eq!(strip("long\rX"), "Xong");
        assert_eq!(strip("long\rX\x1b[K"), "X");
        assert_eq!(strip("a\r\nb"), "a\nb");
        assert_eq!(strip("a\x1bPpayload\x07still hidden\x1b\\b"), "ab");
        assert_eq!(strip("_\x08界 X\x08X"), "界 X");
        assert_eq!(strip("\x1b]8;;url\x07link\x1b]8;;\x1b\\\x1b[31"), "link");
    }
    #[test]
    fn extended_and_reset() {
        let (_, c) = parse("\x1b[38:2::1:2:3;1;2mA\x1b[22mB\x1b[48;5;200mC");
        assert_eq!(c[0].style.fg, Ink::Rgb(1, 2, 3));
        assert!(c[0].style.dim);
        assert!(!c[1].style.dim);
        assert_eq!(c[2].style.bg, Ink::Indexed(200));
    }
}

fn set_font(font: &mut FontStyle, flag: FontStyle, enabled: bool) {
    let flags = [
        (FontStyle::BOLD, "bold"),
        (FontStyle::ITALIC, "italic"),
        (FontStyle::UNDERLINE, "underline"),
        (FontStyle::STRIKETHROUGH, "strikethrough"),
    ];
    let names = flags
        .iter()
        .filter(|(f, _)| {
            if *f == flag {
                enabled
            } else {
                font.contains(*f)
            }
        })
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(" ");
    *font = FontStyle::from_theme_str(&names);
}

#[cfg(test)]
mod invariants {
    use super::*;
    #[test]
    fn sgr_flags_and_colors() {
        let (_, cells) = parse("\x1b[1;2;3;4;7;8;9;91;104mA\x1b[22;23;24;27;28;29;39;49mB");
        let a = cells[0].style;
        assert!(a.font.contains(FontStyle::BOLD));
        assert!(a.font.contains(FontStyle::ITALIC));
        assert!(a.font.contains(FontStyle::UNDERLINE));
        assert!(a.font.contains(FontStyle::STRIKETHROUGH));
        assert!(a.dim && a.hidden && a.inverse);
        assert_eq!(a.fg, Ink::Indexed(9));
        assert_eq!(a.bg, Ink::Indexed(12));
        assert_eq!(cells[1].style, Attributes::default());
        for sequence in ["38;2;1;2;3", "38:2:1:2:3", "38:2::1:2:3"] {
            assert_eq!(
                parse(&format!("\x1b[{sequence}mX")).1[0].style.fg,
                Ink::Rgb(1, 2, 3)
            );
        }
        assert_eq!(
            parse("\x1b[38;5;999;3mX").1[0].style.font,
            FontStyle::ITALIC
        );
        assert_eq!(parse("\x1b[58;2;1;2;3mX").1[0].style, Attributes::default());
        assert!(parse("X\x08X").1[0].style.font.contains(FontStyle::BOLD));
        assert!(parse("X\rX").1[0].style.font.is_empty());
    }
    #[test]
    fn explicit_surface_background_is_not_transparent() {
        let surface = rgb(10, 20, 30);
        let styled = highlight(
            "a\x1b[48;2;10;20;30mb\x1b[49mc",
            rgb(255, 255, 255),
            surface,
            &[surface; 16],
        );
        assert_eq!(styled.runs.len(), 3);
        assert_eq!(styled.runs[0].style.background, styled.background);
        assert_eq!(styled.runs[1].style.background, surface);
        assert_ne!(styled.runs[1].style.background, styled.background);
        assert_eq!(styled.runs[2].style.background, styled.background);
    }
    #[test]
    fn combined_overstrikes_retain_bold_and_underline() {
        for source in ["_\x08X\x08X", "X\x08X\x08_", "X\x08_\x08X"] {
            let (text, cells) = parse(source);
            assert_eq!(text, "X");
            assert!(cells[0].style.font.contains(FontStyle::BOLD));
            assert!(cells[0].style.font.contains(FontStyle::UNDERLINE));
        }
    }
    #[test]
    fn bounded_random_unicode_captures_tile_visible_text() {
        let alphabet = [
            '\x1b', '[', ']', 'P', '\\', 'm', 'K', ';', '0', '1', ':', '\r', '\n', '\x08', '\x07',
            '\t', '界', '🦀', 'é', 'x',
        ];
        let mut seed = 19u64;
        let color = Color::from_hex("#111111").unwrap();
        for length in 0..500 {
            let source: String = (0..length)
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    alphabet[(seed as usize) % alphabet.len()]
                })
                .collect();
            let visible = strip(&source);
            assert!(!visible.contains('\x1b'));
            let styled = highlight(&source, color, color, &[color; 16]);
            let mut end = 0;
            for run in styled.runs {
                assert_eq!(run.range.start, end);
                assert!(visible.is_char_boundary(run.range.start));
                assert!(visible.is_char_boundary(run.range.end));
                end = run.range.end;
            }
            assert_eq!(end, visible.len());
        }
    }
}

#[cfg(test)]
mod palette_tests {
    fn luminance(color: &str) -> f64 {
        [1, 3, 5]
            .into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(i, weight)| {
                let value = u8::from_str_radix(&color[i..i + 2], 16).unwrap() as f64 / 255.;
                weight
                    * if value <= 0.04045 {
                        value / 12.92
                    } else {
                        ((value + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }
    fn contrast(a: &str, b: &str) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn every_theme_has_readable_terminal_and_diff_tokens() {
        for theme in crate::assets::themes().values() {
            for (mode, p) in &theme.syntax {
                let surface = &p["--ink-ansi-surface"];
                for i in 0..16 {
                    assert!(
                        contrast(&p[&format!("--ink-ansi-{i}")], surface) >= 3. - 1e-6,
                        "{} {mode} ANSI {i}",
                        theme.id
                    );
                }
                for kind in ["added", "removed"] {
                    for suffix in ["", "-word"] {
                        assert!(
                            contrast(
                                &p["--ink-foreground"],
                                &p[&format!("--ink-diff-{kind}{suffix}")]
                            ) >= 4.5 - 1e-6,
                            "{} {mode} diff {kind}{suffix}",
                            theme.id
                        );
                    }
                    assert!(
                        contrast(
                            &p[&format!("--ink-diff-{kind}-sign")],
                            &p[&format!("--ink-diff-{kind}")]
                        ) >= 3. - 1e-6
                    );
                }
                for i in [1, 2, 9, 10] {
                    assert_ne!(
                        p[&format!("--ink-ansi-{i}")].to_lowercase(),
                        p["--ink-foreground"].to_lowercase()
                    );
                }
            }
        }
    }
}
