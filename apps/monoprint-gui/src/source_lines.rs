//! Whole-line marks and gutter interaction over the existing editor's geometry.
use crate::{appearance::ACCENT, studio::Studio};
use gpui_kit::{
    component::input::{EditorState, RopeExt},
    prelude::*,
    *,
};

#[derive(PartialEq)]
pub struct SourceLine {
    pub index: usize,
    pub bounds: Bounds<Pixels>,
    pub gutter: Bounds<Pixels>,
}

/// Only visible logical lines are visited. Their laid-out bounds include wrapping
/// and vertical scrolling; no second text layout or highlighting pass is needed.
pub fn visible_lines(editor: &EditorState) -> Vec<SourceLine> {
    let viewport = editor.input_bounds();
    editor
        .visible_row_range()
        .unwrap_or_default()
        .filter_map(|index| {
            let text = editor.text();
            let start = text.line_start_offset(index);
            let end = text.line_end_offset(index);
            let extent = editor.range_to_bounds(&(start..end))?;
            let bounds = Bounds::from_corners(
                point(viewport.left(), extent.top()),
                point(viewport.right(), extent.bottom()),
            )
            .intersect(&viewport);
            if bounds.size.height <= px(0.) {
                return None;
            }
            // Text bounds move horizontally; the number gutter stays fixed.
            let gutter_right = (extent.left() - editor.scroll_offset().x)
                .max(viewport.left())
                .min(viewport.right());
            let gutter = Bounds::from_corners(bounds.origin, point(gutter_right, bounds.bottom()));
            Some(SourceLine {
                index,
                bounds,
                gutter,
            })
        })
        .collect()
}

/// Resolve the glyph under a click after the editor has positioned its caret.
/// Checking both adjacent characters handles the right half of the last glyph
/// without treating an exclusive range end or empty row space as highlighted.
pub fn character_at_point(editor: &EditorState, position: Point<Pixels>) -> Option<usize> {
    let row = visible_lines(editor)
        .into_iter()
        .find(|row| row.bounds.contains(&position) && !row.gutter.contains(&position))?;
    let source = editor.value().to_string();
    let cursor = editor.cursor();
    let before = source[..cursor]
        .char_indices()
        .next_back()
        .map(|(offset, _)| offset);
    for start in before.into_iter().chain(std::iter::once(cursor)) {
        let Some(character) = source[start..].chars().next() else {
            continue;
        };
        if matches!(character, '\r' | '\n') {
            continue;
        }
        let end = start + character.len_utf8();
        let Some(extent) = editor.range_to_bounds(&(start..end)) else {
            continue;
        };
        let height = editor.line_height()?;
        let bounds = if extent.size.height > height {
            // The following caret is on the next wrapped row. This character
            // occupies the tail of its own visual row, never the following row.
            Bounds::from_corners(
                extent.origin,
                point(row.bounds.right(), extent.top() + height),
            )
        } else {
            extent
        };
        if bounds.intersect(&row.bounds).contains(&position) {
            return Some(start);
        }
    }
    None
}

pub fn overlay(editor: &Entity<EditorState>, studio: WeakEntity<Studio>) -> impl IntoElement {
    let layout_editor = editor.clone();
    let editor = editor.clone();
    canvas(
        move |_, window, cx| {
            let lines = visible_lines(layout_editor.read(cx));
            let text_hitbox = window.insert_hitbox(
                layout_editor.read(cx).input_bounds(),
                HitboxBehavior::Normal,
            );
            let hitboxes = lines
                .iter()
                .map(|line| window.insert_hitbox(line.gutter, HitboxBehavior::Normal))
                .collect::<Vec<_>>();
            (lines, hitboxes, text_hitbox)
        },
        move |_, (previous_lines, hitboxes, text_hitbox), window, cx| {
            // The editor paints first, so read its current geometry for the tint
            // and hit testing, including the first frame after a scroll or wrap.
            let lines = visible_lines(editor.read(cx));
            if lines != previous_lines {
                // Refresh hitboxes once after the editor settles new wrapping or
                // scroll geometry, even when its selection and viewport are unchanged.
                let _ = studio.update(cx, |_, cx| cx.notify());
            }
            let highlights = studio
                .read_with(cx, |studio, cx| studio.line_highlights(cx))
                .unwrap_or_default();
            for line in &lines {
                if highlights.iter().any(|range| range.contains(&line.index)) {
                    window.paint_quad(fill(line.bounds, rgb(ACCENT).opacity(0.10)));
                    window.paint_quad(fill(line.gutter, rgb(ACCENT).opacity(0.08)));
                    window.paint_quad(fill(
                        Bounds::new(line.gutter.origin, size(px(2.), line.gutter.size.height)),
                        rgb(ACCENT).opacity(0.8),
                    ));
                }
            }
            for hitbox in &hitboxes {
                window.set_cursor_style(CursorStyle::PointingHand, hitbox);
            }
            let click_studio = studio.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.capture()
                    && event.button == MouseButton::Left
                    && event.click_count == 1
                    && text_hitbox.is_hovered(window)
                {
                    let _ = click_studio.update(cx, |studio, cx| {
                        studio.open_highlight_menu(event.position, cx)
                    });
                }
            });
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if !phase.capture()
                    || event.button != MouseButton::Left
                    || !hitboxes.iter().any(|hitbox| hitbox.is_hovered(window))
                {
                    return;
                }
                if let Some(line) = lines
                    .iter()
                    .find(|line| line.gutter.contains(&event.position))
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    let _ = studio.update(cx, |studio, cx| {
                        studio.toggle_line_highlights(line.index..line.index + 1, cx);
                    });
                }
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}
