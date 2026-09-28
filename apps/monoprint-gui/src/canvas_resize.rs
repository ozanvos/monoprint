//! Width-handle geometry in image pixels, independent of preview completion.
use crate::viewport::PreviewViewport;
use gpui_kit::{Pixels, Point, Size, point, px};
use std::ops::RangeInclusive;

#[derive(Clone, Copy)]
pub enum ResizeEdge {
    Left,
    Right,
}

pub struct CanvasResize {
    edge: ResizeEdge,
    start_x: Pixels,
    start_width: u32,
    start_origin: Point<Pixels>,
    range: RangeInclusive<u32>,
    pub scale: f32,
    pub width: u32,
    pub dragging: bool,
    pub changed: bool,
}

impl CanvasResize {
    pub fn new(
        edge: ResizeEdge,
        pointer: Point<Pixels>,
        image: Size<Pixels>,
        viewport: &PreviewViewport,
        range: RangeInclusive<u32>,
    ) -> Self {
        Self {
            edge,
            start_x: pointer.x,
            start_width: f32::from(image.width).round() as u32,
            start_origin: viewport.origin(image),
            range,
            scale: viewport.scale(image),
            width: f32::from(image.width).round() as u32,
            dragging: true,
            changed: false,
        }
    }

    pub fn update(&mut self, pointer: Point<Pixels>) -> bool {
        let direction = match self.edge {
            ResizeEdge::Left => -1.,
            ResizeEdge::Right => 1.,
        };
        let delta = f32::from(pointer.x - self.start_x) / self.scale * direction;
        let width = (self.start_width as f32 + delta)
            .round()
            .clamp(*self.range.start() as f32, *self.range.end() as f32) as u32;
        if width == self.width {
            return false;
        }
        self.width = width;
        self.changed = true;
        true
    }

    /// Keep the opposite edge and the top of the image anchored. Use the last
    /// completed width for the image, and the requested width for its outline.
    pub fn origin(&self, width: Pixels) -> Point<Pixels> {
        let x = match self.edge {
            ResizeEdge::Left => {
                self.start_origin.x + (px(self.start_width as f32) - width) * self.scale
            }
            ResizeEdge::Right => self.start_origin.x,
        };
        point(x, self.start_origin.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::PreviewZoom;
    use gpui_kit::{Bounds, size};

    #[test]
    fn handles_use_frozen_zoom_and_anchor_the_opposite_edge() {
        let mut viewport = PreviewViewport::default();
        viewport.reset(PreviewZoom::Half);
        viewport.bounds = Bounds::new(point(px(240.), px(40.)), size(px(700.), px(600.)));
        let image = size(px(720.), px(400.));
        let pointer = point(px(700.), px(300.));
        for edge in [ResizeEdge::Left, ResizeEdge::Right] {
            let mut resize = CanvasResize::new(edge, pointer, image, &viewport, 177..=8192);
            assert!(!resize.update(pointer));
            assert!(!resize.changed);
            assert!(resize.update(pointer + point(px(50.), px(70.))));
            let origin = resize.origin(px(resize.width as f32));
            let initial = viewport.origin(image);
            assert_eq!(
                origin.y, initial.y,
                "vertical pointer movement does not resize height"
            );
            match edge {
                ResizeEdge::Left => {
                    assert_eq!(resize.width, 620);
                    assert_eq!(
                        origin.x + px(620.) * resize.scale,
                        initial.x + px(720.) * resize.scale
                    );
                }
                ResizeEdge::Right => {
                    assert_eq!(resize.width, 820);
                    assert_eq!(origin.x, initial.x);
                }
            }
        }
    }

    #[test]
    fn handles_respect_padding_minimum_and_export_maximum() {
        let viewport = PreviewViewport::default();
        let image = size(px(720.), px(400.));
        let mut resize = CanvasResize::new(
            ResizeEdge::Right,
            Point::default(),
            image,
            &viewport,
            241..=8192,
        );
        resize.update(point(px(-100_000.), px(0.)));
        assert_eq!(resize.width, 241);
        resize.update(point(px(100_000.), px(0.)));
        assert_eq!(resize.width, 8192);
    }
}
