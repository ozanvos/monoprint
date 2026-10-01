use crate::{
    appearance::*, canvas_resize::ResizeEdge, document::Format, studio::*, viewport::PreviewZoom,
};
use gpui_kit::assets::IconName;
use gpui_kit::{
    component::{
        button::*,
        collapsible::Collapsible,
        input::{Editor, Input},
        menu::{DropdownMenu, PopupMenuItem},
        popover::Popover,
        scroll::Scrollbar,
        select::Select,
        switch::Switch,
        tooltip::Tooltip,
        *,
    },
    prelude::*,
    *,
};
use monoprint::{Appearance, Background, Language, Theme as ImageTheme};

struct DragThumbnail(Option<std::sync::Arc<RenderImage>>);
impl Render for DragThumbnail {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(160.))
            .h(px(100.))
            .when_some(self.0.clone(), |view, image| {
                view.child(img(image).size_full().object_fit(ObjectFit::Contain))
            })
    }
}

fn header_bar(id: &'static str, window: &mut Window, cx: &mut App) -> Stateful<Div> {
    let header = div().id(id);
    if !cfg!(target_os = "macos") {
        return header;
    }

    // Match GPUI Kit's TitleBar gesture ownership, but exclude interactive
    // children: their mouse-down prevents default before it reaches the header.
    let pressed = window.use_keyed_state(id, cx, |_, _| false);
    header
        .on_mouse_down_out(window.listener_for(&pressed, |pressed, _, _, _| *pressed = false))
        .on_mouse_down(
            MouseButton::Left,
            window.listener_for(&pressed, |pressed, _, window, _| {
                *pressed = !window.default_prevented();
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(&pressed, |pressed, event: &MouseUpEvent, window, _| {
                if std::mem::take(pressed) && event.click_count == 2 {
                    // Honor the macOS preference (zoom, minimize, or no action).
                    window.titlebar_double_click();
                }
            }),
        )
        .on_mouse_move(window.listener_for(
            &pressed,
            |pressed, event: &MouseMoveEvent, window, _| {
                if event.dragging() && std::mem::take(pressed) {
                    window.start_window_move();
                }
            },
        ))
}

impl Render for Studio {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let window_width = f32::from(window.viewport_size().width);
        let (source_width, inspector_width) = pane_widths(window_width);
        // Keep a usable preview even at the minimum supported window size.
        let source_max = (window_width - inspector_width - 360.).clamp(240., 560.);
        div()
            .id("monoprint-studio")
            .key_context(if self.browsing_themes {
                "Monoprint ThemeBrowser"
            } else {
                "Monoprint"
            })
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.chrome())
            .text_color(rgb(p.ink))
            .text_size(px(13.))
            .on_action(cx.listener(Self::open))
            .on_action(cx.listener(Self::export_file))
            .on_action(cx.listener(Self::copy_image))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::toggle_ui_appearance))
            .on_action(cx.listener(Self::show_shortcuts))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<ExternalPaths>, _, cx| {
                    if this.incoming_source.is_none() {
                        this.incoming_source = event
                            .dragged_item()
                            .downcast_ref::<ExternalPaths>()
                            .map(|files| files.paths().to_vec());
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_source_drop))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_source_drop))
            .on_file_drop_exit(cx.listener(|this, _, _, cx| {
                this.incoming_source = None;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &input::Escape, window, cx| {
                if this.browsing_themes {
                    this.close_theme_browser(window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        div().flex_1().min_w_0().h_full().child(
                            h_resizable("source-preview-split")
                                .with_state(&self.source_split)
                                .with_handle_appearance(std::rc::Rc::new(move |handle, _, _| {
                                    Some(
                                        div()
                                            .flex_none()
                                            .w(px(1.))
                                            .h_full()
                                            .bg(rgb(if handle.is_active() {
                                                p.accent
                                            } else {
                                                p.border
                                            }))
                                            .group_hover("handle", |line| line.bg(rgb(p.accent)))
                                            .into_any_element(),
                                    )
                                }))
                                .child(
                                    resizable_panel()
                                        .size(px(source_width))
                                        .size_range(px(240.)..px(source_max))
                                        .flex_none()
                                        .child(self.source_panel(window, cx)),
                                )
                                .child(
                                    resizable_panel()
                                        .size_range(px(360.)..Pixels::MAX)
                                        .child(self.canvas_panel(window, cx)),
                                ),
                        ),
                    )
                    .child(self.inspector(window, cx)),
            )
            // Editors own their mouse-up gestures. During a file drag, a topmost
            // target receives the drop before those handlers can consume it.
            .when(self.incoming_source.is_some(), |view| {
                view.child(
                    div()
                        .id("source-file-drop")
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_file_drop_exit(cx.listener(|this, _, _, cx| {
                            this.incoming_source = None;
                            cx.notify();
                        })),
                )
            })
            .children(self.selection_toolbar(cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
impl Studio {
    fn export_actions(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .flex()
            .gap(px(6.))
            .w_full()
            .child(
                Button::new("export-image")
                    .primary()
                    .h(px(34.))
                    .flex_1()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .label(if self.exporting {
                        "Exporting…".to_owned()
                    } else {
                        format!("Export {}", self.document.format.label())
                    })
                    .disabled(self.exporting)
                    .tooltip("Save an image · ⌘S")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.export_file(&ExportImage, window, cx)
                    })),
            )
            .child(
                Button::new("copy-image")
                    .outline()
                    .border_color(rgb(p.strong))
                    .bg(rgb(p.control))
                    .h(px(34.))
                    .px(px(9.))
                    .text_size(px(11.))
                    // Reserve space for Copying… so feedback cannot resize Export.
                    .w(px(110.))
                    .icon(if self.copy_state == CopyState::Copied {
                        IconName::Check
                    } else {
                        IconName::Copy
                    })
                    .label(match self.copy_state {
                        CopyState::Idle => "Copy",
                        CopyState::Copying => "Copying…",
                        CopyState::Copied => "Copied",
                    })
                    .disabled(self.exporting || self.copy_state == CopyState::Copying)
                    .tooltip("Copy PNG at the selected scale · ⌘⇧C")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.copy_image(&CopyImage, window, cx)),
                    ),
            )
    }

    fn drag_image(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .id("drag-image")
            .absolute()
            .left(px(8.))
            .top(px(8.))
            .invisible()
            .group_hover("preview-image", |view| view.visible())
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if *hovered {
                    this.prepare_drag_export(cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.prepare_drag_export(cx);
                    cx.stop_propagation();
                }),
            )
            .when_some(
                self.drag_export
                    .clone()
                    .filter(|export| export.revision == self.revision),
                |view, export| {
                    let thumbnail = self.preview.as_ref().map(|preview| preview.image.clone());
                    view.cursor(CursorStyle::OpenHand)
                        .on_drag(export, move |_, _, _, cx| {
                            cx.new(|_| DragThumbnail(thumbnail.clone()))
                        })
                        .external_drag_payload(|export: &crate::handoff::DragExport, _, _| {
                            Some(ExternalDragPayload::Files(FileDragPaths::new([(
                                export.path.clone(),
                                false,
                            )])))
                        })
                },
            )
            .child(
                Button::new("drag-image-button")
                    .outline()
                    .border_color(rgb(p.strong))
                    .bg(rgb(p.control))
                    .size(px(34.))
                    .p_0()
                    .loading(self.preparing_drag)
                    .child(Icon::new(IconName::File).size(px(18.)))
                    .accessibility_label("Drag PNG into another app")
                    .tooltip(if self.preparing_drag {
                        "Preparing PNG…"
                    } else {
                        "Drag PNG into another app at the selected scale"
                    }),
            )
    }
    fn source_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let highlighted =
            !self.document.highlights.is_empty() || !self.document.text_highlights.is_empty();
        div()
            .bg(rgb(p.canvas))
            .flex()
            .flex_col()
            .size_full()
            .child(
                header_bar("source-header", window, cx)
                    .h(px(40.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(if highlighted { 2. } else { 7. }))
                    // Leave a clear gap after the native traffic-light cluster.
                    // The language selector yields space before the icon buttons.
                    .pl(px(if cfg!(target_os = "macos") { 92. } else { 13. }))
                    .pr(px(9.))
                    .border_b_1()
                    .border_color(rgb(p.border))
                    .bg(rgb(p.surface))
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        Button::new("open-source")
                            .outline()
                            .xsmall()
                            .size(px(26.))
                            .flex_shrink_0()
                            .p_0()
                            .border_color(rgb(p.strong))
                            .bg(rgb(p.control))
                            .child(Icon::new(IconName::FolderOpen).size(px(16.)))
                            .accessibility_label("Open source file")
                            .tooltip("Open source file · ⌘O")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open(&OpenSource, window, cx)
                            })),
                    )
                    .child(
                        Button::new("paste-source")
                            .outline()
                            .xsmall()
                            .size(px(26.))
                            .flex_shrink_0()
                            .p_0()
                            .border_color(rgb(p.strong))
                            .bg(rgb(p.control))
                            .child(Icon::new(IconName::ClipboardPaste).size(px(16.)))
                            .accessibility_label("Paste source code")
                            .tooltip("Replace code with clipboard text")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.paste(&PasteSource, window, cx)
                            })),
                    )
                    .when(highlighted, |view| {
                        view.child(
                            Button::new("clear-highlights")
                                .outline()
                                .xsmall()
                                .size(px(26.))
                                .flex_shrink_0()
                                .p_0()
                                .border_color(rgb(p.strong))
                                .bg(rgb(p.control))
                                .child(Icon::new(IconName::Eraser).size(px(16.)))
                                .accessibility_label("Clear all highlights")
                                .tooltip("Clear all line and text highlights")
                                .on_click(cx.listener(|this, _, _, cx| this.clear_highlights(cx))),
                        )
                    })
                    .child(
                        dropdown(
                            p,
                            "source-language",
                            &self.language,
                            "Code language",
                            "Find a language…",
                        )
                        .w(px(160.))
                        .min_w(px(26.))
                        // Buttons prevent default on mouse-down; Select does not.
                        // Give the language trigger the same header-gesture boundary.
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default()),
                    )
                    .child(
                        Button::new("wrap-source")
                            .outline()
                            .xsmall()
                            .size(px(26.))
                            .flex_shrink_0()
                            .p_0()
                            .border_color(rgb(p.strong))
                            .bg(rgb(p.control))
                            .text_color(rgb(if self.wrap { p.ink } else { p.muted }))
                            .child(Icon::new(IconName::TextWrap).size(px(16.)))
                            .accessibility_label("Toggle soft wrap")
                            .toggled(self.wrap)
                            .tooltip(if self.wrap {
                                "Soft wrap: on"
                            } else {
                                "Soft wrap: off"
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.wrap = !this.wrap;
                                this.editor.update(cx, |editor, cx| {
                                    editor.set_soft_wrap(this.wrap, window, cx)
                                });
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .relative()
                    .pt(px(13.))
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        Editor::new(&self.editor)
                            .appearance(false)
                            .bordered(false)
                            .h_full()
                            .text_size(px(11.))
                            .line_height(relative(1.75))
                            .aria_label("Source code"),
                    )
                    .child(crate::source_lines::overlay(
                        &self.editor,
                        cx.entity().downgrade(),
                    )),
            )
    }
    fn selection_toolbar(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let p = palette(cx);
        let editor = self.editor.read(cx);
        let selection = editor.selected_range();
        let clicked = selection.is_empty() && self.highlight_menu.is_some();
        if selection.is_empty() && !clicked {
            return None;
        }
        if !editor
            .visible_row_range()?
            .contains(&(editor.cursor_position().line as usize))
        {
            return None;
        }
        let viewport = editor.input_bounds();
        let cursor = editor.cursor();
        let anchor = if clicked {
            self.highlight_menu.as_ref()?.anchor
        } else {
            editor.range_to_bounds(&(cursor..cursor))?
        };
        if anchor.bottom() < viewport.top()
            || anchor.top() > viewport.bottom()
            || viewport.size.height < px(40.)
        {
            return None;
        }
        let width = px(if clicked { 150. } else { 200. });
        let left = (anchor.center().x - width / 2.)
            .max(viewport.left() + px(4.))
            .min(viewport.right() - width - px(4.));
        let above = anchor.top() - px(36.) >= viewport.top();
        let top = if above {
            anchor.top() - px(36.)
        } else {
            (anchor.bottom() + px(4.)).min(viewport.bottom() - px(32.))
        };
        let marked =
            clicked || crate::selection::covers(&self.source_marks.get_ranges(cx), &selection);
        let marked_lines = crate::selection::covers(
            &self.line_highlights(cx),
            &crate::selection::selected_lines(&self.document.source, selection),
        );
        Some(
            deferred(
                div()
                    .occlude()
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        if this.highlight_menu.take().is_some() {
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .absolute()
                    .left(left)
                    .top(top)
                    .w(width)
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .p(px(3.))
                    .rounded(px(6.))
                    .bg(rgb(p.overlay))
                    .text_color(rgb(p.overlay_ink))
                    .shadow_md()
                    .child(
                        canvas(
                            |bounds, _, _| bounds,
                            move |bounds, _, window, _| {
                                let mut path = PathBuilder::fill();
                                let edge = if above { 0. } else { 4. };
                                let tip = if above { 4. } else { 0. };
                                path.move_to(bounds.origin + point(px(0.), px(edge)));
                                path.line_to(bounds.origin + point(px(8.), px(edge)));
                                path.line_to(bounds.origin + point(px(4.), px(tip)));
                                path.close();
                                if let Ok(path) = path.build() {
                                    window.paint_path(path, rgb(p.overlay));
                                }
                            },
                        )
                        .absolute()
                        .left((anchor.center().x - left - px(4.)).clamp(px(8.), width - px(16.)))
                        .top(px(if above { 30. } else { -4. }))
                        .w(px(8.))
                        .h(px(4.)),
                    )
                    .child(
                        Button::new("highlight-selection")
                            .custom(highlight_button_style(cx))
                            .xsmall()
                            .h(px(24.))
                            .text_size(px(11.))
                            .flex_1()
                            .label(if marked {
                                "Remove highlight"
                            } else {
                                "Highlight"
                            })
                            .tooltip(if marked {
                                "Remove highlighting from this selection"
                            } else {
                                "Highlight the selected text in the image"
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if clicked {
                                    this.remove_clicked_highlight(cx);
                                } else {
                                    this.highlight_selection(false, cx);
                                }
                            })),
                    )
                    .when(!clicked, |view| {
                        view.child(
                            Button::new("highlight-selected-lines")
                                .custom(highlight_button_style(cx))
                                .xsmall()
                                .h(px(24.))
                                .text_size(px(11.))
                                .label(if marked_lines { "Clear lines" } else { "Lines" })
                                .tooltip(if marked_lines {
                                    "Remove highlights from the selected lines"
                                } else {
                                    "Highlight the selected source lines"
                                })
                                .on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.highlight_selection(true, cx)
                                    }),
                                ),
                        )
                    }),
            )
            .with_priority(1),
        )
    }
    fn preview_size(&self) -> Option<gpui_kit::Size<Pixels>> {
        self.preview.as_ref().map(|preview| {
            size(
                px(preview.width as f32 / 2.),
                px(preview.height as f32 / 2.),
            )
        })
    }

    fn preview_canvas(
        &mut self,
        size: gpui_kit::Size<Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx);
        self.viewport.bounds.size = size;
        let mut stage = div()
            .id("preview-canvas")
            .size_full()
            .relative()
            .overflow_hidden()
            .flex()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                if this.canvas_resize.is_some() {
                    cx.stop_propagation();
                    return;
                }
                if let Some(image) = this.preview_size() {
                    this.viewport
                        .pan_by(event.delta.pixel_delta(px(24.)), image);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_pinch(cx.listener(|this, event: &PinchEvent, _, cx| {
                if this.canvas_resize.is_some() {
                    cx.stop_propagation();
                    return;
                }
                if let Some(image) = this.preview_size() {
                    this.viewport.zoom_by(event.delta, event.position, image);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    if this.preview.is_some() && this.canvas_resize.is_none() {
                        this.viewport.start_drag(event.position);
                        cx.stop_propagation();
                        cx.notify();
                    }
                }),
            );
        if let Some(preview) = &self.preview {
            let image_size = self.preview_size().unwrap();
            if self.canvas_resize.is_none() {
                self.viewport.constrain_pan(image_size);
            }
            let scale = self.viewport.scale(image_size);
            let displayed = image_size.map(|length| length * scale);
            let image_origin = self.viewport.origin(image_size);
            let (origin, width) = self
                .canvas_resize
                .as_ref()
                .map(|resize| {
                    let width = px(resize.width as f32);
                    (resize.origin(width), width * resize.scale)
                })
                .unwrap_or((image_origin, displayed.width));
            let active = self.canvas_resize.is_some();
            let handle_y = ((px(0.) - origin.y).max(px(0.))
                + (size.height - origin.y).min(displayed.height))
                / 2.;
            stage = stage
                .cursor(if active {
                    CursorStyle::ResizeLeftRight
                } else if self.viewport.is_dragging() {
                    CursorStyle::ClosedHand
                } else {
                    CursorStyle::OpenHand
                })
                .child(
                    div()
                        .id("preview-image")
                        .group("preview-image")
                        .test_support()
                        .absolute()
                        .left(image_origin.x)
                        .top(image_origin.y)
                        .w(displayed.width)
                        .h(displayed.height)
                        .child(canvas_checker(p))
                        // Keep the entire stale image visible. Only the outline
                        // follows the pointer until decoded replacement pixels arrive.
                        .child(
                            img(preview.image.clone())
                                .size_full()
                                .object_fit(ObjectFit::Contain),
                        )
                        .when(!active && !self.viewport.is_dragging(), |view| {
                            view.child(self.drag_image(cx))
                        }),
                )
                .child(
                    div()
                        .absolute()
                        .left(origin.x)
                        .top(origin.y)
                        .w(width)
                        .h(displayed.height)
                        .child(div().absolute().inset_0().border_1().border_color(
                            rgb(if active { p.accent } else { p.strong }).opacity(if active {
                                1.
                            } else {
                                0.5
                            }),
                        ))
                        .child(
                            div()
                                .absolute()
                                .top(displayed.height + px(9.))
                                .w_full()
                                .text_size(px(10.))
                                .text_color(rgb(p.muted))
                                .child(if active {
                                    format!("{} px wide", self.document.width)
                                } else {
                                    format!("{} × {}", preview.width / 2, preview.height / 2)
                                }),
                        ),
                );
            for (id, edge, x) in [
                ("resize-image-left", ResizeEdge::Left, origin.x),
                ("resize-image-right", ResizeEdge::Right, origin.x + width),
            ] {
                let dragging = self
                    .canvas_resize
                    .as_ref()
                    .is_some_and(|resize| resize.is_dragging_edge(edge));
                stage = stage.child(
                    div()
                        .group(id)
                        .absolute()
                        .left(x - px(24.))
                        .top(origin.y + handle_y - px(36.))
                        .w(px(48.))
                        .h(px(72.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id(id)
                                .w(px(24.))
                                .h(px(44.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor(CursorStyle::ResizeLeftRight)
                                .aria_label("Resize image width")
                                .tooltip(|window, cx| {
                                    Tooltip::new("Drag to resize image width · height follows code")
                                        .build(window, cx)
                                })
                                .child(
                                    div()
                                        .w(px(7.))
                                        .h(px(28.))
                                        .rounded(px(4.))
                                        .bg(rgb(p.control))
                                        .border_1()
                                        .border_color(rgb(p.accent))
                                        .when(!dragging, |grip| {
                                            grip.invisible().group_hover(id, |grip| grip.visible())
                                        }),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                        this.begin_canvas_resize(edge, event.position, cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .test_support(),
                        ),
                );
            }
        } else {
            stage = stage.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .text_color(rgb(p.muted))
                    .m_auto()
                    .child(Icon::new(IconName::Image).size(px(28.)))
                    .child(if self.revision != self.preview_revision {
                        "Preparing your canvas…"
                    } else {
                        "Your code image appears here"
                    }),
            );
        }
        // Track actual canvas bounds and finish an active drag even when the
        // pointer leaves the canvas. No other pane can initiate a canvas drag.
        let studio = cx.entity().downgrade();
        stage.child(
            canvas(
                cx.processor(|this, bounds, _, _| this.viewport.bounds = bounds),
                move |_, _, window, _| {
                    let moving = studio.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if !phase.capture() {
                            return;
                        }
                        let _ = moving.update(cx, |this, cx| {
                            if this
                                .canvas_resize
                                .as_ref()
                                .is_some_and(|resize| resize.dragging)
                            {
                                if event.pressed_button == Some(MouseButton::Left) {
                                    this.update_canvas_resize(event.position, window, cx);
                                } else {
                                    this.end_canvas_resize(cx);
                                }
                                cx.stop_propagation();
                            } else if this.viewport.is_dragging() {
                                if event.pressed_button == Some(MouseButton::Left) {
                                    if let Some(image) = this.preview_size() {
                                        this.viewport.drag_to(event.position, image);
                                    }
                                    cx.stop_propagation();
                                } else {
                                    this.viewport.end_drag();
                                }
                                cx.notify();
                            }
                        });
                    });
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase.capture() && event.button == MouseButton::Left {
                            let _ = studio.update(cx, |this, cx| {
                                if this
                                    .canvas_resize
                                    .as_ref()
                                    .is_some_and(|resize| resize.dragging)
                                {
                                    this.update_canvas_resize(event.position, window, cx);
                                    this.end_canvas_resize(cx);
                                    cx.stop_propagation();
                                } else if this.viewport.is_dragging() {
                                    this.viewport.end_drag();
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            });
                        }
                    });
                },
            )
            .absolute()
            .size_full(),
        )
    }

    fn canvas_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(p.preview_background())
            .child(
                header_bar("canvas-header", window, cx)
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(40.))
                    .flex_shrink_0()
                    .px(px(13.))
                    .border_b_1()
                    .border_color(rgb(p.border))
                    .bg(p.chrome())
                    .window_control_area(WindowControlArea::Drag)
                    .gap(px(7.))
                    .child(div().flex_1())
                    .when_some(
                        match self.viewport.zoom {
                            PreviewZoom::Custom(scale) => Some(scale),
                            _ => None,
                        },
                        |view, scale| {
                            view.child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(p.muted))
                                    .child(format!("{:.0}%", scale * 100.)),
                            )
                        },
                    )
                    .child(
                        segments(p).children(
                            [
                                ("fit-preview", "Fit", PreviewZoom::Fit),
                                ("half-preview", "50%", PreviewZoom::Half),
                                ("actual-preview", "100%", PreviewZoom::Actual),
                            ]
                            .into_iter()
                            .map(|(id, label, zoom)| {
                                segment(p, id)
                                    .label(label)
                                    .selected(self.viewport.zoom == zoom)
                                    .toggled(self.viewport.zoom == zoom)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.canvas_resize = None;
                                        this.viewport.reset(zoom);
                                        cx.notify();
                                    }))
                            }),
                        ),
                    )
                    .child(
                        Button::new("ui-appearance")
                            .outline()
                            .size(px(26.))
                            .p_0()
                            .border_color(rgb(p.strong))
                            .bg(rgb(p.control))
                            .child(
                                Icon::new(if p.light {
                                    IconName::Moon
                                } else {
                                    IconName::Sun
                                })
                                .size(px(16.)),
                            )
                            .accessibility_label(if p.light {
                                "Use dark interface"
                            } else {
                                "Use light interface"
                            })
                            .tooltip("Toggle interface appearance · ⌘⇧L")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_ui_appearance(&ToggleUiAppearance, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .flex()
                    .overflow_hidden()
                    .child(container_query(cx.processor(|this, size, _, cx| {
                        this.preview_canvas(size, cx).into_any_element()
                    }))),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    div()
                        .mx_5()
                        .mb_3()
                        .p_3()
                        .rounded_md()
                        .bg(rgb(if p.light { 0xFBEAEA } else { 0x382627 }))
                        .text_color(rgb(if p.light { 0xA33838 } else { 0xFFB4B4 }))
                        .text_size(px(12.))
                        .child(error)
                        .when(self.preview.is_some(), |view| {
                            view.child(div().mt_2().child("Showing the previous preview."))
                        }),
                )
            })
            .when_some(self.notice.clone(), |view, notice| {
                view.child(
                    div()
                        .mx_5()
                        .mb_3()
                        .p_3()
                        .rounded_md()
                        .bg(rgb(p.accent).opacity(0.12))
                        .text_color(rgb(p.accent))
                        .text_size(px(12.))
                        .child(notice),
                )
            })
    }
    fn section_header(
        &self,
        section: InspectorSection,
        detail: impl Into<SharedString>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx);
        let open = self.open_sections[section as usize];
        let on_change = cx.listener(move |this, open: &bool, _, cx| {
            this.open_sections[section as usize] = *open;
            cx.notify();
        });
        gpui_kit::base::AccordionTrigger::new(section.id())
            .open(open)
            .aria_label(format!("{} settings", section.title()))
            .cursor_pointer()
            .h(px(24.))
            .w_full()
            .flex()
            .items_center()
            .gap(px(6.))
            .text_color(rgb(p.muted))
            .hover(|style| style.text_color(rgb(p.ink)))
            .child(
                Icon::new(if open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size(px(12.)),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(10.))
                    .child(section.title().to_uppercase()),
            )
            .child(div().text_size(px(11.)).truncate().child(detail.into()))
            .on_change(move |open, _, window, cx| on_change(&open, window, cx))
    }

    fn theme_settings(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let theme = self
            .document
            .theme
            .parse::<ImageTheme>()
            .unwrap_or_default();
        let image = self
            .thumbnails
            .iter()
            .find(|(item, _)| *item == theme)
            .map(|(_, image)| {
                if self.document.light {
                    self.light_thumbnails.get(&theme).unwrap_or(image)
                } else {
                    image
                }
            });
        settings_section(p)
            .open(self.open_sections[InspectorSection::Theme as usize])
            .child(self.section_header(InspectorSection::Theme, "", cx))
            .content(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        Button::new("browse-themes")
                            .ghost()
                            .w_full()
                            .h(px(58.))
                            .p(px(8.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(rgb(p.strong))
                            .bg(rgb(p.control))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .w_full()
                                    .min_w_0()
                                    .when_some(image, |view, image| {
                                        view.child(
                                            div()
                                                .w(px(66.))
                                                .h(px(38.))
                                                .flex_shrink_0()
                                                .rounded(px(3.))
                                                .overflow_hidden()
                                                .child(
                                                    img(image.clone())
                                                        .size_full()
                                                        .object_fit(ObjectFit::Cover),
                                                ),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_size(px(12.))
                                            .truncate()
                                            .child(theme.name()),
                                    )
                                    .child(
                                        Icon::new(IconName::ChevronRight)
                                            .size(px(14.))
                                            .text_color(rgb(p.muted)),
                                    ),
                            )
                            .accessibility_label(format!(
                                "Browse themes, selected: {}",
                                theme.name()
                            ))
                            .tooltip("Browse themes")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.browse_themes(window, cx)),
                            ),
                    )
                    .children(self.theme_appearance(cx)),
            )
    }

    fn theme_appearance(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let p = palette(cx);
        let theme = self
            .document
            .theme
            .parse::<ImageTheme>()
            .unwrap_or_default();
        (theme.supports(Appearance::Light) && theme.supports(Appearance::Dark)).then(|| {
            row(
                p,
                "Appearance",
                segments(p).children(
                    [
                        ("light-appearance", "light", Appearance::Light),
                        ("dark-appearance", "dark", Appearance::Dark),
                    ]
                    .into_iter()
                    .map(|(id, label, appearance)| {
                        let light = appearance == Appearance::Light;
                        segment(p, id)
                            .label(label)
                            .selected(self.document.light == light)
                            .toggled(self.document.light == light)
                            .tooltip(format!("Use the {label} variant"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.document.light = light;
                                this.changed(cx);
                            }))
                    }),
                ),
            )
        })
    }

    fn theme_browser(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let count = self.filtered_themes.len();
        let gallery = uniform_list(
            "theme-gallery",
            count.div_ceil(3),
            cx.processor(|this, rows: std::ops::Range<usize>, _, cx| {
                rows.map(|row| {
                    let mut tiles = div().flex().gap(px(7.)).w_full().h(px(73.)).pb(px(7.));
                    for column in 0..3 {
                        tiles = tiles.child(
                            div().flex_1().min_w_0().children(
                                this.filtered_themes
                                    .get(row * 3 + column)
                                    .map(|index| this.theme_tile(*index, cx)),
                            ),
                        );
                    }
                    tiles
                })
                .collect::<Vec<_>>()
            }),
        )
        .size_full()
        .track_scroll(&self.theme_scroll);
        div()
            .id("theme-browser")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .gap(px(10.))
                    .p(px(13.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.))
                            .child(
                                Button::new("close-theme-browser")
                                    .ghost()
                                    .small()
                                    .px(px(3.))
                                    .icon(Icon::new(IconName::ArrowLeft).size(px(14.)))
                                    .label("Back")
                                    .accessibility_label("Back to settings")
                                    .tooltip("Back to settings (Esc)")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_theme_browser(window, cx)
                                    })),
                            )
                            .child(
                                segments(p).children(
                                    [
                                        ("themes-all", "all", ThemeFilter::All),
                                        ("themes-light", "light", ThemeFilter::Light),
                                        ("themes-dark", "dark", ThemeFilter::Dark),
                                    ]
                                    .into_iter()
                                    .map(
                                        |(id, label, filter)| {
                                            segment(p, id)
                                                .label(label)
                                                .selected(self.theme_filter == filter)
                                                .toggled(self.theme_filter == filter)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.theme_filter = filter;
                                                    this.filter_themes(cx);
                                                }))
                                        },
                                    ),
                                ),
                            ),
                    )
                    .child(
                        text_input(p, &self.theme_search)
                            .cleanable(true)
                            .prefix(Icon::new(IconName::Search).size(px(13.)))
                            .aria_label("Find a theme"),
                    ),
            )
            .child(
                div().flex_1().min_h_0().px(px(13.)).pb(px(6.)).child(
                    div()
                        .id("theme-gallery-viewport")
                        .relative()
                        .size_full()
                        .overflow_hidden()
                        .when(count > 0, |view| {
                            view.child(gallery)
                                .child(Scrollbar::vertical(&self.theme_scroll))
                        })
                        .when(count == 0, |view| {
                            view.child(
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .text_size(px(12.))
                                    .text_color(rgb(p.muted))
                                    .child("No matching themes")
                                    .child(
                                        Button::new("clear-theme-search")
                                            .ghost()
                                            .small()
                                            .label("Clear search")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.theme_search.update(cx, |input, cx| {
                                                    input.replace_all("", window, cx)
                                                });
                                            })),
                                    ),
                            )
                        }),
                ),
            )
            .children(self.theme_appearance(cx).map(|control| {
                div()
                    .flex_shrink_0()
                    .px(px(13.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(rgb(p.border))
                    .child(control)
            }))
    }

    fn theme_tile(&self, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let (theme, image) = &self.thumbnails[index];
        let image = if self.theme_filter == ThemeFilter::Light {
            self.light_thumbnails.get(theme).unwrap_or(image)
        } else {
            image
        };
        let theme = *theme;
        let selected = self.document.theme == theme.id()
            && self
                .theme_filter
                .appearance()
                .is_none_or(|appearance| self.document.light == (appearance == Appearance::Light));
        Button::new(("theme-swatch", index))
            .ghost()
            .p_0()
            .w_full()
            .min_w_0()
            .h_full()
            .rounded(px(5.))
            .overflow_hidden()
            .border_1()
            .border_color(rgb(if selected { p.accent } else { p.strong }))
            .bg(rgb(p.canvas))
            .selected(selected)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .child(
                        div()
                            .h(px(43.))
                            .w_full()
                            .overflow_hidden()
                            .bg(rgb(p.stage))
                            .child(
                                img(image.clone())
                                    .size_full()
                                    .object_fit(ObjectFit::Contain),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .h(px(23.))
                            .px(px(7.))
                            .border_t_1()
                            .border_color(rgb(p.border))
                            .text_size(px(10.))
                            .text_color(rgb(if selected { p.ink } else { p.muted }))
                            .child(div().flex_1().min_w_0().truncate().child(theme.name()))
                            .when(selected, |view| {
                                view.child(
                                    Icon::new(IconName::Check)
                                        .size(px(11.))
                                        .text_color(rgb(p.accent)),
                                )
                            }),
                    ),
            )
            .accessibility_label(format!("{} theme", theme.name()))
            .tooltip(theme.name())
            .on_click(cx.listener(move |this, _, window, cx| this.select_theme(theme, window, cx)))
    }

    fn typography_settings(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        settings_section(p)
            .open(self.open_sections[InspectorSection::Typeface as usize])
            .child(self.section_header(InspectorSection::Typeface, "", cx))
            .content(div().flex().flex_col().gap(px(9.))
            .child(self.font_control(false, cx))
            .child(self.font_control(true, cx))
            .child(self.font_size_control(cx))
            .child(row(p, "Coding ligatures", Switch::new("coding-ligatures").small()
                .accessibility_label("Coding ligatures")
                .tooltip("Combine supported code symbols, such as => and !=. Depends on the selected typeface.")
                .checked(self.document.ligatures)
                .on_change(cx.listener(|this, value, _, cx| {
                    this.document.ligatures = *value;
                    this.changed(cx);
                })))))
    }

    fn font_size_control(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let value = self.document.font_size.trim().parse::<i32>().ok();
        div()
            .flex()
            .items_center()
            .gap(px(9.))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(64.))
                    .text_size(px(11.))
                    .text_color(rgb(p.ink2))
                    .child("Size"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(
                        text_input(p, &self.font_size)
                            .id("font-size")
                            .flex_1()
                            .min_w_0()
                            .aria_label("Font size")
                            .suffix(
                                div()
                                    .text_color(rgb(p.muted))
                                    .text_size(px(11.))
                                    .child("px"),
                            ),
                    )
                    .children(
                        [
                            (
                                "decrease-font-size",
                                IconName::Minus,
                                "Decrease font size",
                                -1,
                            ),
                            (
                                "increase-font-size",
                                IconName::Plus,
                                "Increase font size",
                                1,
                            ),
                        ]
                        .into_iter()
                        .map(|(id, icon, description, delta)| {
                            Button::new(id)
                                .outline()
                                .xsmall()
                                .size(px(26.))
                                .flex_shrink_0()
                                .p_0()
                                .border_color(rgb(p.strong))
                                .bg(rgb(p.control))
                                .text_color(rgb(p.ink))
                                .child(Icon::new(icon).size(px(16.)))
                                .accessibility_label(description)
                                .tooltip(description)
                                .disabled(value == Some(if delta < 0 { 8 } else { 72 }))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let next = this
                                        .document
                                        .font_size
                                        .trim()
                                        .parse::<i32>()
                                        .unwrap_or(15)
                                        .saturating_add(delta)
                                        .clamp(8, 72);
                                    this.font_size.update(cx, |state, cx| {
                                        state.replace_all(next.to_string(), window, cx);
                                    });
                                }))
                        }),
                    ),
            )
    }

    fn font_control(&self, comments: bool, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let (state, label) = if comments {
            (&self.comment_typeface, "Comment typeface")
        } else {
            (&self.code_typeface, "Code typeface")
        };
        div()
            .flex()
            .items_center()
            .gap(px(9.))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(64.))
                    .text_size(px(11.))
                    .text_color(rgb(p.ink2))
                    .child(if comments { "Comments" } else { "Code" }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(
                        dropdown(
                            p,
                            if comments {
                                "comment-typeface"
                            } else {
                                "code-typeface"
                            },
                            state,
                            label,
                            "Find a typeface…",
                        )
                        .flex_1(),
                    )
                    .when(
                        !self.document.typeface_selection(comments).is_empty(),
                        |view| {
                            view.child(
                                Button::new(if comments {
                                    "reset-comment-font"
                                } else {
                                    "theme-font"
                                })
                                .ghost()
                                .xsmall()
                                .size(px(26.))
                                .flex_shrink_0()
                                .p_0()
                                .child(Icon::new(IconName::RotateCcw).size(px(16.)))
                                .accessibility_label(if comments {
                                    "Reset comment typeface"
                                } else {
                                    "Reset code typeface"
                                })
                                .tooltip(if comments {
                                    "Use code typeface"
                                } else {
                                    "Use theme typeface"
                                })
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.document.select_typeface(comments, "");
                                        this.sync_typeface(comments, window, cx);
                                        this.changed(cx);
                                    },
                                )),
                            )
                        },
                    ),
            )
    }

    fn padding_input(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let input = self.padding.clone();
        text_input(p, &self.padding)
            .aria_label("Image padding")
            .suffix(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_color(rgb(p.muted))
                            .text_size(px(11.))
                            .child("px"),
                    )
                    .child(
                        Button::new("padding-presets")
                            .ghost()
                            .xsmall()
                            .size(px(18.))
                            .p_0()
                            .child(Icon::new(IconName::ChevronDown).size(px(14.)))
                            .accessibility_label("Padding presets")
                            .tooltip("Choose padding or type an exact value")
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |mut menu, _, cx| {
                                let selected = input.read(cx).value().parse::<u32>().ok();
                                for padding in [0u32, 16, 32, 64, 96] {
                                    let input = input.clone();
                                    menu = menu.item(
                                        PopupMenuItem::new(format!("{padding} px"))
                                            .checked(selected == Some(padding))
                                            .on_click(move |_, window, cx| {
                                                input.update(cx, |state, cx| {
                                                    state.replace_all(
                                                        padding.to_string(),
                                                        window,
                                                        cx,
                                                    )
                                                });
                                            }),
                                    );
                                }
                                menu
                            }),
                    ),
            )
    }

    fn advanced_export_settings(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        Popover::new("advanced-export-popover")
            .anchor(Anchor::BottomRight)
            .trigger(
                Button::new("advanced-export")
                    .ghost()
                    .xsmall()
                    .size(px(24.))
                    .p_0()
                    .child(Icon::new(IconName::SlidersHorizontal).size(px(16.)))
                    .accessibility_label("Advanced export settings")
                    .tooltip("Advanced export settings"),
            )
            .child(
                div()
                    .w(px(250.))
                    .p(px(10.))
                    .text_color(rgb(p.ink))
                    .child(row(
                        p,
                        "Compression",
                        segments(p).children(
                            [
                                ("fast", "Fast", "Faster encoding. Same image quality."),
                                (
                                    "balanced",
                                    "Balanced",
                                    "Smaller PNG files. Same image quality.",
                                ),
                            ]
                            .into_iter()
                            .map(|(mode, label, help)| {
                                segment(p, format!("png-compression-{mode}"))
                                    .label(label)
                                    .tooltip(help)
                                    .selected(self.document.png_compression == mode)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.document.png_compression = mode.into();
                                        this.changed(cx);
                                    }))
                            }),
                        ),
                    )),
            )
    }

    fn inspector_settings(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .id("style-inspector")
            .track_scroll(&self.settings_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .child(self.theme_settings(cx))
            .child(self.typography_settings(cx))
            .child(
                settings_section(p)
                    .open(self.open_sections[InspectorSection::Frame as usize])
                    .child(self.section_header(InspectorSection::Frame, "", cx))
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(9.))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(9.))
                                    .child(
                                        field(
                                            p,
                                            "Width",
                                            text_input(p, &self.width)
                                                .aria_label("Image width")
                                                .suffix(
                                                    div()
                                                        .text_color(rgb(p.muted))
                                                        .text_size(px(11.))
                                                        .child("px"),
                                                ),
                                        )
                                        .flex_1(),
                                    )
                                    .child(field(p, "Padding", self.padding_input(cx)).flex_1()),
                            )
                            .child(field(
                                p,
                                "Backdrop",
                                segments(p).children(Background::ALL.into_iter().map(
                                    |background| {
                                        let label = match background {
                                            Background::Theme => "Theme",
                                            Background::Solid => "Solid",
                                            Background::Lattice => "Lattice",
                                            Background::Halftone => "Halftone",
                                            Background::None => "None",
                                        };
                                        segment(p, format!("background-{background}"))
                                            .flex_1()
                                            .px(px(2.))
                                            .text_size(px(10.))
                                            .min_w_0()
                                            .label(label)
                                            .selected(
                                                self.document.background().ok() == Some(background),
                                            )
                                            .tooltip(if background == Background::None {
                                                "Transparent canvas; keep the code window"
                                            } else {
                                                label
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.document.select_background(background);
                                                this.changed(cx);
                                            }))
                                    },
                                )),
                            ))
                            .child(row(
                                p,
                                "Line numbers",
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .when(self.document.line_numbers.is_some(), |view| {
                                        view.child(
                                            Button::new("reset-line-numbers")
                                                .ghost()
                                                .xsmall()
                                                .size(px(26.))
                                                .flex_shrink_0()
                                                .p_0()
                                                .child(Icon::new(IconName::RotateCcw).size(px(16.)))
                                                .accessibility_label(
                                                    "Use theme default line numbers",
                                                )
                                                .tooltip("Use theme default")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.document.line_numbers = None;
                                                    this.changed(cx);
                                                })),
                                        )
                                    })
                                    .child(
                                        Switch::new("line-numbers")
                                            .small()
                                            .accessibility_label("Line numbers")
                                            .checked(self.document.numbers_visible())
                                            .disabled(
                                                self.document.language == Language::PLAIN.id(),
                                            )
                                            .on_change(cx.listener(|this, value, _, cx| {
                                                this.document.line_numbers = Some(*value);
                                                this.changed(cx);
                                            })),
                                    ),
                            ))
                            .child(field(
                                p,
                                "Window title",
                                text_input(p, &self.title).aria_label("Window title"),
                            )),
                    ),
            )
    }

    fn inspector(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .w(px(pane_widths(f32::from(window.viewport_size().width)).1))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(p.strong))
            .bg(p.inspector_background())
            .child(if self.browsing_themes {
                self.theme_browser(cx).into_any_element()
            } else {
                self.inspector_settings(cx).into_any_element()
            })
            .child(
                Collapsible::new()
                    .open(self.open_sections[InspectorSection::Export as usize])
                    .px(px(13.))
                    .pt(px(12.))
                    .pb(px(13.))
                    .bg(p.chrome())
                    .border_t_1()
                    .border_color(rgb(p.border))
                    .flex_shrink_0()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div().flex_1().min_w_0().child(
                                    self.section_header(
                                        InspectorSection::Export,
                                        self.preview
                                            .as_ref()
                                            .map(|preview| {
                                                format!(
                                                    "{} × {}",
                                                    preview.width / 2 * self.document.scale,
                                                    preview.height / 2 * self.document.scale
                                                )
                                            })
                                            .unwrap_or_else(|| "—".into()),
                                        cx,
                                    ),
                                ),
                            )
                            .when(self.document.format == Format::Png, |view| {
                                view.child(self.advanced_export_settings(cx))
                            }),
                    )
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(9.))
                            .child(segments(p).children(
                                [Format::Png, Format::Svg, Format::Html].into_iter().map(
                                    |format| {
                                        segment(p, format.extension())
                                            .flex_1()
                                            .label(format.label())
                                            .selected(self.document.format == format)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.document.format = format;
                                                this.changed(cx);
                                            }))
                                    },
                                ),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(p.muted))
                                            .child("Scale"),
                                    )
                                    .child(segments(p).flex_1().children(
                                        [1, 2, 3, 4, 5, 6].into_iter().map(|scale| {
                                            segment(p, ("scale", scale))
                                                .px(px(3.))
                                                .flex_1()
                                                .label(format!("{scale}×"))
                                                .selected(self.document.scale == scale)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.document.scale = scale;
                                                    this.changed(cx);
                                                }))
                                        }),
                                    )),
                            ),
                    )
                    .child(self.export_actions(cx))
                    .when_some(self.last_export.clone(), |view, path| {
                        view.child(
                            Button::new("reveal-export")
                                .ghost()
                                .xsmall()
                                .label("Show last export in folder")
                                .on_click(move |_, _, cx| cx.reveal_path(&path)),
                        )
                    }),
            )
    }
}
impl Choice {
    pub fn menu_item(&self, cx: &App) -> AnyElement {
        let Some(preview) = self.preview else {
            return self.name.clone().into_any_element();
        };
        let p = palette(cx);
        let (sample, detail) = match preview {
            ChoicePreview::Typeface(font) => (
                div()
                    .font_family(if font.id() == "commit-mono" {
                        "CommitMono"
                    } else {
                        font.name()
                    })
                    .text_size(px(21.))
                    .child("{Aa}"),
                if font.id() == "inter" {
                    "proportional"
                } else {
                    "monospace"
                },
            ),
            ChoicePreview::Inherited => (
                div().text_size(px(22.)).text_color(rgb(p.muted)).child("↳"),
                "Use the default typeface",
            ),
            ChoicePreview::Custom => (
                div().child(Icon::new(IconName::File).size(px(20.))),
                "Load a TTF or OTF file",
            ),
        };
        // Keep every row the same height, including inheritance and file choices,
        // so the dropdown's virtual list can measure and scroll them correctly.
        div()
            .flex()
            .items_center()
            .h(px(38.))
            .gap(px(10.))
            .child(div().w(px(54.)).flex_shrink_0().child(sample))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(div().text_size(px(11.)).truncate().child(self.name.clone()))
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child(detail),
                    ),
            )
            .into_any_element()
    }
}

