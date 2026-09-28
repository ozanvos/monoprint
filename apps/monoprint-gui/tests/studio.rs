//! Real GPUI controls and Metal pixels, without controlling the user's desktop.
#[path = "../src/appearance.rs"]
mod appearance;
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
#[path = "../src/source_highlight.rs"]
mod source_highlight;
#[path = "../src/studio.rs"]
mod studio;
#[path = "../src/view.rs"]
mod view;

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

    fn inspector_click(
        cx: &mut HeadlessAppContext,
        handle: WindowHandle<Root>,
        studio: &Entity<studio::Studio>,
        id: &'static str,
    ) {
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let target = window.find(id).bounds();
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
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(s.preview.is_some(), "{:?}", s.error)
            });
            capture(&mut cx, handle, &format!("studio-{}", width as u32));

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
                    ScrollDelta::Pixels(point(px(0.), px(-2000.))),
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
                        .scroll_to_item(candy / 2, ScrollStrategy::Center);
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
                window.click(("padding", 32u32), cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
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

            cx.update_window(handle.into(), |_, window, cx| {
                assert!(window.find("export-image").visible());
                assert!(window.find("transparent-background").visible());
                window.click("light-appearance", cx);
                window.click("transparent-background", cx);
            })
            .unwrap();
            settle(&mut cx, handle, &studio);
            cx.read_entity(&studio, |s, _| {
                assert!(s.document.light);
                assert!(s.document.transparent);
                assert!(s.error.is_none());
            });
            capture(&mut cx, handle, &format!("transparent-{}", width as u32));
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
            cx.update_window(handle.into(), |_, window, cx| {
                window.click("light-appearance", cx);
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
            inspector_click(&mut cx, handle, &studio, "advanced-settings");
            assert!(cx.read_entity(&studio, |s, _| s.advanced));
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
            cx.update_window(handle.into(), |_, _, cx| {
                studio.update(cx, |s, cx| {
                    let start = s.document.source.find("café").unwrap();
                    s.editor.update(cx, |editor, cx| {
                        editor.set_selected_range(start..start + "café".len(), cx)
                    });
                });
            })
            .unwrap();
            inspector_click(&mut cx, handle, &studio, "highlight-selection");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(s.document.text_highlights, "2:5-2:9")
            });
            cx.update_window(handle.into(), |_, _, cx| {
                studio.update(cx, |s, cx| {
                    let end = s.document.source.find("let ok").unwrap();
                    s.editor
                        .update(cx, |editor, cx| editor.set_selected_range(0..end, cx));
                });
            })
            .unwrap();
            inspector_click(&mut cx, handle, &studio, "highlight-selected-lines");
            cx.read_entity(&studio, |s, _| {
                assert_eq!(
                    s.document.highlights, "1-2",
                    "a selection ending at column one excludes that line"
                );
                assert!(s.error.is_none(), "{:?}", s.error);
            });
            capture(
                &mut cx,
                handle,
                &format!("advanced-highlights-{}", width as u32),
            );
            inspector_click(&mut cx, handle, &studio, "clear-highlights");
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
            inspector_click(&mut cx, handle, &studio, "highlight-selection");
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
            println!(
                "Studio {}×{}: render, virtual theme scrolling/filtering, transparency, format, source highlighting, copy feedback, advanced typography, source selections, validation and recovery passed",
                width, height
            );
        }
    }
}
