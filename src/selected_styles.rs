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
    base_font_size: Option<f32>,
}

pub(crate) fn frame(theme: &assets::Theme, style: &Style, lines: usize) -> Frame {
    static STUDIES: LazyLock<BTreeMap<String, Study>> = LazyLock::new(|| {
        let mut studies: BTreeMap<String, Study> =
            serde_json::from_str(include_str!("../assets/selected/frames.json"))
                .expect("bundled selected frames");
        let code_styles: BTreeMap<String, Study> =
            serde_json::from_str(include_str!("../assets/code-styles/frames.json"))
                .expect("bundled code-only frames");
        for (id, study) in code_styles {
            assert!(studies.insert(id, study).is_none(), "duplicate study ID");
        }
        studies
    });
    let study = &STUDIES[&theme.id];
    let size = style.font_size as f32;
    let scale = size / study.base_font_size.unwrap_or(25.0);
    let leading = size * study.line_height;
    // Native exports lay out at 2x. Half-pixel leading keeps flowing HTML and
    // raster/vector line stacks the same height instead of rounding each row.
    let leading = if study.base_font_size.is_some() {
        (leading * 2.0).round() / 2.0
    } else {
        leading
    };
    let digits = lines.to_string().len().max(2) as f32;
    let number_size = if study.base_font_size.is_some() {
        size * 0.7
    } else {
        size
    };
    let number_width = if study.base_font_size.is_some() {
        digits * number_size * 0.62
    } else {
        digits * size
    };
    let gutter = number_width + size;
    let available = style.width.unwrap_or(520).saturating_sub(style.padding * 2) as f32;
    let inset = (size * 2.0).min(available * 0.06);
    let tail = (size * 1.15).min(available * 0.04);
    let mut css = format!(
        "{}\n{}\n#capture{{--original-scale:{scale};--pad:{}px;--original-gutter:{gutter}px;--original-number-width:{number_width}px;--inset:{inset}px;--tail:{tail}px;--code-leading:{leading}px}}\
         .Original .Editor__formatted{{font-family:CodeMono;font-size:{size}px;line-height:{leading}px}}\
         .Original .Editor__showLineNumbers .line:before{{font-size:{number_size}px}}\
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