fn settings_section(p: Palette) -> Collapsible {
    Collapsible::new()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .px(px(13.))
        .pt(px(13.))
        .pb(px(17.))
        .gap(px(9.))
        .border_b_1()
        .border_color(rgb(p.border))
}

fn segments(p: Palette) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .h(px(26.))
        .p(px(2.))
        .gap(px(2.))
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(p.strong))
        .bg(rgb(p.control))
}

fn segment(p: Palette, id: impl Into<ElementId>) -> Button {
    Button::new(id)
        .text_color(rgb(p.ink2))
        .ghost()
        .xsmall()
        .h(px(20.))
        .min_w(px(26.))
        .px(px(7.))
        .rounded(px(3.))
        .text_size(px(11.))
}

fn highlight_button_style(cx: &App) -> ButtonCustomVariant {
    let p = palette(cx);
    ButtonCustomVariant::new(cx)
        .foreground(rgb(p.overlay_ink).into())
        .hover(rgb(0x7F7F7F).opacity(0.22).into())
        .active(rgb(0x7F7F7F).opacity(0.3).into())
}

// Select styles apply to its trigger, while its outer element fills its parent.
// Give that parent an explicit control height so toolbar centering and hit areas agree.
fn dropdown(
    p: Palette,
    id: &'static str,
    state: &Entity<Choices>,
    label: &'static str,
    search: &'static str,
) -> Div {
    div().h(px(26.)).min_w_0().child(
        Select::new(state)
            .id(id)
            .xsmall()
            .focus_ring(false)
            .h(px(26.))
            .w_full()
            .px(px(7.))
            .bg(rgb(p.control))
            .text_color(rgb(p.ink))
            .text_size(px(11.))
            .menu_width(px(240.))
            .menu_max_h(px(280.))
            .accessibility_label(label)
            .search_placeholder(search),
    )
}

