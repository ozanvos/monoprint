//! Shared frame for imported editor palettes; other presets keep their own markup.
use crate::{Background, assets};

pub(crate) fn template(title: bool, background: Background) -> String {
    let shadow = if background == Background::None {
        ""
    } else {
        " EditorFrame__shadow"
    };
    let label = if title {
        "<div class=\"EditorFrame__title\">__TITLE__</div>"
    } else {
        ""
    };
    format!(
        "<div class=\"EditorFrame\" style=\"padding:__PADDING__px\"><div class=\"EditorFrame__window{shadow}\"><div class=\"EditorFrame__header\"><div class=\"EditorFrame__controls\"><i></i><i></i><i></i></div>{label}</div>__CODE_IMAGES_EDITOR__</div></div>"
    )
}

pub(crate) fn css(theme: &assets::Theme) -> String {
    let e = theme.editor.as_ref().expect("editor palette");
    format!(
        "{}\n#capture{{--editor-surface:{};--editor-foreground:{};--editor-gutter:{};--editor-border:{}}}",
        include_str!("../assets/editor/frame.css"),
        e.surface,
        e.foreground,
        e.gutter,
        e.border
    )
}

// SynthWave '84 Neon Dreams, adapted from the pinned upstream theme_template.js.
// Preserve upstream's token-specific glow colors. License: licenses/editor-themes-NOTICE.txt.
pub(crate) fn glow(color: &str) -> &'static str {
    match color {
        "#FE4450" => {
            "color:#fff5f6;text-shadow:0 0 2px #000,0 0 10px #fc1f2c75,0 0 5px #fc1f2c75,0 0 25px #fc1f2c75;"
        }
        "#FF7EDB" => "color:#f92aad;text-shadow:0 0 2px #100c0f,0 0 5px #dc078e33,0 0 10px #fff3;",
        "#FEDE5D" => {
            "color:#f4eee4;text-shadow:0 0 2px #393a33,0 0 8px #f39f0575,0 0 2px #f39f0575;"
        }
        "#72F1B8" => "text-shadow:0 0 2px #100c0f,0 0 10px #257c5575,0 0 35px #21272475;",
        "#36F9F6" => {
            "color:#fdfdfd;text-shadow:0 0 2px #001716,0 0 3px #03edf975,0 0 5px #03edf975,0 0 8px #03edf975;"
        }
        _ => "",
    }
}
