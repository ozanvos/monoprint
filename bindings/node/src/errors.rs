use monoprint::{Error, InvalidOption, TextRole};
use serde_json::json;

pub fn internal(message: &str) -> String {
    json!({ "code": "RENDER_FAILED", "message": message, "details": {} }).to_string()
}

pub fn encode(error: Error) -> String {
    let (code, details) = match &error {
        Error::UnknownTheme(id) => ("UNKNOWN_THEME", json!({ "id": id })),
        Error::UnknownLanguage(id) => ("UNKNOWN_LANGUAGE", json!({ "id": id })),
        Error::UnknownFont(id) => ("UNKNOWN_FONT", json!({ "id": id })),
        Error::InvalidFont => ("INVALID_FONT", json!({})),
        Error::UnsupportedGlyph {
            text,
            character,
            code_font,
        } => (
            "UNSUPPORTED_GLYPH",
            json!({
                "text": match text { TextRole::Code => "code", TextRole::Comment => "comment", TextRole::Title => "title" },
                "character": character.to_string(), "codeFont": code_font,
            }),
        ),
        Error::ImageTooLarge {
            width,
            height,
            supersampled,
        } => (
            "IMAGE_TOO_LARGE",
            json!({ "width": width, "height": height, "supersampled": supersampled }),
        ),
        Error::InvalidOption(option) => (
            "INVALID_OPTION",
            match option {
                InvalidOption::SourceTooLarge { len } => {
                    json!({ "option": "source", "bytes": len, "max": monoprint::MAX_SOURCE_BYTES })
                }
                InvalidOption::FontTooLarge { len } => {
                    json!({ "option": "font", "bytes": len, "max": monoprint::MAX_FONT_BYTES })
                }
                InvalidOption::Padding(value) => {
                    json!({ "option": "padding", "value": value, "min": 0, "max": 512 })
                }
                InvalidOption::FontSize(value) => {
                    json!({ "option": "fontSize", "value": value, "min": 8, "max": 72 })
                }
                InvalidOption::Width { width, min, max } => {
                    json!({ "option": "width", "value": width, "min": min, "max": max })
                }
                InvalidOption::Scale(value) => {
                    json!({ "option": "scale", "value": value, "min": 1, "max": 6 })
                }
                InvalidOption::PngCompression(value) => {
                    json!({ "option": "compression", "value": value })
                }
                InvalidOption::Background(value) => {
                    json!({ "option": "background", "value": value })
                }
                InvalidOption::Appearance { theme, appearance } => {
                    json!({ "option": "appearance", "theme": theme.id(), "appearance": appearance.to_string() })
                }
                InvalidOption::HighlightedLines {
                    start,
                    end,
                    line_count,
                } => {
                    json!({ "option": "highlight", "start": start, "end": end, "lineCount": line_count })
                }
                InvalidOption::Subtitle { theme } => {
                    json!({ "option": "subtitle", "theme": theme.id() })
                }
                _ => json!({}),
            },
        ),
        _ => ("RENDER_FAILED", json!({})),
    };
    json!({ "code": code, "message": error.to_string(), "details": details }).to_string()
}
