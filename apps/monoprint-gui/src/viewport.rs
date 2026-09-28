//! View-only canvas navigation. Never changes the document or renders an image.
use gpui_kit::{Bounds, Pixels, Point, Size, point, px};

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum PreviewZoom {
    #[default]
    Fit,
    Half,
    Actual,
    Custom(f32),
}

#[derive(Default)]
pub struct PreviewViewport {
    pub zoom: PreviewZoom,
    pub pan: Point<Pixels>,
    pub bounds: Bounds<Pixels>,
    drag_position: Option<Point<Pixels>>,
}

impl PreviewViewport {
    fn fit_scale(&self, image: Size<Pixels>) -> f32 {
        let width = (f32::from(self.bounds.size.width) - 96.).max(1.);
        let height = (f32::from(self.bounds.size.height) - 110.).max(1.);
        (width / f32::from(image.width))
            .min(height / f32::from(image.height))
            .min(1.)
    }

    pub fn scale(&self, image: Size<Pixels>) -> f32 {
        match self.zoom {
            PreviewZoom::Fit => self.fit_scale(image),
            PreviewZoom::Half => 0.5,
            PreviewZoom::Actual => 1.,
            PreviewZoom::Custom(scale) => scale,
        }
    }

    /// Image origin relative to the canvas, centered before applying pan.
    pub fn origin(&self, image: Size<Pixels>) -> Point<Pixels> {
        let scaled = image.map(|length| length * self.scale(image));
        point(
            (self.bounds.size.width - scaled.width) / 2.,
            (self.bounds.size.height - scaled.height) / 2.,
        ) + self.pan
    }

    pub fn reset(&mut self, zoom: PreviewZoom) {
        self.zoom = zoom;
        self.pan = Point::default();
        self.end_drag();
    }

    pub fn pan_by(&mut self, delta: Point<Pixels>, image: Size<Pixels>) {
        self.pan += delta;
        self.constrain_pan(image);
    }

    pub fn zoom_by(&mut self, delta: f32, position: Point<Pixels>, image: Size<Pixels>) {
        if !delta.is_finite() || delta == 0. {
            return;
        }
        let old_scale = self.scale(image);
        // A very large image may fit below 10%; allow zooming back to that size.
        let scale = (old_scale * (1. + delta)).clamp(self.fit_scale(image).min(0.1), 4.);
        let anchor = position - self.bounds.center();
        self.pan = anchor - (anchor - self.pan) * (scale / old_scale);
        self.zoom = PreviewZoom::Custom(scale);
        self.constrain_pan(image);
    }

    pub fn constrain_pan(&mut self, image: Size<Pixels>) {
        let scaled = image.map(|length| length * self.scale(image));
        // Leave a small portion within reach, even after a long momentum scroll.
        let limit_x = ((self.bounds.size.width + scaled.width) / 2. - px(32.)).max(px(0.));
        let limit_y = ((self.bounds.size.height + scaled.height) / 2. - px(32.)).max(px(0.));
        self.pan.x = self.pan.x.clamp(-limit_x, limit_x);
        self.pan.y = self.pan.y.clamp(-limit_y, limit_y);
    }

    pub fn start_drag(&mut self, position: Point<Pixels>) {
        self.drag_position = Some(position);
    }

    pub fn drag_to(&mut self, position: Point<Pixels>, image: Size<Pixels>) {
        if let Some(previous) = self.drag_position.replace(position) {
            self.pan_by(position - previous, image);
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_position.is_some()
    }

    pub fn end_drag(&mut self) {
        self.drag_position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::size;

    #[test]
    fn pinch_preserves_the_image_point_under_the_pointer() {
        let mut viewport = PreviewViewport {
            bounds: Bounds::new(point(px(300.), px(40.)), size(px(800.), px(700.))),
            pan: point(px(40.), px(-30.)),
            ..Default::default()
        };
        let image = size(px(720.), px(400.));
        let position = point(px(830.), px(320.));
        let before =
            (position - viewport.bounds.origin - viewport.origin(image)) / viewport.scale(image);
        viewport.zoom_by(0.4, position, image);
        let after =
            (position - viewport.bounds.origin - viewport.origin(image)) / viewport.scale(image);
        assert!((before.x - after.x).abs() < px(0.001));
        assert!((before.y - after.y).abs() < px(0.001));
    }

    #[test]
    fn navigation_is_bounded_and_presets_recenter() {
        let mut viewport = PreviewViewport {
            bounds: Bounds::new(Point::default(), size(px(360.), px(600.))),
            ..Default::default()
        };
        let image = size(px(720.), px(400.));
        let center = viewport.bounds.center();
        viewport.zoom_by(100., center, image);
        assert_eq!(viewport.scale(image), 4.);
        viewport.zoom_by(-1., center, image);
        assert_eq!(viewport.scale(image), 0.1);
        viewport.pan_by(point(px(100_000.), px(-100_000.)), image);
        let origin = viewport.origin(image);
        assert_eq!(origin.x, viewport.bounds.size.width - px(32.));
        assert_eq!(origin.y + image.height * viewport.scale(image), px(32.));
        viewport.start_drag(center);
        viewport.reset(PreviewZoom::Fit);
        assert_eq!(viewport.pan, Point::default());
        assert!(!viewport.is_dragging());
        // Pinching out from Fit must never zoom a very tall image in.
        let tall = size(px(720.), px(60_000.));
        let fit = viewport.scale(tall);
        viewport.zoom_by(-0.2, center, tall);
        assert_eq!(viewport.scale(tall), fit);
    }
}