fn text_input(p: Palette, state: &Entity<input::InputState>) -> Input {
    // Input::h is for multi-line editors; single-line controls use Styled::h.
    Styled::h(Input::new(state).small(), px(26.))
        .flex_shrink_0()
        .px(px(7.))
        .text_size(px(11.))
        .bg(rgb(p.control))
}

fn field(p: Palette, label: &'static str, element: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .min_w_0()
        .gap(px(5.))
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(p.muted))
                .child(label),
        )
        .child(element)
}
fn row(p: Palette, label: &'static str, element: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .flex_shrink_0()
        .min_h(px(26.))
        .text_size(px(11.))
        .text_color(rgb(p.ink2))
        .child(label)
        .child(element)
}
fn canvas_checker(p: Palette) -> impl IntoElement {
    canvas(
        |bounds, _, _| bounds,
        move |bounds, _, window, _| {
            let cell = px(7.);
            let visible = bounds.intersect(&window.content_mask().bounds);
            window.paint_quad(fill(visible, rgb(p.stage)));
            let first_x = (f32::from(visible.left() - bounds.left()) / 7.).floor() as u32;
            let first_y = (f32::from(visible.top() - bounds.top()) / 7.).floor() as u32;
            let end_x = (f32::from(visible.right() - bounds.left()) / 7.).ceil() as u32;
            let end_y = (f32::from(visible.bottom() - bounds.top()) / 7.).ceil() as u32;
            for y in first_y..end_y {
                for x in (first_x + (first_x + y) % 2..end_x).step_by(2) {
                    let origin = bounds.origin + point(cell * x as f32, cell * y as f32);
                    let size = size(
                        cell.min(bounds.right() - origin.x),
                        cell.min(bounds.bottom() - origin.y),
                    );
                    window.paint_quad(fill(Bounds::new(origin, size), rgb(p.surface)));
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

// Match the reference's clamped panel widths at each native window size.
fn pane_widths(width: f32) -> (f32, f32) {
    (
        (width * 0.22).clamp(240., 318.),
        (width * 0.21).clamp(260., 300.),
    )
}
