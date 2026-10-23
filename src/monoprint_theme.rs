//! The selected Masthead/Oxide composition, with paired appearance tokens.
//! Flow layout keeps the frame attached to real, possibly wrapped source text.
use crate::{Style, assets, originals::Frame};

pub(crate) fn frame(theme: &assets::Theme, style: &Style, lines: usize) -> Frame {
    let palette = &theme.syntax[style.appearance.as_str()];
    let colors = palette
        .iter()
        .map(|(key, value)| format!("{key}:{value};"))
        .collect::<String>();
    let size = style.font_size as f32;
    let scale = size / 25.0;
    let available = style.width.map(|w| w - 2 * style.padding).unwrap_or(912) as f32;
    // At the selected 1000px/25px study size these are exactly 72px and 40px.
    // Narrow exports retain a usable reading column rather than fixed gutters.
    let inset = (72.0 * scale)
        .min((available - 2.0).max(0.0) * 0.08)
        .round();
    let label_inset = (40.0 * scale)
        .min((available - 2.0).max(0.0) * 0.06)
        .round();
    let digits = lines.to_string().len() as f32;
    let number_gap = (size * 1.6).round();
    let number_width = (size * (digits * 0.7).max(2.0)).ceil() + number_gap;
    // Native output uses even internal scales. Half-pixel leading and whole
    // frame dimensions keep wrapping and export dimensions stable at 1–6x.
    let leading = (size * 1.6 * 2.0).round() / 2.0;
    let scaled = |value: f32| (value * scale).round();
    let css = format!(
        "{}\n#capture{{{colors}--masthead-mast-height:{mast_height}px;--masthead-foot-height:{foot_height}px;\
         --masthead-label-padding:{label_padding}px;--masthead-foot-padding:{foot_padding}px;\
         --masthead-body-padding:{body_padding}px;--masthead-inset:{inset}px;\
         --masthead-label-inset:{label_inset}px;--masthead-leading:{}px;\
         --masthead-code-size:{size}px;--masthead-label-size:{}px;\
         --masthead-file-size:{}px;--masthead-number-width:{number_width}px;--masthead-number-gap:{number_gap}px;\
         --editor-foreground:var(--ink-foreground);--editor-gutter:var(--ink-token-comment)}}\n{}",
        include_str!("../assets/editor/frame.css"),
        leading,
        scaled(12.0).max(10.0),
        scaled(16.0).max(12.0),
        include_str!("../assets/originals/monoprint.css"),
        mast_height = scaled(56.0),
        // Keep the strip's original height after removing its text labels.
        foot_height = scaled(40.0).max(scaled(12.0).max(10.0) * 1.5 + scaled(10.0) * 2.0 + 1.0),
        label_padding = scaled(12.0),
        foot_padding = scaled(10.0),
        body_padding = scaled(42.0),
    );
    Frame {
        template: "<div class=\"Monoprint\" style=\"padding:__PADDING__px\">\
          <div class=\"Monoprint__page\">\
            <div class=\"Monoprint__mast\"><div class=\"Monoprint__file\">__TITLE__</div><div class=\"Monoprint__language\">__LANGUAGE__</div></div>\
            <div class=\"Monoprint__body\">__CODE_IMAGES_EDITOR__</div>\
            <div class=\"Monoprint__foot\" aria-hidden=\"true\"></div>\
          </div></div>".into(),
        css,
    }
}
