//! The user's approved visual studies, adapted to flowing native code frames.
//! Material artwork is inert bundled HTML/CSS shared by every output format.
use crate::{Background, Style, assets, originals::Frame};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Study {
    html: String,
    css: String,
    code_weight: u32,
    line_height: f32,
}

pub(crate) fn frame(theme: &assets::Theme, style: &Style, lines: usize) -> Frame {
    static STUDIES: LazyLock<BTreeMap<String, Study>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/selected/frames.json"))
            .expect("bundled selected frames")
    });
    let study = &STUDIES[&theme.id];
    let size = style.font_size as f32;
    let scale = size / 25.0;
    let leading = size * study.line_height;
    let digits = lines.to_string().len().max(2) as f32;
    let number_width = digits * size;
    let gutter = number_width + size;
    let mut css = format!(
        "{}\n{}\n#capture{{--original-scale:{scale};--pad:{}px;--original-gutter:{gutter}px;--original-number-width:{number_width}px}}\
         .Original .Editor__formatted{{font-family:CodeMono;font-size:{size}px;line-height:{leading}px}}\
         .Original .Editor__editor .line{{min-height:{leading}px;font-weight:{}}}",
        study.css,
        include_str!("../assets/selected/frame.css"),
        style.padding,
        study.code_weight,
    );
    if style.background != Background::Theme {
        css += ".Original{background-image:none;background-color:transparent}.Original--holofoil .glow{display:none}";
    }
    if matches!(theme.id.as_str(), "label-tape" | "ragged-block") {
        css += ".Original .line.empty-line .lc{padding:0;background:none;box-shadow:none}";
    }
    Frame {
        template: study.html.clone(),
        css,
    }
}
