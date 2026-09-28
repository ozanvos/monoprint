//! Real GPUI controls and Metal pixels, without controlling the user's desktop.
#[path = "../src/appearance.rs"]
mod appearance;
#[allow(dead_code, unused_imports)]
#[path = "../src/canvas_resize.rs"]
mod canvas_resize;
#[allow(dead_code, unused_imports)] // Shared source also contains standard-harness tests.
#[path = "../src/document.rs"]
mod document;
#[cfg(target_os = "macos")]
#[allow(dead_code)] // The production raw-window adapter is unavailable on the test platform.
#[path = "../src/material.rs"]
mod material;
#[allow(unused_imports)]
#[path = "../src/preview.rs"]
mod preview;
#[allow(dead_code, unused_imports)]
#[path = "../src/selection.rs"]
mod selection;
#[allow(dead_code, unused_imports)]
#[path = "../src/source_highlight.rs"]
mod source_highlight;
#[path = "../src/source_lines.rs"]
mod source_lines;
#[path = "../src/studio.rs"]
mod studio;
#[path = "../src/view.rs"]
mod view;
#[allow(dead_code, unused_imports)]
#[path = "../src/viewport.rs"]
mod viewport;

fn main() {
    #[cfg(target_os = "macos")]
    macos::run();
    #[cfg(not(target_os = "macos"))]
    println!("Native pixel checks require GPUI's macOS headless Metal renderer.");
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use gpui_kit::{component::Root, prelude::*, test::TestWindowExt, *};
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    fn settle(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
                .unwrap();
            if cx.read_entity(studio, |s, _| s.preview_revision == s.revision) {
                break;
            }
            assert!(Instant::now() < deadline, "preview worker did not complete");
            std::thread::sleep(Duration::from_millis(10));
        }
        // Image decode and editor parsing finish on GPUI's executor after layout.
        cx.advance_clock(Duration::from_millis(100));
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
                .unwrap();
        }
    }

    fn capture(cx: &mut HeadlessAppContext, handle: WindowHandle<Root>, name: &str) {
        let pixels = cx
            .capture_screenshot(handle.into())
            .expect("Metal renderer must be available");
        if let Some(dir) = std::env::var_os("MONOPRINT_GUI_SCREENSHOTS") {
            std::fs::create_dir_all(&dir).unwrap();
            pixels
                .save(std::path::PathBuf::from(dir).join(format!("{name}.png")))
                .unwrap();
        }
    }

    fn assert_canvas_gestures(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let (bounds, image, revision, document, cached_image) = cx.read_entity(studio, |s, _| {
            let (cached, preview) = s.preview.as_ref().unwrap();
            (
                s.viewport.bounds,
                size(
                    px(preview.width as f32 / 2.),
                    px(preview.height as f32 / 2.),
                ),
                s.revision,
                serde_json::to_string(&s.document).unwrap(),
                cached.clone(),
            )
        });
        let anchor = bounds.center() + point(px(70.), px(-40.));
        // Native trackpad deltas pan freely in both axes, including momentum.
        for phase in [TouchPhase::Started, TouchPhase::Moved, TouchPhase::Ended] {
            cx.update_window(handle.into(), |_, window, cx| {
                window.dispatch_event(
                    ScrollWheelEvent {
                        position: anchor,
                        delta: ScrollDelta::Pixels(point(px(12.), px(-8.))),
                        touch_phase: phase,
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
            })
            .unwrap();
            settle(cx, handle, studio);
        }
        let image_point = cx.read_entity(studio, |s, _| {
            assert_eq!(s.viewport.pan, point(px(36.), px(-24.)));
            (anchor - bounds.origin - s.viewport.origin(image)) / s.viewport.scale(image)
        });
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(
                PinchEvent {
                    position: anchor,
                    delta: 0.5,
                    phase: TouchPhase::Moved,
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert!(matches!(s.viewport.zoom, viewport::PreviewZoom::Custom(_)));
            let after =
                (anchor - bounds.origin - s.viewport.origin(image)) / s.viewport.scale(image);
            assert!((after.x - image_point.x).abs() < px(0.01));
            assert!((after.y - image_point.y).abs() < px(0.01));
        });
        let pan = cx.read_entity(studio, |s, _| s.viewport.pan);
        cx.update_window(handle.into(), |_, window, cx| {
            window.drag(anchor, anchor + point(px(50.), px(30.)), cx);
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert!((s.viewport.pan.x - pan.x - px(50.)).abs() < px(0.01));
            assert!((s.viewport.pan.y - pan.y - px(30.)).abs() < px(0.01));
            assert!(!s.viewport.is_dragging());
        });
        capture(
            cx,
            handle,
            &format!("canvas-gestures-{}", f32::from(bounds.size.width) as u32),
        );
        // Capture continues over an adjacent pane and releases there cleanly.
        let pan = cx.read_entity(studio, |s, _| s.viewport.pan);
        let from = point(bounds.right() - px(10.), bounds.center().y);
        cx.update_window(handle.into(), |_, window, cx| {
            window.drag(from, from + point(px(30.), px(0.)), cx);
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert!((s.viewport.pan.x - pan.x - px(30.)).abs() < px(0.01));
            assert!(!s.viewport.is_dragging());
        });
        for preset in ["half-preview", "actual-preview", "fit-preview"] {
            click(cx, handle, studio, preset);
            cx.read_entity(studio, |s, _| assert_eq!(s.viewport.pan, Point::default()));
            if preset != "fit-preview" {
                cx.update_window(handle.into(), |_, window, cx| {
                    window.drag(anchor, anchor + point(px(20.), px(10.)), cx);
                })
                .unwrap();
                settle(cx, handle, studio);
            }
        }
        // A gesture over the source editor must not reach the canvas.
        let outside = point(bounds.left() - px(80.), anchor.y);
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(
                PinchEvent {
                    position: outside,
                    delta: 0.5,
                    phase: TouchPhase::Moved,
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
            window.dispatch_event(
                ScrollWheelEvent {
                    position: outside,
                    delta: ScrollDelta::Pixels(point(px(25.), px(25.))),
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert_eq!(s.viewport.zoom, viewport::PreviewZoom::Fit);
            assert_eq!(s.viewport.pan, Point::default());
            assert_eq!(s.revision, revision, "navigation must not request a render");
            assert_eq!(serde_json::to_string(&s.document).unwrap(), document);
            assert!(Arc::ptr_eq(&s.preview.as_ref().unwrap().0, &cached_image));
        });
    }

    fn assert_canvas_resizing(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        // Actual pointer hit targets, not calls to the resize methods.
        click(cx, handle, studio, "half-preview");
        let (source, initial_width, initial_height, initial_revision) =
            cx.read_entity(studio, |s, _| {
                (
                    s.document.source.clone(),
                    s.document.width.clone(),
                    s.preview.as_ref().unwrap().1.height,
                    s.revision,
                )
            });
        let right = cx
            .update_window(handle.into(), |_, window, _| {
                window.find("resize-image-right").bounds().center()
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            pointer_click(window, right, cx)
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert_eq!(
                s.revision, initial_revision,
                "a click must not resize or render"
            );
            assert!(s.canvas_resize.is_none());
            assert_eq!(s.viewport.zoom, viewport::PreviewZoom::Half);
        });
        let left_before = cx
            .update_window(handle.into(), |_, window, _| {
                window.find("resize-image-left").bounds().center().x
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.drag(right, right - point(px(110.), px(0.)), cx);
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.update_window(handle.into(), |_, window, cx| {
            let s = studio.read(cx);
            assert_eq!(
                s.document.width, "500",
                "110 screen px at 50% is 220 export px"
            );
            assert_eq!(s.width.read(cx).value().as_ref(), "500");
            assert_eq!(s.preview.as_ref().unwrap().1.width, 1000);
            assert!(
                s.preview.as_ref().unwrap().1.height > initial_height,
                "code must reflow rather than stretch"
            );
            assert!(s.canvas_resize.is_none());
            assert_eq!(s.document.source, source);
            assert!(
                (window.find("resize-image-left").bounds().center().x - left_before).abs() < px(1.)
            );
        })
        .unwrap();
        capture(
            cx,
            handle,
            &format!(
                "resized-canvas-{}",
                f32::from(cx.read_entity(studio, |s, _| s.viewport.bounds.size.width)) as u32
            ),
        );

        let (left, fixed_right) = cx
            .update_window(handle.into(), |_, window, _| {
                (
                    window.find("resize-image-left").bounds().center(),
                    window.find("resize-image-right").bounds().center().x,
                )
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.drag(left, left - point(px(40.), px(0.)), cx);
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.update_window(handle.into(), |_, window, cx| {
            let s = studio.read(cx);
            assert_eq!(s.document.width, "580");
            assert_eq!(s.preview.as_ref().unwrap().1.width, 1160);
            assert!(
                (window.find("resize-image-right").bounds().center().x - fixed_right).abs()
                    < px(1.)
            );
        })
        .unwrap();

        // Keep moving without the typing debounce's 90 ms pause. A completed
        // intermediate frame must arrive before mouse-up, even with newer sizes queued.
        let revision = cx.read_entity(studio, |s, _| s.revision);
        let start = cx
            .update_window(handle.into(), |_, window, cx| {
                let start = window.find("resize-image-right").bounds().center();
                window.dispatch_event(
                    MouseMoveEvent {
                        position: start,
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
                window.dispatch_event(
                    MouseDownEvent {
                        position: start,
                        button: MouseButton::Left,
                        click_count: 1,
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
                start
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut step = 0;
        let end = loop {
            let position = start + point(px(10. + (step % 15) as f32), px(0.));
            cx.update_window(handle.into(), |_, window, cx| {
                window.render_frame(cx);
                window.dispatch_event(
                    MouseMoveEvent {
                        position,
                        pressed_button: Some(MouseButton::Left),
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
            })
            .unwrap();
            cx.run_until_parked();
            let arrived = cx.read_entity(studio, |s, _| {
                assert!(s.canvas_resize.as_ref().unwrap().dragging);
                s.preview_revision > revision
            });
            if arrived {
                break position;
            }
            assert!(
                Instant::now() < deadline,
                "continuous resize must display intermediate renders"
            );
            std::thread::sleep(Duration::from_millis(10));
            step += 1;
        };
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    position: end,
                    button: MouseButton::Left,
                    click_count: 1,
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert!(s.canvas_resize.is_none());
            assert_eq!(
                s.preview.as_ref().unwrap().1.width,
                s.document.width.parse::<u32>().unwrap() * 2
            );
            assert_eq!(s.document.source, source);
            assert!(s.error.is_none());
        });
        // Restore the document and camera before the rest of the inspector tests.
        cx.update_window(handle.into(), |_, window, cx| {
            studio.update(cx, |s, cx| {
                s.width.update(cx, |field, cx| {
                    field.replace_all(initial_width.clone(), window, cx)
                })
            });
        })
        .unwrap();
        settle(cx, handle, studio);
        click(cx, handle, studio, "fit-preview");
    }

    fn assert_material_visible(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let (viewport, source_width, copy_button) = cx
            .update_window(handle.into(), |_, window, cx| {
                (
                    window.viewport_size(),
                    studio.read(cx).source_split.read(cx).sizes()[0],
                    window.find("copy-image").bounds().center(),
                )
            })
            .unwrap();
        // The headless renderer clears to opaque black. Compare two backing
        // colors to verify the real painted layers transmit the material below.
        let positions = [
            point(source_width + px(20.), px(70.)),
            point(viewport.width - px(5.), px(100.)),
            point(px(20.), px(70.)),
            copy_button,
        ];
        let mut samples = Vec::new();
        for backing in [0x000000, 0xFFFFFF] {
            handle
                .update(cx, |root, _, cx| {
                    root.style().background = Some(rgb(backing).into());
                    cx.notify();
                })
                .unwrap();
            settle(cx, handle, studio);
            let pixels = cx.capture_screenshot(handle.into()).unwrap();
            let scale = pixels.width() as f32 / f32::from(viewport.width);
            samples.push(positions.map(|position| {
                pixels
                    .get_pixel(
                        (f32::from(position.x) * scale) as u32,
                        (f32::from(position.y) * scale) as u32,
                    )
                    .0
            }));
        }
        handle
            .update(cx, |root, _, cx| {
                root.style().background = Some(transparent_black().into());
                cx.notify();
            })
            .unwrap();
        settle(cx, handle, studio);
        for index in 0..2 {
            assert!(
                samples[1][index][0].abs_diff(samples[0][index][0]) > 40,
                "preview and inspector must expose the material at {:?}",
                positions[index]
            );
        }
        assert_eq!(samples[0][2], samples[1][2], "editor stays opaque");
        assert_eq!(samples[0][3], samples[1][3], "controls stay opaque");
    }

    fn click(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
        id: &'static str,
    ) {
        cx.update_window(handle.into(), |_, window, cx| window.click(id, cx))
            .unwrap();
        settle(cx, handle, studio);
    }

    fn inspector_click(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
        id: impl Into<ElementId>,
    ) {
        let id = id.into();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let target = window.find(id.clone()).bounds();
            let scroll = &studio.read(cx).settings_scroll;
            scroll.set_offset(
                scroll.offset() + point(px(0.), scroll.bounds().center().y - target.center().y),
            );
            window.render_frame(cx);
            window.click(id, cx);
        })
        .unwrap();
        settle(cx, handle, studio);
    }

    fn reveal_theme_gallery(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let target = studio.read(cx).theme_scroll.0.borrow().base_handle.bounds();
            let scroll = &studio.read(cx).settings_scroll;
            scroll.set_offset(
                scroll.offset() + point(px(0.), scroll.bounds().center().y - target.center().y),
            );
            window.render_frame(cx);
        })
        .unwrap();
        settle(cx, handle, studio);
    }

    fn gutter_click(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
        line: usize,
    ) {
        cx.update_window(handle.into(), |_, window, cx| {
            let editor = studio.read(cx).editor.read(cx);
            let gutter = source_lines::visible_lines(editor)
                .into_iter()
                .find(|row| row.index + 1 == line)
                .unwrap()
                .gutter;
            let position = point(
                gutter.center().x,
                gutter.top() + editor.line_height().unwrap() / 2.,
            );
            pointer_click(window, position, cx);
        })
        .unwrap();
        settle(cx, handle, studio);
    }

    fn pointer_click(window: &mut Window, position: Point<Pixels>, cx: &mut App) {
        window.dispatch_event(
            MouseMoveEvent {
                position,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        window.dispatch_event(
            MouseDownEvent {
                position,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseUpEvent {
                position,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    }

    pub fn run() {
        // Headless pixels cannot capture the WindowServer backdrop. Exercise the
        // actual AppKit view's material, blending, ownership and resizing instead.
        {
            use objc2::{MainThreadMarker, MainThreadOnly};
            use objc2_app_kit::{
                NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
            };
            let main_thread = MainThreadMarker::new().unwrap();
            let host = NSView::initWithFrame(NSView::alloc(main_thread), Default::default());
            let gpui_view = NSView::initWithFrame(NSView::alloc(main_thread), Default::default());
            host.addSubview(&gpui_view);
            let material = material::attach(&host, main_thread);
            assert_eq!(
                material.material(),
                NSVisualEffectMaterial::UnderWindowBackground
            );
            assert_eq!(
                material.blendingMode(),
                NSVisualEffectBlendingMode::BehindWindow
            );
            assert_eq!(
                material.state(),
                NSVisualEffectState::FollowsWindowActiveState
            );
            let mut size = host.bounds().size;
            size.width = 640.;
            size.height = 480.;
            host.setFrameSize(size);
            assert_eq!(material.frame().size, host.bounds().size);
            assert!(
                std::ptr::eq::<NSView>(&*host.subviews().objectAtIndex(1), &*gpui_view),
                "GPUI must remain above the material"
            );
            drop(material);
            assert_eq!(
                host.subviews().len(),
                2,
                "native parent must retain the backdrop"
            );
            println!("AppKit frosted material, behind-window blending and autoresizing passed");
        }
        for (width, height) in [(1440., 900.), (1100., 700.)] {
            let mut cx = HeadlessAppContext::with_platform(
                gpui_kit::platform::current_platform(true).text_system(),
                Arc::new(gpui_kit::assets::Assets),
                gpui_kit::platform::current_headless_renderer,
            );
            cx.update(|cx| {
                gpui_kit::init(cx);
                appearance::init(cx);
            });
            let mut studio = None;
            let handle = cx
                .open_window(size(px(width), px(height)), |window, cx| {
                    let entity = cx.new(|cx| {
                        studio::Studio::new(
                            document::Document::default(),
                            None,
                            "Untitled snippet".into(),
                            None,
                            window,
                            cx,
                        )
                    });
                    studio = Some(entity.clone());
                    cx.new(|cx| Root::new(entity, window, cx).bg(transparent_black()))
                })
                .unwrap();
            let studio = studio.unwrap();
            cx.update_window(handle.into(), |_, window, _| window.activate_window())
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(s.preview.is_some(), "{:?}", s.error)
            });
            capture(&mut cx, handle, &format!("studio-{}", width as u32));
            assert_material_visible(&mut cx, handle, &studio);

            // The select's visible trigger and hit area must both fit the toolbar.
            // Select styles alone do not constrain its full-size outer wrapper.
            cx.update_window(handle.into(), |_, window, _| {
                let open = window.find("open-source").bounds();
                let language = window.find("source-language").bounds();
                let wrap = window.find("wrap-source").bounds();
                assert_eq!(language.size.height, px(26.));
                assert!(language.size.width <= px(80.));
                assert!(language.size.width >= px(40.));
                assert_eq!(language.center().y, open.center().y);
                assert_eq!(language.center().y, wrap.center().y);
                assert!(open.right() < language.left());
                assert!(language.right() < wrap.left());
                assert!(
                    open.left() >= px(76.),
                    "leave room for the native window controls"
                );
                let code = window.find("code-typeface").bounds();
                let comments = window.find("comment-typeface").bounds();
                assert_eq!(code.size.height, px(26.));
                assert_eq!(comments.size.height, px(26.));
                assert_eq!(code.left(), comments.left());
                assert_eq!(code.right(), comments.right());
            })
            .unwrap();
            let (original_split, original_document, original_revision, original_image) = cx
                .read_entity(&studio, |s, cx| {
                    (
                        s.source_split.read(cx).sizes()[0],
                        serde_json::to_string(&s.document).unwrap(),
                        s.revision,
                        s.preview.as_ref().unwrap().0.clone(),
                    )
                });
            let maximum = if width == 1440. { 560. } else { 480. };
            for (target, expected, label) in [
                (440., 440., "wide"),
                (2000., maximum, "max"),
                (0., 240., "min"),
                // Move far enough to cross GPUI's drag threshold before restoring
                // the narrow window's default (only 2 px above the minimum).
                (440., 440., "reexpanded"),
                (
                    f32::from(original_split),
                    f32::from(original_split),
                    "restored",
                ),
            ] {
                cx.update_window(handle.into(), |_, window, cx| {
                    let current = studio.read(cx).source_split.read(cx).sizes()[0];
                    window.drag(
                        point(current, px(height / 2.)),
                        point(px(target), px(height / 2.)),
                        cx,
                    );
                })
                .unwrap();
                settle(&mut cx, handle, &studio);
                cx.update_window(handle.into(), |_, window, cx| {
                    // Measure the rendered toolbar as well as the split's state.
                    let source_right = window.find("wrap-source").bounds().right() + px(9.);
                    assert!(
                        (f32::from(source_right) - expected).abs() < 1.,
                        "{label}: {source_right:?}"
                    );
                    let s = studio.read(cx);
                    assert!(s.source_split.read(cx).sizes()[1] >= px(359.));
                    assert_eq!(
                        s.revision, original_revision,
                        "resizing must not render an export"
                    );
                    assert_eq!(
                        serde_json::to_string(&s.document).unwrap(),
                        original_document
                    );
                    assert!(Arc::ptr_eq(&s.preview.as_ref().unwrap().0, &original_image));
                })
                .unwrap();
                if matches!(label, "wide" | "max" | "min") {
                    capture(&mut cx, handle, &format!("source-{label}-{}", width as u32));
                }
            }
            click(&mut cx, handle, &studio, "wrap-source");
            cx.read_entity(&studio, |s, _| assert!(!s.wrap));
            click(&mut cx, handle, &studio, "wrap-source");
            cx.read_entity(&studio, |s, _| assert!(s.wrap));

            click(&mut cx, handle, &studio, "source-language");
            std::thread::sleep(Duration::from_millis(180));
            settle(&mut cx, handle, &studio);
            capture(&mut cx, handle, &format!("language-menu-{}", width as u32));
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);

            let original = cx.read_entity(&studio, |s, _| {
                (s.revision, serde_json::to_string(&s.document).unwrap())
            });
            click(&mut cx, handle, &studio, "half-preview");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.viewport.zoom, viewport::PreviewZoom::Half)
            });
            capture(&mut cx, handle, &format!("half-preview-{}", width as u32));
            click(&mut cx, handle, &studio, "actual-preview");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.viewport.zoom, viewport::PreviewZoom::Actual)
            });
            click(&mut cx, handle, &studio, "fit-preview");
            assert_canvas_gestures(&mut cx, handle, &studio);
            click(&mut cx, handle, &studio, "ui-appearance");
            cx.update(|cx| assert!(appearance::palette(cx).light));
            assert_material_visible(&mut cx, handle, &studio);
            capture(
                &mut cx,
                handle,
                &format!("light-interface-{}", width as u32),
            );
            click(&mut cx, handle, &studio, "ui-appearance");
            cx.update(|cx| assert!(!appearance::palette(cx).light));
            cx.read_entity(&studio, |s, _| {
                assert_eq!(
                    (s.revision, serde_json::to_string(&s.document).unwrap()),
                    original,
                    "interface appearance and zoom must not change the export or request renders"
                );
            });
            assert_canvas_resizing(&mut cx, handle, &studio);
            inspector_click(&mut cx, handle, &studio, "code-typeface");
            std::thread::sleep(Duration::from_millis(180));
            settle(&mut cx, handle, &studio);
            capture(
                &mut cx,
                handle,
                &format!("typeface-previews-{}", width as u32),
            );
            cx.update_window(handle.into(), |_, window, cx| {
                window.input("Commit Mono", cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| window.press("enter", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.font_id.as_deref(), Some("commit-mono"))
            });
            inspector_click(&mut cx, handle, &studio, "theme-font");
            cx.read_entity(&studio, |s, _| assert!(s.document.font_id.is_none()));
            reveal_theme_gallery(&mut cx, handle, &studio);
            let revision = cx.read_entity(&studio, |s, _| s.revision);
            inspector_click(&mut cx, handle, &studio, "themes-light");
            cx.read_entity(&studio, |s, _| {
                let expected = monoprint::Theme::all()
                    .filter(|theme| theme.supports(monoprint::Appearance::Light))
                    .count();
                assert_eq!(s.filtered_themes.len(), expected);
                assert_eq!(s.light_thumbnails.len(), expected);
                assert!(
                    s.filtered_themes
                        .iter()
                        .all(|&index| s.thumbnails[index].0.supports(monoprint::Appearance::Light))
                );
                assert_eq!(s.revision, revision);
            });
            reveal_theme_gallery(&mut cx, handle, &studio);
            capture(
                &mut cx,
                handle,
                &format!("light-theme-filter-{}", width as u32),
            );
            let candy_variant = cx.read_entity(&studio, |s, _| {
                s.thumbnails
                    .iter()
                    .position(|(theme, _)| theme.id() == "removed-frame")
                    .unwrap()
            });
            cx.update_window(handle.into(), |_, window, cx| {
                window.click(("theme-swatch", candy_variant), cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(
                    s.document.light,
                    "selecting a light thumbnail must use its displayed variant"
                )
            });
            inspector_click(&mut cx, handle, &studio, "themes-dark");
            reveal_theme_gallery(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| {
                window.click(("theme-swatch", candy_variant), cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| assert!(!s.document.light));
            inspector_click(&mut cx, handle, &studio, "themes-light");
            let revision = cx.read_entity(&studio, |s, _| s.revision);
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.theme_search
                        .update(cx, |input, cx| input.replace_all("Ayu", window, cx))
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.filtered_themes.len(), 1);
                assert_eq!(s.thumbnails[s.filtered_themes[0]].0.id(), "ayu-light");
            });
            inspector_click(&mut cx, handle, &studio, "themes-dark");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.filtered_themes.len(), 2);
                assert_eq!(s.revision, revision);
            });
            inspector_click(&mut cx, handle, &studio, "themes-all");
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.theme_search
                        .update(cx, |input, cx| input.replace_all("", window, cx))
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            reveal_theme_gallery(&mut cx, handle, &studio);

            let (candy, last, initial_revision) = cx.read_entity(&studio, |s, _| {
                let mut bundled = s
                    .thumbnails
                    .iter()
                    .map(|(theme, _)| theme.id())
                    .collect::<Vec<_>>();
                bundled.sort();
                assert_eq!(
                    bundled,
                    monoprint::Theme::all()
                        .map(|theme| theme.id())
                        .collect::<Vec<_>>()
                );
                (
                    s.thumbnails
                        .iter()
                        .position(|(theme, _)| theme.id() == "removed-frame")
                        .unwrap(),
                    s.thumbnails.len() - 1,
                    s.revision,
                )
            });
            cx.update_window(handle.into(), |_, window, cx| {
                assert!(window.find(("theme-swatch", candy)).visible());
                assert!(window.find(("theme-swatch", candy)).bounds().size.width <= px(128.));
                assert!(
                    window.try_find(("theme-swatch", last)).is_none(),
                    "offscreen themes should not build views"
                );
                window.scroll(
                    ("theme-swatch", candy),
                    ScrollDelta::Pixels(point(px(0.), px(-5000.))),
                    cx,
                );
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.find(("theme-swatch", last)).visible());
                assert!(window.try_find(("theme-swatch", candy)).is_none());
            })
            .unwrap();
            capture(
                &mut cx,
                handle,
                &format!("themes-scrolled-{}", width as u32),
            );
            cx.read_entity(&studio, |s, _| {
                assert_eq!(
                    s.revision, initial_revision,
                    "browsing must not request a render"
                )
            });
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.theme_search
                        .update(cx, |input, cx| input.replace_all("  eViL  ", window, cx));
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            let rabbit = cx.read_entity(&studio, |s, _| {
                assert_eq!(s.filtered_themes.len(), 1);
                let index = s.filtered_themes[0];
                assert_eq!(s.thumbnails[index].0.id(), "removed-frame");
                assert_eq!(s.revision, initial_revision);
                index
            });
            cx.update_window(handle.into(), |_, window, cx| {
                window.click(("theme-swatch", rabbit), cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| assert_eq!(s.document.theme, "removed-frame"));
            capture(
                &mut cx,
                handle,
                &format!("themes-filtered-{}", width as u32),
            );
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.theme_search.update(cx, |input, cx| {
                        input.replace_all("no matching theme", window, cx)
                    });
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| assert!(s.filtered_themes.is_empty()));
            capture(&mut cx, handle, &format!("themes-empty-{}", width as u32));
            cx.update_window(handle.into(), |_, window, cx| {
                window.click("clear-theme-search", cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.filtered_themes.len(), s.thumbnails.len())
            });
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.theme_scroll
                        .scroll_to_item(candy / 3, ScrollStrategy::Center);
                    s.select_theme("removed-frame".parse().unwrap(), window, cx);
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);

            // Copy updates only its own button, writes an image, and resets its
            // feedback timeout on a second copy instead of flashing a banner.
            for attempt in 0..2 {
                cx.update_window(handle.into(), |_, window, cx| {
                    window.click("copy-image", cx);
                    studio.read_with(cx, |s, _| {
                        assert_eq!(s.copy_state, studio::CopyState::Copying);
                        assert!(!s.exporting);
                    });
                })
                .unwrap();
                cx.run_until_parked();
                cx.read_entity(&studio, |s, cx| {
                    assert_eq!(s.copy_state, studio::CopyState::Copied);
                    assert!(s.notice.is_none());
                    assert!(matches!(
                        cx.read_from_clipboard().unwrap().entries(),
                        [ClipboardEntry::Image(_)]
                    ));
                });
                cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
                    .unwrap();
                if attempt == 0 {
                    capture(&mut cx, handle, &format!("copied-{}", width as u32));
                }
                cx.advance_clock(Duration::from_secs(1));
                cx.run_until_parked();
                cx.read_entity(&studio, |s, _| {
                    assert_eq!(s.copy_state, studio::CopyState::Copied)
                });
            }
            cx.advance_clock(Duration::from_secs(1));
            cx.run_until_parked();
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.copy_state, studio::CopyState::Idle)
            });

            cx.update_window(handle.into(), |_, window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string("const pasted = 7;".into()));
                window.click("paste-source", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            inspector_click(&mut cx, handle, &studio, ("padding", 32u32));
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.source, "const pasted = 7;");
                assert_eq!(s.document.padding, "32");
            });
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.install_source("greet.rb".into(), "# Monochrome, using Giallo\ndef greet(name)\n  puts \"Hello, #{name}!\"\nend\n".into(), window, cx)
                });
            }).unwrap();
            settle(&mut cx, handle, &studio);
            capture(&mut cx, handle, &format!("source-ruby-{}", width as u32));
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.install_source("main.rs".into(), "fn main() {}".into(), window, cx)
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.source, "fn main() {}");
                assert_eq!(s.document.language, "rust");
                assert_eq!(s.document.title, "main.rs");
            });

            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.find("export-image").visible());
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            inspector_click(&mut cx, handle, &studio, "light-appearance");
            cx.read_entity(&studio, |s, _| assert!(s.document.light));
            inspector_click(&mut cx, handle, &studio, "dark-appearance");
            cx.read_entity(&studio, |s, _| assert!(!s.document.light));
            inspector_click(&mut cx, handle, &studio, "light-appearance");
            inspector_click(&mut cx, handle, &studio, "background-none");
            cx.read_entity(&studio, |s, _| {
                assert!(s.document.light);
                assert_eq!(
                    s.document.background().unwrap(),
                    monoprint::Background::None
                );
                assert!(s.error.is_none());
            });
            capture(&mut cx, handle, &format!("transparent-{}", width as u32));
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.select_theme("catppuccin-latte".parse().unwrap(), window, cx)
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            for background in monoprint::Background::ALL {
                cx.update_window(handle.into(), |_, window, cx| {
                    window.click(format!("background-{background}"), cx);
                })
                .unwrap();
                settle(&mut cx, handle, &studio);
                cx.read_entity(&studio, |s, _| {
                    assert!(s.document.light);
                    assert_eq!(s.document.background().unwrap(), background);
                    assert!(s.error.is_none());
                });
                capture(
                    &mut cx,
                    handle,
                    &format!("latte-{background}-{}", width as u32),
                );
            }
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.select_theme("removed-frame".parse().unwrap(), window, cx)
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(!s.document.light);
                assert_eq!(s.document.theme, "removed-frame");
            });
            let revision = cx.read_entity(&studio, |s, _| s.revision);
            inspector_click(&mut cx, handle, &studio, "light-appearance");
            cx.read_entity(&studio, |s, _| {
                assert!(
                    !s.document.light,
                    "an unsupported variant must stay disabled"
                );
                assert_eq!(s.revision, revision);
            });
            // Restore the inspector's top before checking the disabled appearance.
            cx.update_window(handle.into(), |_, window, cx| {
                studio
                    .read(cx)
                    .settings_scroll
                    .set_offset(point(px(0.), px(0.)));
                window.render_frame(cx);
            })
            .unwrap();
            cx.update_window(handle.into(), |_, window, cx| {
                window.click("svg", cx);
                studio.update(cx, |s, cx| {
                    s.width
                        .update(cx, |input, cx| input.replace_all("90", window, cx));
                    s.editor.update(cx, |editor, cx| {
                        editor.replace_all("const current = 42;\n", window, cx)
                    });
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(
                    !s.document.light,
                    "Stripe's disabled appearance must reject clicks"
                );
                assert_eq!(s.document.format, document::Format::Svg);
                assert_eq!(s.document.source, "const current = 42;\n");
                assert!(s.error.as_ref().unwrap().contains("Width"));
            });
            cx.update_window(handle.into(), |_, window, cx| {
                window.click("copy-image", cx)
            })
            .unwrap();
            cx.run_until_parked();
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.copy_state, studio::CopyState::Idle);
                assert!(s.notice.as_ref().unwrap().contains("Could not copy image"));
            });
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.width
                        .update(cx, |input, cx| input.replace_all("720", window, cx))
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| assert!(s.error.is_none(), "{:?}", s.error));

            // Advanced controls use the same renderer for preview and export.
            // Selection coordinates must remain Unicode-aware after editor blur.
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.install_source(
                        std::path::PathBuf::from("example.rs"),
                        "// Another typeface\nlet café = 10;\nlet ok = café != 0;\n".into(),
                        window,
                        cx,
                    );
                    s.select_theme("removed-frame".parse().unwrap(), window, cx);
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("edit-highlight-ranges").is_none());
            })
            .unwrap();
            for (id, query) in [
                ("code-typeface", "Fira Code"),
                ("comment-typeface", "Space Mono"),
            ] {
                inspector_click(&mut cx, handle, &studio, id);
                // Dropdown entry uses the renderer's wall clock, not the test executor clock.
                std::thread::sleep(Duration::from_millis(180));
                settle(&mut cx, handle, &studio);
                capture(&mut cx, handle, &format!("{id}-menu-{}", width as u32));
                cx.update_window(handle.into(), |_, window, cx| window.input(query, cx))
                    .unwrap();
                settle(&mut cx, handle, &studio);
                cx.update_window(handle.into(), |_, window, cx| window.press("enter", cx))
                    .unwrap();
                settle(&mut cx, handle, &studio);
            }
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.font_id.as_deref(), Some("fira-code"));
                assert_eq!(s.document.comment_font_id.as_deref(), Some("space-mono"));
                assert!(s.document.font_path.is_none());
                assert!(s.document.comment_font_path.is_none());
            });
            inspector_click(&mut cx, handle, &studio, "coding-ligatures");
            cx.read_entity(&studio, |s, _| {
                assert!(s.document.ligatures);
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            capture(&mut cx, handle, &format!("advanced-fonts-{}", width as u32));
            let revision = cx.read_entity(&studio, |s, _| s.revision);
            // Use actual pointer events: selecting text alone must not render an image.
            cx.update_window(handle.into(), |_, window, cx| {
                let s = studio.read(cx);
                let start = s.document.source.find("café").unwrap();
                let editor = s.editor.read(cx);
                let bounds = editor
                    .range_to_bounds(&(start..start + "café".len()))
                    .unwrap();
                window.drag(
                    point(bounds.left() + px(0.1), bounds.center().y),
                    point(bounds.right() + px(0.1), bounds.center().y),
                    cx,
                );
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, cx| {
                assert_eq!(s.editor.read(cx).selected_text().to_string(), "café");
                assert_eq!(s.revision, revision);
            });
            capture(
                &mut cx,
                handle,
                &format!("selection-toolbar-{}", width as u32),
            );
            click(&mut cx, handle, &studio, "highlight-selection");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.text_highlights, "2:5-2:9")
            });
            // The same contextual action removes a marked selection, then adds it again.
            click(&mut cx, handle, &studio, "highlight-selection");
            cx.read_entity(&studio, |s, cx| {
                assert!(s.document.text_highlights.is_empty());
                assert!(s.source_marks.get_ranges(cx).is_empty());
            });
            click(&mut cx, handle, &studio, "highlight-selection");
            // Native decorations follow edits; persisted/export positions must match.
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.editor.update(cx, |editor, cx| {
                        editor.set_selected_range(0..0, cx);
                        editor.focus(window, cx);
                    });
                });
                window.input("\n", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, cx| {
                assert_eq!(s.document.text_highlights, "3:5-3:9");
                let marks = s.source_marks.get_ranges(cx);
                assert_eq!(&s.document.source[marks[0].clone()], "café");
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            cx.update_window(handle.into(), |_, window, cx| {
                window.dispatch_action(Box::new(gpui_kit::component::input::Undo), cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.text_highlights, "2:5-2:9");
            });
            capture(
                &mut cx,
                handle,
                &format!("persistent-highlights-{}", width as u32),
            );
            // A click on the right half of a Unicode glyph opens removal without
            // selecting code, even though the caret lands at the exclusive end.
            let click_revision = cx.read_entity(&studio, |s, _| s.revision);
            for click_highlight in [true, false, true] {
                cx.update_window(handle.into(), |_, window, cx| {
                    let s = studio.read(cx);
                    let offset = s.document.source.find("café").unwrap()
                        + if click_highlight { 3 } else { "café".len() };
                    let end = offset + if click_highlight { "é".len() } else { 1 };
                    let bounds = s.editor.read(cx).range_to_bounds(&(offset..end)).unwrap();
                    pointer_click(
                        window,
                        point(bounds.left() + bounds.size.width * 0.75, bounds.center().y),
                        cx,
                    );
                })
                .unwrap();
                settle(&mut cx, handle, &studio);
                cx.read_entity(&studio, |s, cx| {
                    assert_eq!(s.highlight_menu.is_some(), click_highlight);
                    assert!(s.editor.read(cx).selected_range().is_empty());
                    assert_eq!(s.document.text_highlights, "2:5-2:9");
                    assert_eq!(
                        s.revision, click_revision,
                        "opening or dismissing a menu must not render an image"
                    );
                });
            }
            capture(
                &mut cx,
                handle,
                &format!("clicked-highlight-menu-{}", width as u32),
            );
            let before_removal = cx.read_entity(&studio, |s, cx| {
                (s.document.source.clone(), s.editor.read(cx).cursor())
            });
            click(&mut cx, handle, &studio, "highlight-selection");
            cx.read_entity(&studio, |s, cx| {
                assert!(s.document.text_highlights.is_empty());
                assert!(s.highlight_menu.is_none());
                assert_eq!(s.document.source, before_removal.0);
                assert_eq!(s.editor.read(cx).cursor(), before_removal.1);
            });
            cx.update_window(handle.into(), |_, _, cx| {
                studio.update(cx, |s, cx| {
                    let start = s.document.source.find("café").unwrap();
                    s.editor.update(cx, |editor, cx| {
                        editor.set_selected_range(start..start + "café".len(), cx)
                    });
                    s.highlight_selection(false, cx);
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| {
                let s = studio.read(cx);
                let start = s.document.source.find("café").unwrap();
                let bounds = s
                    .editor
                    .read(cx)
                    .range_to_bounds(&(start..start + 2))
                    .unwrap();
                window.drag(
                    point(bounds.left() + px(0.1), bounds.center().y),
                    point(bounds.right() + px(0.1), bounds.center().y),
                    cx,
                );
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, cx| {
                assert!(
                    s.highlight_menu.is_none(),
                    "dragging retains normal text selection"
                );
                assert_eq!(s.editor.read(cx).selected_text().to_string(), "ca");
            });
            cx.update_window(handle.into(), |_, _, cx| {
                studio.update(cx, |s, cx| {
                    let end = s.document.source.find("let ok").unwrap();
                    s.editor
                        .update(cx, |editor, cx| editor.set_selected_range(0..end, cx));
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            click(&mut cx, handle, &studio, "highlight-selected-lines");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(
                    s.document.highlights, "1-2",
                    "a selection ending at column one excludes that line"
                );
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            click(&mut cx, handle, &studio, "highlight-selected-lines");
            cx.read_entity(&studio, |s, _| assert!(s.document.highlights.is_empty()));
            click(&mut cx, handle, &studio, "highlight-selected-lines");
            cx.update_window(handle.into(), |_, _, cx| {
                studio
                    .read(cx)
                    .editor
                    .clone()
                    .update(cx, |editor, cx| editor.set_selected_range(0..0, cx));
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            let sample_points = cx.read_entity(&studio, |s, cx| {
                let row = source_lines::visible_lines(s.editor.read(cx))
                    .into_iter()
                    .find(|row| row.index == 1)
                    .unwrap();
                [
                    point(row.gutter.left() + px(4.), row.gutter.center().y),
                    point(row.bounds.right() - px(4.), row.bounds.center().y),
                ]
            });
            let marked = cx.capture_screenshot(handle.into()).unwrap();
            gutter_click(&mut cx, handle, &studio, 2);
            cx.read_entity(&studio, |s, cx| {
                assert_eq!(s.document.highlights, "1");
                assert_eq!(
                    s.document.text_highlights, "2:5-2:9",
                    "gutter toggles preserve exact marks"
                );
                assert_eq!(
                    s.editor.read(cx).selected_range(),
                    0..0,
                    "gutter toggles do not move the caret"
                );
            });
            let unmarked = cx.capture_screenshot(handle.into()).unwrap();
            let scale = marked.width() as f32 / width;
            for point in sample_points {
                let (x, y) = (
                    (f32::from(point.x) * scale) as u32,
                    (f32::from(point.y) * scale) as u32,
                );
                assert_ne!(
                    marked.get_pixel(x, y),
                    unmarked.get_pixel(x, y),
                    "line tint must cover both gutter and empty code-row space"
                );
            }
            gutter_click(&mut cx, handle, &studio, 2);
            gutter_click(&mut cx, handle, &studio, 4);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(
                    s.document.highlights, "1-2, 4",
                    "blank lines can be highlighted"
                );
            });
            capture(
                &mut cx,
                handle,
                &format!("source-highlights-{}", width as u32),
            );
            click(&mut cx, handle, &studio, "clear-highlights");
            cx.read_entity(&studio, |s, _| {
                assert!(s.document.highlights.is_empty());
                assert!(s.document.text_highlights.is_empty());
            });
            inspector_click(&mut cx, handle, &studio, "reset-comment-font");
            inspector_click(&mut cx, handle, &studio, "theme-font");
            cx.read_entity(&studio, |s, _| {
                assert!(s.document.font_path.is_none());
                assert!(s.document.comment_font_path.is_none());
                assert!(s.document.font_id.is_none());
                assert!(s.document.comment_font_id.is_none());
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            cx.update_window(handle.into(), |_, _, cx| {
                studio.update(cx, |s, cx| {
                    let start = s.document.source.find("café").unwrap();
                    s.editor.update(cx, |editor, cx| {
                        editor.set_selected_range(start..start + "café".len(), cx)
                    });
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            click(&mut cx, handle, &studio, "highlight-selection");
            cx.read_entity(&studio, |s, _| {
                assert!(!s.document.text_highlights.is_empty())
            });
            cx.update_window(handle.into(), |_, window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string("replacement".into()));
                window.click("paste-source", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.source, "replacement");
                assert!(s.document.text_highlights.is_empty());
                assert!(s.document.highlights.is_empty());
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            // Keep the toolbar attached to a visible selection in a scrolled document.
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.install_source("long.rs".into(), "let value = 42;\n".repeat(80), window, cx);
                    s.editor.update(cx, |editor, cx| {
                        editor.set_scroll_offset(point(px(0.), px(-10000.)), cx);
                        editor.focus(window, cx);
                    });
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, _, cx| {
                studio.update(cx, |s, cx| {
                    let start = s.document.source.rfind("value").unwrap();
                    s.editor.update(cx, |editor, cx| {
                        editor.set_selected_range(start..start + 5, cx);
                    });
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| {
                let editor = studio.read(cx).editor.read(cx);
                assert!(editor.scroll_offset().y < px(0.));
                let viewport = editor.input_bounds();
                let toolbar = window.find("highlight-selection").bounds();
                assert!(viewport.contains(&toolbar.center()));
            })
            .unwrap();
            click(&mut cx, handle, &studio, "highlight-selection");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.text_highlights, "80:5-80:10")
            });
            cx.update_window(handle.into(), |_, window, cx| {
                let s = studio.read(cx);
                let start = s.document.source.rfind("value").unwrap();
                let bounds = s
                    .editor
                    .read(cx)
                    .range_to_bounds(&(start..start + 1))
                    .unwrap();
                pointer_click(window, bounds.center(), cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(
                    s.highlight_menu.is_some(),
                    "clicking a scrolled mark opens its menu"
                )
            });
            gutter_click(&mut cx, handle, &studio, 80);
            cx.read_entity(&studio, |s, _| assert_eq!(s.document.highlights, "80"));
            capture(
                &mut cx,
                handle,
                &format!("scrolled-selection-{}", width as u32),
            );
            gutter_click(&mut cx, handle, &studio, 80);
            cx.read_entity(&studio, |s, _| assert!(s.document.highlights.is_empty()));

            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.install_source(
                        "wrapped.rs".into(),
                        format!(
                            "// {}\n\nlet second = true;",
                            "long café comment ".repeat(10)
                        ),
                        window,
                        cx,
                    );
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, cx| {
                let editor = s.editor.read(cx);
                let rows = source_lines::visible_lines(editor);
                assert!(rows[0].bounds.size.height > editor.line_height().unwrap() * 2.);
                assert_eq!(rows[1].index, 1);
            });
            gutter_click(&mut cx, handle, &studio, 1);
            gutter_click(&mut cx, handle, &studio, 2);
            cx.read_entity(&studio, |s, _| assert_eq!(s.document.highlights, "1-2"));
            capture(
                &mut cx,
                handle,
                &format!("wrapped-line-highlights-{}", width as u32),
            );
            click(&mut cx, handle, &studio, "wrap-source");
            gutter_click(&mut cx, handle, &studio, 2);
            cx.read_entity(&studio, |s, _| assert_eq!(s.document.highlights, "1"));
            cx.update_window(handle.into(), |_, _, cx| {
                studio.read(cx).editor.clone().update(cx, |editor, cx| {
                    editor.set_scroll_offset(point(px(-100.), px(0.)), cx)
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            gutter_click(&mut cx, handle, &studio, 1);
            cx.read_entity(&studio, |s, _| assert!(s.document.highlights.is_empty()));
            println!(
                "Studio {}×{}: render, sidebar resizing, virtual theme scrolling/filtering, transparency, format, source highlighting, copy feedback, typography, interface appearance, canvas pan/pinch/drag/live resize, source selections, validation and recovery passed",
                width, height
            );
        }
    }
}
