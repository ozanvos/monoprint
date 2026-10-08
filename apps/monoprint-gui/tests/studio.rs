//! Real GPUI controls and Metal pixels, without controlling the user's desktop.
#[path = "../src/appearance.rs"]
mod appearance;
#[path = "../src/assets.rs"]
mod assets;
#[allow(dead_code, unused_imports)]
#[path = "../src/canvas_resize.rs"]
mod canvas_resize;
#[allow(dead_code, unused_imports)] // Shared source also contains standard-harness tests.
#[path = "../src/document.rs"]
mod document;
#[allow(dead_code, unused_imports)]
#[path = "../src/handoff.rs"]
mod handoff;
#[allow(dead_code, unused_imports)]
#[path = "../src/interaction_motion.rs"]
mod interaction_motion;
#[allow(dead_code, unused_imports)]
#[path = "../src/language_picker.rs"]
mod language_picker;
#[cfg(target_os = "macos")]
#[allow(dead_code)] // The production raw-window adapter is unavailable on the test platform.
#[path = "../src/material.rs"]
mod material;
#[path = "../src/platform_keys.rs"]
mod platform_keys;
#[allow(dead_code, unused_imports)] // Standard-harness preview tests are not run by this binary.
#[path = "../src/preview.rs"]
mod preview;
#[allow(dead_code, unused_imports)]
#[path = "../src/sandbox.rs"]
mod sandbox;
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
#[allow(dead_code, unused_imports)]
#[path = "../src/updates/mod.rs"]
mod updates;
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
        // Editor parsing and theme thumbnail loading finish after layout.
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

    fn source_colored_pixels(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) -> usize {
        let (bounds, viewport) = cx
            .update_window(handle.into(), |_, window, cx| {
                window.render_frame(cx);
                (
                    studio.read(cx).editor.read(cx).input_bounds(),
                    window.viewport_size(),
                )
            })
            .unwrap();
        let pixels = cx.capture_screenshot(handle.into()).unwrap();
        let scale = pixels.width() as f32 / f32::from(viewport.width);
        let left = (f32::from(bounds.left()) * scale) as u32;
        let right = (f32::from(bounds.right()) * scale) as u32;
        let top = (f32::from(bounds.top()) * scale) as u32;
        let bottom = (f32::from(bounds.bottom()) * scale) as u32;
        let mut colored = 0;
        for y in top..bottom.min(pixels.height()) {
            for x in left..right.min(pixels.width()) {
                let [r, g, b, _] = pixels.get_pixel(x, y).0;
                if r.max(g).max(b) - r.min(g).min(b) > 25 {
                    colored += 1;
                }
            }
        }
        colored
    }

    fn assert_source_syntax_colored(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            settle(cx, handle, studio);
            let colored = source_colored_pixels(cx, handle, studio);
            if colored > 100 {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "source editor must visibly render colored syntax; found {colored} colored pixels"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn assert_syntax_survives_typing(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        assert_source_syntax_colored(cx, handle, studio);
        let original = cx.read_entity(studio, |s, _| s.document.source.clone());
        let offset = original.find("scale").unwrap() + "scale".len();
        cx.update_window(handle.into(), |_, window, cx| {
            let editor = studio.read(cx).editor.clone();
            editor.update(cx, |editor, cx| {
                editor.set_selected_range(offset..offset, cx);
                editor.focus(window, cx);
            });
        })
        .unwrap();
        for text in ["_", "c", "a", "f", "é"] {
            cx.update_window(handle.into(), |_, window, cx| window.input(text, cx))
                .unwrap();
            // Inspect the immediate frame, before advancing the debounce clock.
            assert!(
                source_colored_pixels(cx, handle, studio) > 100,
                "typing must not clear source syntax colors"
            );
        }
        assert_source_syntax_colored(cx, handle, studio);
        cx.update_window(handle.into(), |_, window, cx| {
            let editor = studio.read(cx).editor.clone();
            editor.update(cx, |editor, cx| {
                editor.replace_all(original, window, cx);
            });
        })
        .unwrap();
        assert_source_syntax_colored(cx, handle, studio);
    }

    fn assert_handoffs(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("示例 #1.py");
        let second = directory.path().join("drop.rs");
        let invalid = directory.path().join("binary.png");
        std::fs::write(&first, "print('hello')").unwrap();
        std::fs::write(&second, "fn dropped() {}").unwrap();
        std::fs::write(&invalid, [0xff, 0xfe]).unwrap();
        let (sender, incoming) = async_channel::unbounded();
        cx.update_window(handle.into(), |_, window, cx| {
            studio.update(cx, |s, cx| s.listen_for_sources(incoming, window, cx));
        })
        .unwrap();
        sender
            .try_send(vec![url::Url::from_file_path(&first).unwrap().into()])
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while cx.read_entity(studio, |s, _| s.document.source != "print('hello')") {
            cx.run_until_parked();
            assert!(
                Instant::now() < deadline,
                "open-document event not delivered"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert_eq!(s.document.language, "python");
            assert_eq!(s.document.title, "示例 #1.py");
        });
        cx.update_window(handle.into(), |_, window, cx| {
            let position = point(px(150.), px(180.));
            assert!(!cx.has_active_drag(), "no previous drag should remain");
            window.dispatch_event(
                FileDropEvent::Entered {
                    position,
                    paths: ExternalPaths(vec![second.clone()].into()),
                }
                .to_platform_input(),
                cx,
            );
            assert!(cx.has_active_drag());
        })
        .unwrap();
        cx.run_until_parked();
        cx.read_entity(studio, |s, _| {
            assert!(
                s.incoming_source.is_some(),
                "file enter must activate the drop target"
            )
        });
        cx.update_window(handle.into(), |_, window, cx| {
            let position = point(px(150.), px(180.));
            window.refresh();
            window.render_frame(cx);
            window.dispatch_event(FileDropEvent::Submit { position }.to_platform_input(), cx);
        })
        .unwrap();
        while cx.read_entity(studio, |s, _| s.document.source != "fn dropped() {}") {
            cx.run_until_parked();
            assert!(Instant::now() < deadline, "file drop not delivered");
            std::thread::sleep(Duration::from_millis(10));
        }
        settle(cx, handle, studio);
        cx.update_window(handle.into(), |_, window, cx| {
            studio.update(cx, |s, cx| {
                assert_eq!(s.document.language, "rust");
                s.open_paths(&[first.clone(), second.clone()], window, cx);
                assert!(s.notice.as_ref().unwrap().contains("one source file"));
                s.open_paths(&[invalid], window, cx);
            });
        })
        .unwrap();
        while cx.read_entity(studio, |s, _| {
            !s.notice.as_ref().is_some_and(|n| n.contains("UTF-8"))
        }) {
            cx.run_until_parked();
            assert!(
                Instant::now() < deadline,
                "invalid-file error not delivered"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        cx.read_entity(studio, |s, _| {
            assert_eq!(s.document.source, "fn dropped() {}")
        });
        // A cancelled drop must not leave an invisible target over the editor.
        cx.update_window(handle.into(), |_, window, cx| {
            let position = point(px(150.), px(180.));
            window.dispatch_event(
                FileDropEvent::Entered {
                    position,
                    paths: ExternalPaths(vec![first].into()),
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.dispatch_event(FileDropEvent::Exited.to_platform_input(), cx);
        })
        .unwrap();
        cx.read_entity(studio, |s, _| {
            assert!(s.incoming_source.is_none());
            assert_eq!(s.document.source, "fn dropped() {}");
        });
        settle(cx, handle, studio);
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: point(px(150.), px(180.)),
                    pressed_button: None,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
            assert!(
                window
                    .try_find("drag-image-button")
                    .is_none_or(|button| !button.visible()),
                "drag control stays hidden outside the preview"
            );
            window.hover("preview-image", cx);
            let image = window.find("preview-image").bounds();
            let button = window.find("drag-image-button");
            assert!(
                button.visible(),
                "hovering the preview reveals the drag control"
            );
            assert_eq!(button.bounds().origin, image.origin + point(px(8.), px(8.)));
        })
        .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let position = window.find("drag-image-button").bounds().center();
            window.dispatch_event(
                MouseMoveEvent {
                    position,
                    pressed_button: None,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        while cx.read_entity(studio, |s, _| s.drag_export.is_none()) {
            cx.run_until_parked();
            assert!(Instant::now() < deadline, "drag export not prepared");
            std::thread::sleep(Duration::from_millis(10));
        }
        settle(cx, handle, studio);
        let path = cx.read_entity(studio, |s, _| s.drag_export.as_ref().unwrap().path.clone());
        cx.update_window(handle.into(), |_, window, cx| {
            let origin = window.find("drag-image-button").bounds().center();
            window.dispatch_event(
                MouseDownEvent {
                    position: origin,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                !studio.read(cx).viewport.is_dragging(),
                "dragging the document control must not start canvas panning"
            );
            window.dispatch_event(
                MouseMoveEvent {
                    position: origin + point(px(-20.), px(0.)),
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(cx.has_active_drag(), "export affordance must start a drag");
            window.dispatch_event(
                MouseUpEvent {
                    position: origin,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            studio.update(cx, |s, cx| {
                s.install_source("main.rs".into(), "fn main() {}".into(), window, cx);
                assert!(
                    s.drag_export.is_none(),
                    "edits invalidate the cached export"
                );
            });
        })
        .unwrap();
        assert!(
            path.exists(),
            "recipient can still read the PNG after an edit"
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        settle(cx, handle, studio);
    }

    fn assert_fallback_glyphs(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
        window_width: u32,
    ) {
        let source = "// 中文 日本語 한국어\n// 👩🏽‍💻 👨‍👩‍👧‍👦 🇳🇱 ❤️\nlet greeting = \"你好 👋🏽 ☀️\";";
        let title = "示例・예제 🇯🇵.rs";
        cx.update_window(handle.into(), |_, window, cx| {
            studio.update(cx, |s, cx| {
                s.document.select_typeface(false, "fira-code");
                s.document.select_typeface(true, "space-mono");
                s.sync_typeface(false, window, cx);
                s.sync_typeface(true, window, cx);
                s.document.select_background(monoprint::Background::Theme);
                s.select_theme("removed-frame".parse().unwrap(), window, cx);
                s.font_size
                    .update(cx, |input, cx| input.replace_all("23", window, cx));
                s.install_source(title.into(), source.into(), window, cx);
            });
        })
        .unwrap();
        settle(cx, handle, studio);
        cx.read_entity(studio, |s, _| {
            assert!(s.error.is_none(), "fallback preview: {:?}", s.error);
            assert!(s.preview.is_some());
            assert_eq!(s.document.title, title);
        });
        let start = source.find("👩🏽‍💻").unwrap();
        let selection = start..start + "👩🏽‍💻".len();
        cx.update_window(handle.into(), |_, window, cx| {
            studio.read(cx).editor.clone().update(cx, |editor, cx| {
                editor.set_selected_range(selection.clone(), cx);
                editor.focus(window, cx);
            });
        })
        .unwrap();
        settle(cx, handle, studio);
        // A render-triggering control must preserve the original Unicode source
        // and byte-based selection, including the complete emoji sequence.
        inspector_click(cx, handle, studio, "increase-font-size");
        let document = cx.read_entity(studio, |s, cx| {
            assert!(s.error.is_none(), "fallback resize: {:?}", s.error);
            assert_eq!(s.document.font_size, "24");
            assert_eq!(s.document.source, source);
            assert_eq!(s.editor.read(cx).value().as_ref(), source);
            assert_eq!(s.editor.read(cx).selected_range(), selection);
            assert_eq!(s.editor.read(cx).selected_text().to_string(), "👩🏽‍💻");
            assert_eq!(s.document.font_id.as_deref(), Some("fira-code"));
            assert_eq!(s.document.comment_font_id.as_deref(), Some("space-mono"));
            s.document.clone()
        });
        let renderer = monoprint::Renderer::new();
        for format in [
            document::Format::Png,
            document::Format::Svg,
            document::Format::Html,
        ] {
            let output = document::export(&renderer, &document, format)
                .unwrap_or_else(|error| panic!("{} fallback export: {error:#}", format.label()));
            assert!(!output.is_empty());
        }
        cx.read_entity(studio, |s, cx| {
            assert_eq!(s.document.source, source);
            assert_eq!(s.editor.read(cx).selected_range(), selection);
        });
        capture(cx, handle, &format!("fallback-glyphs-{window_width}"));
    }

    fn assert_canvas_gestures(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let (bounds, image, revision, document, cached_image) = cx.read_entity(studio, |s, _| {
            let preview = s.preview.as_ref().unwrap();
            (
                s.viewport.bounds,
                size(
                    px(preview.width as f32 / 2.),
                    px(preview.height as f32 / 2.),
                ),
                s.revision,
                serde_json::to_string(&s.document).unwrap(),
                preview.image.clone(),
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
            assert!(Arc::ptr_eq(
                &s.preview.as_ref().unwrap().image,
                &cached_image
            ));
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
                    s.preview.as_ref().unwrap().height,
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
            assert_eq!(s.preview.as_ref().unwrap().width, 1000);
            assert!(
                s.preview.as_ref().unwrap().height > initial_height,
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
            assert_eq!(s.preview.as_ref().unwrap().width, 1160);
            assert!(
                (window.find("resize-image-right").bounds().center().x - fixed_right).abs()
                    < px(1.)
            );
        })
        .unwrap();

        // Inspect the first painted frame of every replacement, without a decode
        // settling delay. Keep moving so updates must arrive before mouse-up.
        let mut previous_image =
            cx.read_entity(studio, |s, _| s.preview.as_ref().unwrap().image.clone());
        let mut replacements = 0;
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
            let position = start - point(px(10. + (step % 15) as f32), px(0.));
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
            let (sample, expected, viewport_width) = cx
                .update_window(handle.into(), |_, window, cx| {
                    window.render_frame(cx);
                    let s = studio.read(cx);
                    assert!(s.canvas_resize.as_ref().unwrap().dragging);
                    let preview = s.preview.as_ref().unwrap();
                    if !Arc::ptr_eq(&preview.image, &previous_image) {
                        previous_image = preview.image.clone();
                        replacements += 1;
                    }
                    let image_size = size(
                        px(preview.width as f32 / 2.),
                        px(preview.height as f32 / 2.),
                    );
                    // Sample near the stale image's right edge: it must remain
                    // visible even when the requested outline is narrower.
                    let zoom = s.viewport.scale(image_size);
                    let origin = s.viewport.origin(image_size);
                    let resize = s.canvas_resize.as_ref().unwrap();
                    let outline_right =
                        resize.origin(px(resize.width as f32)).x + px(resize.width as f32) * zoom;
                    let mut x = preview.width - 24;
                    if (origin.x + px(x as f32 / 2.) * zoom - outline_right).abs() < px(3.) {
                        x -= 32; // Avoid sampling the moving blue guide itself.
                    }
                    let y = 24;
                    let offset = ((y * preview.width + x) * 4) as usize;
                    let bgra = &preview.image.as_bytes(0).unwrap()[offset..offset + 4];
                    let sample = s.viewport.bounds.origin
                        + origin
                        + point(px(x as f32 / 2.), px(y as f32 / 2.)) * zoom;
                    (
                        sample,
                        [bgra[2], bgra[1], bgra[0]],
                        window.viewport_size().width,
                    )
                })
                .unwrap();
            let pixels = cx.capture_screenshot(handle.into()).unwrap();
            let scale = pixels.width() as f32 / f32::from(viewport_width);
            let actual = pixels
                .get_pixel(
                    (f32::from(sample.x) * scale) as u32,
                    (f32::from(sample.y) * scale) as u32,
                )
                .0;
            for channel in 0..3 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= 5,
                    "preview must remain painted during resize: {actual:?} vs {expected:?}"
                );
            }
            if replacements >= 2 {
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
                s.preview.as_ref().unwrap().width,
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
        // colors to verify only the preview transmits the material below.
        let positions = [
            point(source_width + px(20.), px(70.)),
            point(viewport.width - px(5.), px(100.)),
            point(px(20.), px(70.)),
            copy_button,
            point(source_width + px(20.), px(20.)),
            point(px(20.), px(20.)),
            point(viewport.width - px(5.), px(20.)),
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
        assert!(
            samples[1][0][0].abs_diff(samples[0][0][0]) > 40,
            "preview must expose the material at {:?}",
            positions[0]
        );
        assert_eq!(samples[0][1], samples[1][1], "inspector stays opaque");
        assert_eq!(samples[0][2], samples[1][2], "editor stays opaque");
        assert_eq!(samples[0][3], samples[1][3], "controls stay opaque");
        for index in 4..7 {
            assert_eq!(
                samples[0][index], samples[1][index],
                "top bar stays opaque at {:?}",
                positions[index]
            );
        }
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
            if !studio.read(cx).browsing_themes {
                let scroll = &studio.read(cx).settings_scroll;
                scroll.set_offset(
                    scroll.offset() + point(px(0.), scroll.bounds().center().y - target.center().y),
                );
            }
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
        if !cx.read_entity(studio, |s, _| s.browsing_themes) {
            inspector_click(cx, handle, studio, "browse-themes");
        }
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
        pointer_click_count(window, position, 1, false, cx);
    }

    fn pointer_click_count(
        window: &mut Window,
        position: Point<Pixels>,
        click_count: usize,
        move_while_pressed: bool,
        cx: &mut App,
    ) {
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
                click_count,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        if move_while_pressed {
            // TestWindow intentionally panics on start_window_move. This makes
            // accidental titlebar dragging from a control a regression failure.
            window.dispatch_event(
                MouseMoveEvent {
                    position: position + point(px(1.), px(0.)),
                    pressed_button: Some(MouseButton::Left),
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
        }
        window.dispatch_event(
            MouseUpEvent {
                position,
                button: MouseButton::Left,
                click_count,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    }

    fn header_control_click(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
        id: &'static str,
        click_count: usize,
    ) {
        cx.update_window(handle.into(), |_, window, cx| {
            let position = window.find(id).bounds().center();
            pointer_click_count(window, position, click_count, true, cx);
        })
        .unwrap();
        settle(cx, handle, studio);
    }

    fn assert_interaction_motion(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        let frame = |cx: &mut HeadlessAppContext| {
            cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
                .unwrap();
        };
        let bounds = |cx: &mut HeadlessAppContext, id: &'static str| {
            cx.update_window(handle.into(), |_, window, _| window.find(id).bounds())
                .unwrap()
        };
        let sample = |cx: &mut HeadlessAppContext, position: Point<Pixels>| {
            let image = cx.capture_screenshot(handle.into()).unwrap();
            let size = cx
                .update_window(handle.into(), |_, window, _| window.viewport_size())
                .unwrap();
            let scale = image.width() as f32 / f32::from(size.width);
            image
                .get_pixel(
                    (f32::from(position.x) * scale) as u32,
                    (f32::from(position.y) * scale) as u32,
                )
                .0
        };
        let open = bounds(cx, "open-source");
        let corner = open.origin + point(px(5.), px(5.));
        let normal = sample(cx, corner);
        cx.update_window(handle.into(), |_, window, cx| {
            window.hover("open-source", cx)
        })
        .unwrap();
        frame(cx);
        let start = sample(cx, corner);
        cx.advance_clock(Duration::from_millis(40));
        frame(cx);
        let middle = sample(cx, corner);
        cx.advance_clock(Duration::from_millis(120));
        frame(cx);
        let end = sample(cx, corner);
        assert_eq!(start, normal, "hover begins at the current paint color");
        assert_ne!(middle, start, "hover changes gradually");
        assert_ne!(middle, end, "hover has an intermediate paint color");

        let png = bounds(cx, "png");
        let svg = bounds(cx, "svg");
        let html = bounds(cx, "html");
        let center = |cx: &mut HeadlessAppContext| {
            let image = cx.capture_screenshot(handle.into()).unwrap();
            let (size, p) = cx
                .update_window(handle.into(), |_, window, cx| {
                    (window.viewport_size(), appearance::palette(cx))
                })
                .unwrap();
            let scale = image.width() as f32 / f32::from(size.width);
            let color = [
                (p.control_on >> 16) as u8,
                (p.control_on >> 8) as u8,
                p.control_on as u8,
            ];
            let y = (f32::from(png.top() + px(3.)) * scale) as u32;
            let matches = ((f32::from(png.left()) * scale) as u32
                ..(f32::from(html.right()) * scale) as u32)
                .filter(|x| image.get_pixel(*x, y).0[..3] == color)
                .collect::<Vec<_>>();
            assert!(
                !matches.is_empty(),
                "one selection backdrop remains visible"
            );
            (*matches.first().unwrap() + *matches.last().unwrap()) as f32 / (2. * scale)
        };
        let from = center(cx);
        cx.update_window(handle.into(), |_, window, cx| window.click("svg", cx))
            .unwrap();
        frame(cx);
        assert!(
            (center(cx) - from).abs() < 2.,
            "the backdrop starts at the previous tab"
        );
        cx.advance_clock(Duration::from_millis(40));
        frame(cx);
        let midway = center(cx);
        let to = f32::from(svg.center().x);
        assert!(
            midway > from + 2. && midway < to - 2.,
            "selection slides between tabs: {from} → {midway} → {to}"
        );
        cx.advance_clock(Duration::from_millis(180));
        frame(cx);
        assert!(
            (center(cx) - to).abs() < 2.,
            "selection lands on the new tab"
        );

        cx.update(|cx| cx.set_reduce_motion(true));
        cx.update_window(handle.into(), |_, window, cx| window.click("png", cx))
            .unwrap();
        frame(cx);
        assert!(
            (center(cx) - f32::from(png.center().x)).abs() < 2.,
            "reduced motion switches selection immediately"
        );
        cx.update(|cx| cx.set_reduce_motion(false));
        settle(cx, handle, studio);
        capture(cx, handle, "interaction-motion-settled");
    }

    fn assert_dropdown_motion(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        // The popup uses GPUI's wall-clock animation. Observe an early native
        // frame and compare the visible shadow ring with the panel's opacity.
        let mode = cx
            .update_window(handle.into(), |_, _, cx| {
                if appearance::palette(cx).light {
                    "light"
                } else {
                    "dark"
                }
            })
            .unwrap();
        let closed = cx.capture_screenshot(handle.into()).unwrap();
        let mut attempts = 0;
        let popup = loop {
            cx.update_window(handle.into(), |_, window, cx| {
                window.click("source-language", cx);
                window.render_frame(cx);
            })
            .unwrap();
            std::thread::sleep(Duration::from_millis(15));
            let popup = cx
                .update_window(handle.into(), |_, window, cx| {
                    window.render_frame(cx);
                    let color: Hsla = rgb(appearance::palette(cx).raised).into();
                    let width = 240. * window.scale_factor();
                    window
                        .painted_quads()
                        .into_iter()
                        .find(|quad| {
                            let Some(background) = quad.background.as_solid() else {
                                return false;
                            };
                            (quad.bounds.size.width.0 - width).abs() < 1.
                                && quad.bounds.size.height.0 > 80.
                                && background.h == color.h
                                && background.s == color.s
                                && background.l == color.l
                        })
                        .expect("language popup surface is painted")
                })
                .unwrap();
            let opacity = popup.background.as_solid().unwrap().a;
            if opacity > 0.15 && opacity < 0.85 {
                break popup;
            }
            // A busy machine may miss the short mid-fade interval. Close and
            // reopen to observe it, rather than testing an already settled menu.
            attempts += 1;
            assert!(
                attempts < 8,
                "could not observe a mid-fade popup frame: {opacity}"
            );
            cx.update_window(handle.into(), |_, window, cx| {
                window.press("escape", cx);
                window.render_frame(cx);
            })
            .unwrap();
        };
        let opacity = popup.background.as_solid().unwrap().a;
        let early = cx.capture_screenshot(handle.into()).unwrap();
        capture(cx, handle, &format!("language-popup-fading-{mode}"));
        // The 1px ring is the most visible shadow layer in either appearance.
        // Sample its straight left edge, outside the translucent panel.
        let x = (popup.bounds.left().0 - 1.).floor() as u32;
        let y = popup.bounds.center().y.0.floor() as u32;
        let inside_x = (popup.bounds.right().0 - 8.).floor() as u32;
        let underlying = closed.get_pixel(inside_x, y).0[0] as f32;
        let surface = cx
            .update_window(handle.into(), |_, _, cx| {
                (appearance::palette(cx).raised >> 16) as f32
            })
            .unwrap();
        let expected = underlying + (surface - underlying) * opacity;
        let actual = early.get_pixel(inside_x, y).0[0] as f32;
        assert!(
            (actual - expected).abs() < 4.,
            "shadow ink stays outside the fading panel: actual={actual}, expected={expected}"
        );
        let baseline = closed.get_pixel(x, y).0[0] as f32;
        let rising = early.get_pixel(x, y).0[0] as f32 - baseline;
        std::thread::sleep(Duration::from_millis(170));
        cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
            .unwrap();
        let settled = cx.capture_screenshot(handle.into()).unwrap();
        let resting = settled.get_pixel(x, y).0[0] as f32 - baseline;
        assert!(
            resting.abs() > 5.,
            "the popup's shadow ring remains visible: {resting}"
        );
        let ratio = rising / resting;
        assert!(
            ratio > opacity * 0.6 && ratio < opacity * 1.5,
            "shadow fades with the panel instead of arriving later: shadow={ratio}, panel={opacity}"
        );
        cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
            .unwrap();
        settle(cx, handle, studio);

        cx.update(|cx| cx.set_reduce_motion(true));
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("source-language", cx);
            window.render_frame(cx);
            let popup = window
                .painted_quads()
                .into_iter()
                .find(|quad| {
                    (quad.bounds.size.width.0 - 240. * window.scale_factor()).abs() < 1.
                        && quad.bounds.size.height.0 > 80.
                        && quad.background.as_solid().is_some()
                })
                .expect("reduced-motion popup is painted on its first frame");
            assert_eq!(popup.background.as_solid().unwrap().a, 1.);
        })
        .unwrap();
        capture(cx, handle, &format!("language-popup-reduced-motion-{mode}"));
        cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
            .unwrap();
        cx.update(|cx| cx.set_reduce_motion(false));
        settle(cx, handle, studio);
    }

    fn assert_platform_keys(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
    ) {
        // Dispatch keys through the rendered control's focus context, rather
        // than calling editing actions directly. Include a multibyte character.
        macro_rules! check {
            ($window:ident, $app:ident, $state:expr, $cases:expr) => {{
                let state = $state;
                let original = state.read($app).value().to_string();
                let selection = state.read($app).selected_range();
                for (keys, source, cursor, expected_range, expected_text) in $cases {
                    state.update($app, |state, $app| {
                        state.replace_all(source, $window, $app);
                        state.set_selected_range(cursor..cursor, $app);
                        state.focus($window, $app);
                    });
                    $window.render_frame($app);
                    for key in keys.split_whitespace() {
                        $window.dispatch_event(
                            KeyDownEvent {
                                keystroke: Keystroke::parse(key).unwrap(),
                                is_held: false,
                                prefer_character_input: false,
                            }
                            .to_platform_input(),
                            $app,
                        );
                    }
                    assert_eq!(state.read($app).selected_range(), expected_range, "{keys}");
                    assert_eq!(state.read($app).value().as_ref(), expected_text, "{keys}");
                }
                state.update($app, |state, $app| {
                    state.replace_all(original, $window, $app);
                    state.set_selected_range(selection, $app);
                });
            }};
        }
        cx.update_window(handle.into(), |_, window, cx| {
            let horizontal = || {
                let text = "ab界cd";
                let after_character = text.find('c').unwrap();
                [
                    ("ctrl-b", text, after_character, 2..2, text),
                    ("ctrl-f", text, 2, after_character..after_character, text),
                    (
                        "ctrl-shift-b",
                        text,
                        after_character,
                        2..after_character,
                        text,
                    ),
                    ("ctrl-shift-f", text, 2, 2..after_character, text),
                    ("ctrl-h", text, after_character, 2..2, "abcd"),
                    ("ctrl-d", text, 2, 2..2, "abcd"),
                    (
                        "ctrl-h cmd-z",
                        text,
                        after_character,
                        after_character..after_character,
                        text,
                    ),
                    ("ctrl-d cmd-z", text, 2, 2..2, text),
                    ("ctrl-a", text, after_character, 0..0, text),
                    ("ctrl-e", text, 2, text.len()..text.len(), text),
                ]
            };
            check!(window, cx, studio.read(cx).editor.clone(), horizontal());
            check!(window, cx, studio.read(cx).title.clone(), horizontal());
            let text = "abcd\nefgh\nijkl";
            check!(
                window,
                cx,
                studio.read(cx).editor.clone(),
                [
                    ("ctrl-p", text, 7, 2..2, text),
                    ("ctrl-n", text, 7, 12..12, text),
                    ("ctrl-shift-p", text, 7, 2..7, text),
                    ("ctrl-shift-n", text, 7, 7..12, text),
                    ("cmd-left", text, 7, 5..5, text),
                    ("cmd-right", text, 7, 9..9, text),
                    ("cmd-up", text, 7, 0..0, text),
                    ("cmd-down", text, 7, 14..14, text),
                    ("alt-left", text, 7, 5..5, text),
                    ("alt-right", text, 7, 9..9, text),
                ]
            );
            let focus = studio.read(cx).focus.clone();
            window.focus(&focus, cx);
        })
        .unwrap();
        settle(cx, handle, studio);
        println!(
            "macOS Control movement, selection, UTF-8 deletion/undo and Command/Option navigation passed"
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
                Arc::new(crate::assets::Assets),
                gpui_kit::platform::current_headless_renderer,
            );
            cx.update(|cx| {
                gpui_kit::init(cx);
                platform_keys::init(cx);
                appearance::init(cx);
                cx.bind_keys([KeyBinding::new(
                    "escape",
                    gpui_kit::component::input::Escape,
                    Some("ThemeBrowser"),
                )]);
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
            assert_source_syntax_colored(&mut cx, handle, &studio);

            assert_platform_keys(&mut cx, handle, &studio);

            for _ in 0..2 {
                assert_syntax_survives_typing(&mut cx, handle, &studio);
                header_control_click(&mut cx, handle, &studio, "ui-appearance", 1);
            }

            assert_interaction_motion(&mut cx, handle, &studio);
            assert_dropdown_motion(&mut cx, handle, &studio);
            header_control_click(&mut cx, handle, &studio, "ui-appearance", 1);
            assert_dropdown_motion(&mut cx, handle, &studio);
            header_control_click(&mut cx, handle, &studio, "ui-appearance", 1);
            let revision = cx.read_entity(&studio, |s, _| {
                assert_eq!(s.open_sections, [true, false, true, true]);
                s.revision
            });
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("code-typeface").is_none());
                assert!(window.try_find("clear-highlights").is_none());
                assert!(window.try_find("png-compression-balanced").is_none());
                assert!(window.find("browse-themes").visible());
                assert!(window.try_find("close-theme-browser").is_none());
                assert!(window.try_find("themes-all").is_none());
                assert!(
                    window.find("section-theme").bounds().top()
                        < window.find("section-typeface").bounds().top()
                );
                assert!(
                    window.find("section-typeface").bounds().top()
                        < window.find("section-frame").bounds().top()
                );
            })
            .unwrap();
            for (section, control) in [
                ("section-theme", "browse-themes"),
                ("section-frame", "line-numbers"),
                ("section-export", "png"),
            ] {
                inspector_click(&mut cx, handle, &studio, section);
                cx.update_window(handle.into(), |_, window, _| {
                    assert!(window.try_find(control).is_none());
                    assert!(window.find("export-image").visible());
                    assert!(window.find("copy-image").visible());
                })
                .unwrap();
                inspector_click(&mut cx, handle, &studio, section);
            }
            inspector_click(&mut cx, handle, &studio, "section-typeface");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(
                    s.revision, revision,
                    "section toggles must not request renders"
                );
            });

            // Partner themes inherit their gutter by default, but can now be
            // overridden just like any other theme. Reset restores inheritance.
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.select_theme("removed-frame".parse().unwrap(), window, cx);
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.line_numbers, None);
                assert!(s.document.numbers_visible());
            });
            capture(
                &mut cx,
                handle,
                &format!("numbers-default-{}", width as u32),
            );
            inspector_click(&mut cx, handle, &studio, "line-numbers");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.line_numbers, Some(false));
                assert!(!s.document.numbers_visible());
                assert!(s.error.is_none());
            });
            capture(&mut cx, handle, &format!("numbers-hidden-{}", width as u32));
            inspector_click(&mut cx, handle, &studio, "reset-line-numbers");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.line_numbers, None);
                assert!(s.document.numbers_visible());
            });
            cx.update_window(handle.into(), |_, window, cx| {
                assert!(window.try_find("reset-line-numbers").is_none());
                studio.update(cx, |s, cx| {
                    s.select_theme("removed-frame".parse().unwrap(), window, cx);
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            inspector_click(&mut cx, handle, &studio, "line-numbers");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.line_numbers, Some(true));
                assert!(s.document.numbers_visible());
            });
            inspector_click(&mut cx, handle, &studio, "reset-line-numbers");
            cx.read_entity(&studio, |s, _| {
                assert!(!s.document.numbers_visible());
                assert_eq!(s.document.png_compression, "balanced");
            });
            click(&mut cx, handle, &studio, "advanced-export");
            for (mode, control) in [
                ("fast", "png-compression-fast"),
                ("balanced", "png-compression-balanced"),
            ] {
                click(&mut cx, handle, &studio, control);
                cx.read_entity(&studio, |s, _| {
                    assert_eq!(s.document.png_compression, mode);
                    assert!(s.error.is_none(), "{:?}", s.error);
                });
            }
            capture(&mut cx, handle, &format!("compression-{}", width as u32));
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("png-compression-balanced").is_none());
            })
            .unwrap();

            // The select's visible trigger and hit area must both fit the toolbar.
            // Select styles alone do not constrain its full-size outer wrapper.
            cx.update_window(handle.into(), |_, window, _| {
                let open = window.find("open-source").bounds();
                let paste = window.find("paste-source").bounds();
                let language = window.find("source-language").bounds();
                let wrap = window.find("wrap-source").bounds();
                assert_eq!(open.size, size(px(26.), px(26.)));
                assert_eq!(paste.size, open.size);
                assert_eq!(language.size.height, px(26.));
                assert!(language.size.width <= px(160.));
                assert!(language.size.width >= px(40.));
                assert_eq!(language.center().y, open.center().y);
                assert_eq!(language.center().y, paste.center().y);
                assert_eq!(language.center().y, wrap.center().y);
                assert!(open.right() < paste.left());
                assert!(paste.right() < language.left());
                assert!(language.right() < wrap.left());
                assert!(
                    open.left() >= px(92.),
                    "leave a clear gap after the native window controls"
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
                        s.preview.as_ref().unwrap().image.clone(),
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
                    let open = window.find("open-source").bounds();
                    let paste = window.find("paste-source").bounds();
                    let language = window.find("source-language").bounds();
                    assert!(open.left() >= px(92.));
                    assert!(open.right() < paste.left());
                    assert!(paste.right() < language.left());
                    assert!(language.size.width >= px(40.));
                    if expected >= 320. {
                        assert!(
                            language.size.width >= px(120.),
                            "{label}: leave enough text space for JavaScript and TypeScript"
                        );
                    }
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
                    assert!(Arc::ptr_eq(
                        &s.preview.as_ref().unwrap().image,
                        &original_image
                    ));
                })
                .unwrap();
                if matches!(label, "wide" | "max" | "min") {
                    capture(&mut cx, handle, &format!("source-{label}-{}", width as u32));
                }
            }
            header_control_click(&mut cx, handle, &studio, "wrap-source", 1);
            cx.read_entity(&studio, |s, _| assert!(!s.wrap));
            header_control_click(&mut cx, handle, &studio, "wrap-source", 2);
            cx.read_entity(&studio, |s, _| assert!(s.wrap));

            header_control_click(&mut cx, handle, &studio, "source-language", 1);
            std::thread::sleep(Duration::from_millis(180));
            settle(&mut cx, handle, &studio);
            capture(&mut cx, handle, &format!("language-menu-{}", width as u32));
            cx.update_window(handle.into(), |_, window, cx| {
                assert!(window.try_find("more-languages").is_none());
                window.input("typescript", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            capture(
                &mut cx,
                handle,
                &format!("language-typescript-{}", width as u32),
            );
            cx.update_window(handle.into(), |_, window, cx| {
                window.press("enter", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.language, "typescript")
            });
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            header_control_click(&mut cx, handle, &studio, "source-language", 1);
            cx.update_window(handle.into(), |_, window, cx| {
                window.input("angular-ts", cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| window.press("enter", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.language, "typescript")
            });
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            header_control_click(&mut cx, handle, &studio, "source-language", 1);
            cx.update_window(handle.into(), |_, window, cx| window.input("tsx", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| window.press("enter", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| assert_eq!(s.document.language, "tsx"));
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);

            let original = cx.read_entity(&studio, |s, _| {
                (s.revision, serde_json::to_string(&s.document).unwrap())
            });
            header_control_click(&mut cx, handle, &studio, "half-preview", 2);
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
            header_control_click(&mut cx, handle, &studio, "ui-appearance", 1);
            cx.update(|cx| assert!(appearance::palette(cx).light));
            assert_material_visible(&mut cx, handle, &studio);
            assert_source_syntax_colored(&mut cx, handle, &studio);
            capture(
                &mut cx,
                handle,
                &format!("light-interface-{}", width as u32),
            );
            header_control_click(&mut cx, handle, &studio, "ui-appearance", 2);
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
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("style-inspector").is_none());
                assert!(window.try_find("section-typeface").is_none());
                assert!(window.find("close-theme-browser").visible());
                assert!(window.find("export-image").visible());
                assert!(window.find("copy-image").visible());
            })
            .unwrap();
            cx.read_entity(&studio, |s, _| {
                assert!(s.theme_scroll.0.borrow().base_handle.bounds().size.height > px(212.));
            });
            capture(&mut cx, handle, &format!("theme-browser-{}", width as u32));
            let revision = cx.read_entity(&studio, |s, _| s.revision);
            click(&mut cx, handle, &studio, "ui-appearance");
            capture(
                &mut cx,
                handle,
                &format!("theme-browser-light-{}", width as u32),
            );
            click(&mut cx, handle, &studio, "ui-appearance");
            click(&mut cx, handle, &studio, "advanced-export");
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| {
                assert!(
                    studio.read(cx).browsing_themes,
                    "Escape dismisses the popover first"
                );
                assert!(window.try_find("png-compression-balanced").is_none());
            })
            .unwrap();
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
                assert_eq!(s.thumbnails[s.filtered_themes[0]].0.id(), "ayu");
            });
            inspector_click(&mut cx, handle, &studio, "themes-dark");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.filtered_themes.len(), 2);
                assert_eq!(s.revision, revision);
            });
            cx.update_window(handle.into(), |_, window, cx| window.press("escape", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| {
                assert!(!studio.read(cx).browsing_themes);
                assert!(window.try_find("close-theme-browser").is_none());
                assert!(window.find("browse-themes").visible());
            })
            .unwrap();
            reveal_theme_gallery(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, cx| {
                assert_eq!(s.theme_search.read(cx).value().as_str(), "Ayu");
                assert_eq!(s.theme_filter, studio::ThemeFilter::Dark);
                assert_eq!(s.filtered_themes.len(), 2);
                assert_eq!(s.revision, revision, "opening/closing must not render");
            });
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    let single = monoprint::Theme::all()
                        .find(|theme| {
                            !(theme.supports(monoprint::Appearance::Light)
                                && theme.supports(monoprint::Appearance::Dark))
                        })
                        .unwrap();
                    s.select_theme(single, window, cx);
                });
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("light-appearance").is_none());
                assert!(window.try_find("dark-appearance").is_none());
            })
            .unwrap();
            capture(
                &mut cx,
                handle,
                &format!("single-appearance-{}", width as u32),
            );
            let ayu = cx.read_entity(&studio, |s, _| {
                s.thumbnails
                    .iter()
                    .position(|(theme, _)| theme.id() == "ayu")
                    .unwrap()
            });
            reveal_theme_gallery(&mut cx, handle, &studio);
            cx.update_window(handle.into(), |_, window, cx| {
                window.click(("theme-swatch", ayu), cx)
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            for (control, light) in [("light-appearance", true), ("dark-appearance", false)] {
                inspector_click(&mut cx, handle, &studio, control);
                settle(&mut cx, handle, &studio);
                cx.read_entity(&studio, |s, _| {
                    assert_eq!(s.document.theme, "ayu");
                    assert_eq!(s.document.light, light);
                    assert!(s.error.is_none(), "paired appearance must render");
                    assert_eq!(s.preview_revision, s.revision);
                });
                capture(&mut cx, handle, &format!("ayu-{control}-{}", width as u32));
            }
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
            let settings_offset = cx.read_entity(&studio, |s, _| s.settings_scroll.offset());
            let pinned_bounds = cx
                .update_window(handle.into(), |_, window, _| {
                    (
                        window.find("close-theme-browser").bounds(),
                        window.find("export-image").bounds(),
                    )
                })
                .unwrap();
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
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.settings_scroll.offset(), settings_offset)
            });
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.find(("theme-swatch", last)).visible());
                assert!(window.try_find(("theme-swatch", candy)).is_none());
                assert_eq!(window.find("close-theme-browser").bounds(), pinned_bounds.0);
                assert_eq!(window.find("export-image").bounds(), pinned_bounds.1);
            })
            .unwrap();
            let gallery_offset = cx.read_entity(&studio, |s, _| {
                s.theme_scroll.0.borrow().base_handle.offset()
            });
            click(&mut cx, handle, &studio, "close-theme-browser");
            assert_eq!(
                cx.read_entity(&studio, |s, _| s.settings_scroll.offset()),
                settings_offset
            );
            reveal_theme_gallery(&mut cx, handle, &studio);
            assert_eq!(
                cx.read_entity(&studio, |s, _| s
                    .theme_scroll
                    .0
                    .borrow()
                    .base_handle
                    .offset()),
                gallery_offset
            );
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
            reveal_theme_gallery(&mut cx, handle, &studio);
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
            let (export_bounds, copy_bounds) = cx
                .update_window(handle.into(), |_, window, cx| {
                    window.render_frame(cx);
                    (
                        window.find("export-image").bounds(),
                        window.find("copy-image").bounds(),
                    )
                })
                .unwrap();
            let export_pixels = |pixels: &image::RgbaImage| {
                let scale = pixels.width() as f32 / width;
                image::imageops::crop_imm(
                    pixels,
                    (f32::from(export_bounds.origin.x) * scale) as u32,
                    (f32::from(export_bounds.origin.y) * scale) as u32,
                    (f32::from(export_bounds.size.width) * scale) as u32,
                    (f32::from(export_bounds.size.height) * scale) as u32,
                )
                .to_image()
            };
            let idle_export = export_pixels(&cx.capture_screenshot(handle.into()).unwrap());
            for attempt in 0..2 {
                cx.update_window(handle.into(), |_, window, cx| {
                    window.click("copy-image", cx);
                    studio.read_with(cx, |s, _| {
                        assert_eq!(s.copy_state, studio::CopyState::Copying);
                        assert!(!s.exporting);
                    });
                    window.render_frame(cx);
                    assert_eq!(window.find("export-image").bounds(), export_bounds);
                    assert_eq!(window.find("copy-image").bounds(), copy_bounds);
                })
                .unwrap();
                assert_eq!(
                    export_pixels(&cx.capture_screenshot(handle.into()).unwrap()),
                    idle_export,
                    "Copying must not change Export's rendered pixels"
                );
                cx.run_until_parked();
                cx.read_entity(&studio, |s, cx| {
                    assert_eq!(s.copy_state, studio::CopyState::Copied);
                    assert!(s.notice.is_none());
                    assert!(matches!(
                        cx.read_from_clipboard().unwrap().entries(),
                        [ClipboardEntry::Image(_)]
                    ));
                });
                cx.update_window(handle.into(), |_, window, cx| {
                    window.render_frame(cx);
                    assert_eq!(window.find("export-image").bounds(), export_bounds);
                    assert_eq!(window.find("copy-image").bounds(), copy_bounds);
                })
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
                window.render_frame(cx);
                assert_eq!(window.find("export-image").bounds(), export_bounds);
                assert_eq!(window.find("copy-image").bounds(), copy_bounds);
            })
            .unwrap();
            click(&mut cx, handle, &studio, "close-theme-browser");

            cx.update_window(handle.into(), |_, window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string("const pasted = 7;".into()));
                window.click("paste-source", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            inspector_click(&mut cx, handle, &studio, "padding-presets");
            capture(
                &mut cx,
                handle,
                &format!("padding-presets-{}", width as u32),
            );
            cx.update_window(handle.into(), |_, window, cx| {
                for _ in 0..3 {
                    window.press("down", cx);
                }
                window.press("enter", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.source, "const pasted = 7;");
                assert_eq!(s.document.padding, "32");
            });
            cx.update_window(handle.into(), |_, window, cx| {
                studio.update(cx, |s, cx| {
                    s.install_source("greet.rb".into(), "# Syntax colors, using Giallo\ndef greet(name)\n  puts \"Hello, #{name}!\"\nend\n".into(), window, cx)
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
            assert_handoffs(&mut cx, handle, &studio);

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
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("light-appearance").is_none());
                assert!(window.try_find("dark-appearance").is_none());
            })
            .unwrap();
            // Restore the inspector's top before testing export validation.
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
            let (original_height, editor_line_height) = cx.read_entity(&studio, |s, cx| {
                (
                    s.preview.as_ref().unwrap().height,
                    s.editor.read(cx).line_height(),
                )
            });
            inspector_click(&mut cx, handle, &studio, "font-size");
            cx.update_window(handle.into(), |_, window, cx| {
                window.press("cmd-a", cx);
                window.input("24", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            let resized = cx.read_entity(&studio, |s, cx| {
                assert_eq!(s.document.font_size, "24");
                assert!(s.error.is_none(), "{:?}", s.error);
                assert!(s.preview.as_ref().unwrap().height > original_height);
                assert_eq!(s.editor.read(cx).line_height(), editor_line_height);
                s.preview.as_ref().unwrap().image.clone()
            });
            capture(&mut cx, handle, &format!("font-size-24-{}", width as u32));
            // Incomplete edits keep the last rendered image until a valid size arrives.
            cx.update_window(handle.into(), |_, window, cx| {
                window.press("cmd-a", cx);
                window.press("backspace", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(s.document.font_size.is_empty());
                assert!(s.error.as_ref().unwrap().contains("Font size"));
                assert!(Arc::ptr_eq(&resized, &s.preview.as_ref().unwrap().image));
            });
            cx.update_window(handle.into(), |_, window, cx| window.input("18", cx))
                .unwrap();
            settle(&mut cx, handle, &studio);
            inspector_click(&mut cx, handle, &studio, "increase-font-size");
            cx.read_entity(&studio, |s, _| assert_eq!(s.document.font_size, "19"));
            inspector_click(&mut cx, handle, &studio, "decrease-font-size");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.font_size, "18");
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            inspector_click(&mut cx, handle, &studio, "font-size");
            cx.update_window(handle.into(), |_, window, cx| {
                window.press("cmd-a", cx);
                window.input("15", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.font_size, "15");
                assert_eq!(s.preview.as_ref().unwrap().height, original_height);
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
            cx.update_window(handle.into(), |_, window, _| {
                let clear = window.find("clear-highlights").bounds();
                assert_eq!(
                    clear.center().y,
                    window.find("open-source").bounds().center().y
                );
                assert!(window.find("open-source").bounds().left() >= px(92.));
                // With Clear highlights present at the minimum sidebar width,
                // retain a usable dropdown hit target and the traffic-light gap.
                assert!(window.find("source-language").bounds().size.width >= px(26.));
            })
            .unwrap();
            click(&mut cx, handle, &studio, "clear-highlights");
            cx.update_window(handle.into(), |_, window, _| {
                assert!(window.try_find("clear-highlights").is_none());
            })
            .unwrap();
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
            assert_fallback_glyphs(&mut cx, handle, &studio, width as u32);
            println!(
                "Studio {}×{}: render, sidebar resizing, virtual theme scrolling/filtering, transparency, format, source highlighting, copy feedback, typography, automatic emoji/CJK fallback, interface appearance, canvas pan/pinch/drag/live resize, source selections, validation and recovery passed",
                width, height
            );
        }
    }
}
