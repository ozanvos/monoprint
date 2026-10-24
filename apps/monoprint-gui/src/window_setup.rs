#[cfg(any(target_os = "linux", test))]
use gpui_kit::point;
use gpui_kit::{App, Bounds, Pixels, Size, px, size};

pub fn initial_bounds(cx: &App) -> (Bounds<Pixels>, Size<Pixels>) {
    #[cfg(target_os = "linux")]
    {
        let available = cx
            .primary_display()
            .map(|display| display.visible_bounds())
            .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), size(px(1280.), px(800.))));
        fit_display(available)
    }
    #[cfg(not(target_os = "linux"))]
    {
        (
            Bounds::centered(None, size(px(1440.), px(900.)), cx),
            size(px(1100.), px(700.)),
        )
    }
}

#[cfg(any(target_os = "linux", test))]
fn fit_display(available: Bounds<Pixels>) -> (Bounds<Pixels>, Size<Pixels>) {
    // Leave room for shell chrome and the Root's client decoration insets even
    // on compositors which report the entire output as the visible bounds.
    let initial = size(px(1280.), px(800.)).min(&size(
        available.size.width * 0.9,
        available.size.height * 0.9,
    ));
    let minimum = size(px(940.), px(620.)).min(&initial);
    (Bounds::centered_at(available.center(), initial), minimum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_and_minimum_bounds_fit_small_and_offset_displays() {
        for extent in [
            size(px(1024.), px(768.)),
            size(px(800.), px(600.)),
            size(px(3840.), px(2160.)),
        ] {
            let available = Bounds::new(point(px(1700.), px(40.)), extent);
            let (window, minimum) = fit_display(available);
            assert!(window.origin.x > available.origin.x);
            assert!(window.origin.y > available.origin.y);
            assert!(window.right() < available.right());
            assert!(window.bottom() < available.bottom());
            assert!(minimum.width <= window.size.width && minimum.height <= window.size.height);
        }
    }
}
