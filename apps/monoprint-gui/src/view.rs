use crate::{appearance::*, document::Format, studio::*};
use gpui_kit::assets::IconName;
use gpui_kit::{
    component::{
        button::*,
        input::{Editor, Input},
        select::Select,
        switch::Switch,
        *,
    },
    prelude::*,
    *,
};
use monoprint::{Appearance, Language, LineNumberPolicy, Theme as ImageTheme};

impl Render for Studio {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("monoprint-studio")
            .key_context("Monoprint")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(chrome_background())
            .text_color(rgb(TEXT))
            .text_size(px(13.))
            .on_action(cx.listener(Self::open))
            .on_action(cx.listener(Self::export_file))
            .on_action(cx.listener(Self::copy_image))
            .on_action(cx.listener(Self::paste))
            .child(self.toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.source_panel(cx))
                    .child(self.canvas_panel(window, cx))
                    .child(self.inspector(cx)),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
impl Studio {
    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .window_control_area(WindowControlArea::Drag)
            .flex()
            .items_center()
            .justify_between()
            .h(px(60.))
            .flex_shrink_0()
            .pr_4()
            .border_b_1()
            .border_color(rgb(BORDER))
            .pl(px(if cfg!(target_os = "macos") { 88. } else { 18. }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w(px(26.))
                            .h(px(29.))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x697383))
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_family("JetBrains Mono")
                            .text_size(px(17.))
                            .child("m"),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_size(px(16.))
                            .child("Monoprint"),
                    )
                    .child(
                        div()
                            .ml_3()
                            .text_color(rgb(MUTED))
                            .text_size(px(12.))
                            .child("Code, composed."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("open-source")
                            .ghost()
                            .small()
                            .icon(IconName::FolderOpen)
                            .label("Open")
                            .tooltip("Open source file · ⌘O")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open(&OpenSource, window, cx)
                            })),
                    )
                    .child(
                        Button::new("copy-image")
                            .outline()
                            .small()
                            .icon(IconName::Copy)
                            .label("Copy image")
                            .disabled(self.exporting)
                            .tooltip("Copy PNG at the selected scale · ⌘⇧C")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.copy_image(&CopyImage, window, cx)
                            })),
                    )
                    .child(
                        Button::new("export-image")
                            .primary()
                            .small()
                            .icon(IconName::Download)
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
                    ),
            )
    }
    fn source_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(rgb(SURFACE))
            .flex()
            .flex_col()
            .w(px(320.))
            .flex_shrink_0()
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .h(px(52.))
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_shrink_0()
                    .child(section("Source"))
                    .child(
                        Button::new("wrap-source")
                            .ghost()
                            .xsmall()
                            .label("Wrap")
                            .selected(self.wrap)
                            .tooltip("Wrap long lines in the editor")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.wrap = !this.wrap;
                                this.editor.update(cx, |editor, cx| {
                                    editor.set_soft_wrap(this.wrap, window, cx)
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("paste-source")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Clipboard)
                            .label("Paste")
                            .tooltip("Replace code with clipboard text")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.paste(&PasteSource, window, cx)
                            })),
                    ),
            )
            .child(
                div().px_4().pb_4().child(
                    Select::new(&self.language)
                        .small()
                        .w_full()
                        .accessibility_label("Code language")
                        .search_placeholder("Find a language…"),
                ),
            )
            .child(
                div().px_2().flex_1().min_h_0().overflow_hidden().child(
                    Editor::new(&self.editor)
                        .appearance(false)
                        .bordered(false)
                        .h_full()
                        .text_size(px(12.))
                        .aria_label("Source code"),
                ),
            )
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .text_color(rgb(MUTED))
                    .child(self.source_name.clone())
                    .child(format!(
                        "{} lines",
                        self.document.source.split('\n').count()
                    )),
            )
    }
    fn canvas_panel(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self
            .document
            .theme
            .parse::<ImageTheme>()
            .unwrap_or_default();
        let busy = self.revision != self.preview_revision;
        let mut canvas = div()
            .id("preview-canvas")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_scroll()
            .flex()
            .p_8();
        if let Some((image, preview)) = &self.preview {
            let size = window.viewport_size();
            let available_w = (f32::from(size.width) - 320. - 300. - 64.).max(100.);
            let available_h = (f32::from(size.height) - 60. - 52. - 64. - 70.).max(100.);
            let w = preview.width as f32 / 2.;
            let h = preview.height as f32 / 2.;
            let ratio = if self.zoom {
                1.
            } else {
                (available_w / w).min(available_h / h).min(1.)
            };
            let (w, h) = (w * ratio, h * ratio);
            let mut artboard = div().relative().w(px(w)).h(px(h)).flex_shrink_0().m_auto();
            if self.document.transparent {
                artboard = artboard.child(canvas_checker(w, h));
            }
            canvas = canvas.child(
                artboard.child(
                    img(image.clone())
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Contain),
                ),
            );
        } else {
            canvas = canvas.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .text_color(rgb(MUTED))
                    .m_auto()
                    .child(Icon::new(IconName::Image).size(px(28.)))
                    .child(if busy {
                        "Preparing your canvas…"
                    } else {
                        "Your code image appears here"
                    }),
            );
        }
        let dimensions = self
            .preview
            .as_ref()
            .map(|(_, p)| {
                format!(
                    "{} × {} px",
                    p.width / 2 * self.document.scale,
                    p.height / 2 * self.document.scale
                )
            })
            .unwrap_or_else(|| "—".into());
        let status = if busy {
            "Updating preview…".to_owned()
        } else if self.error.is_some() {
            "Preview needs attention".into()
        } else {
            self.preview
                .as_ref()
                .map(|(_, p)| format!("Ready · {:.0} ms", p.elapsed.as_secs_f64() * 1000.))
                .unwrap_or_else(|| "Ready".into())
        };
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(canvas_background())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(52.))
                    .flex_shrink_0()
                    .px_5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(section("Preview"))
                            .child(
                                div()
                                    .text_color(rgb(MUTED))
                                    .text_size(px(12.))
                                    .child(format!("/ {}", theme.name())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                Button::new("fit-preview")
                                    .ghost()
                                    .xsmall()
                                    .label("Fit")
                                    .selected(!self.zoom)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.zoom = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("actual-preview")
                                    .ghost()
                                    .xsmall()
                                    .label("100%")
                                    .selected(self.zoom)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.zoom = true;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(canvas)
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    div()
                        .mx_5()
                        .mb_3()
                        .p_3()
                        .rounded_md()
                        .bg(rgb(0x382627))
                        .text_color(rgb(0xFFB4B4))
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
                        .bg(rgb(0x263241))
                        .text_color(rgb(ACCENT))
                        .text_size(px(12.))
                        .child(notice),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_5()
                    .h(px(38.))
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_size(px(11.))
                    .text_color(rgb(MUTED))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().size(px(5.)).rounded_full().bg(rgb(if busy {
                                0xE4C48C
                            } else {
                                0x8FBFAD
                            })))
                            .child(status),
                    )
                    .child(if self.document.format == Format::Png {
                        dimensions
                    } else {
                        "Vector / CSS dimensions".into()
                    }),
            )
    }
    fn inspector(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = self
            .document
            .theme
            .parse::<ImageTheme>()
            .unwrap_or_default();
        let supports_light = theme.supports(Appearance::Light);
        let numbers = if self.document.language == Language::PLAIN.id() {
            Some(false)
        } else {
            match theme.line_numbers() {
                LineNumberPolicy::Fixed(value) => Some(value),
                _ => None,
            }
        };
        let swatches = self.thumbnails.chunks(3).enumerate().map(|(row, themes)| {
            div().flex().gap_2().children(themes.iter().enumerate().map(
                move |(column, (theme, image))| {
                    let theme = *theme;
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            Button::new(("theme-swatch", row * 3 + column))
                                .ghost()
                                .p_0()
                                .h(px(46.))
                                .w_full()
                                .overflow_hidden()
                                .rounded_md()
                                .border_2()
                                .border_color(rgb(if self.document.theme == theme.id() {
                                    ACCENT
                                } else {
                                    BORDER
                                }))
                                .child(
                                    img(image.clone())
                                        .w_full()
                                        .h_full()
                                        .object_fit(ObjectFit::Cover),
                                )
                                .accessibility_label(format!("{} theme", theme.name()))
                                .tooltip(theme.name())
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.select_theme(theme, window, cx)
                                })),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(if self.document.theme == theme.id() {
                                    TEXT
                                } else {
                                    MUTED
                                }))
                                .child(theme.name()),
                        )
                },
            ))
        });
        div()
            .w(px(300.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .id("style-inspector")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(section("Theme"))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(MUTED))
                                    .child("33 looks"),
                            ),
                    )
                    .child(div().flex().flex_col().gap_2().children(swatches))
                    .child(
                        Select::new(&self.themes)
                            .small()
                            .w_full()
                            .accessibility_label("All themes")
                            .search_placeholder("Explore all themes…"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("dark-appearance")
                                    .small()
                                    .label("Dark")
                                    .icon(IconName::Moon)
                                    .selected(!self.document.light)
                                    .flex_1()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.document.light = false;
                                        this.changed(cx);
                                    })),
                            )
                            .child(
                                Button::new("light-appearance")
                                    .small()
                                    .label("Light")
                                    .icon(IconName::Sun)
                                    .selected(self.document.light)
                                    .disabled(!supports_light)
                                    .flex_1()
                                    .tooltip(if supports_light {
                                        "Use the light variant"
                                    } else {
                                        "This theme supports dark only"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.document.light = true;
                                        this.changed(cx);
                                    })),
                            ),
                    )
                    .child(rule())
                    .child(section("Composition"))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(field(
                                "Width",
                                Input::new(&self.width)
                                    .small()
                                    .aria_label("Image width")
                                    .suffix(
                                        div().text_color(rgb(MUTED)).text_size(px(11.)).child("px"),
                                    ),
                            ))
                            .child(field(
                                "Padding",
                                Input::new(&self.padding)
                                    .small()
                                    .aria_label("Image padding")
                                    .suffix(
                                        div().text_color(rgb(MUTED)).text_size(px(11.)).child("px"),
                                    ),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .children([16u32, 32, 64, 96].into_iter().map(|padding| {
                                Button::new(("padding", padding))
                                    .ghost()
                                    .xsmall()
                                    .flex_1()
                                    .label(padding.to_string())
                                    .selected(self.document.padding == padding.to_string())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.padding.update(cx, |state, cx| {
                                            state.replace_all(padding.to_string(), window, cx)
                                        });
                                    }))
                            })),
                    )
                    .child(row(
                        "Transparent background",
                        Switch::new("transparent-background")
                            .small()
                            .checked(self.document.transparent)
                            .on_change(cx.listener(|this, value, _, cx| {
                                this.document.transparent = *value;
                                this.changed(cx);
                            })),
                    ))
                    .child(row(
                        if numbers.is_some() {
                            "Line numbers · theme"
                        } else {
                            "Line numbers"
                        },
                        Switch::new("line-numbers")
                            .small()
                            .checked(numbers.unwrap_or(self.document.numbers))
                            .disabled(numbers.is_some())
                            .on_change(cx.listener(|this, value, _, cx| {
                                this.document.numbers = *value;
                                this.changed(cx);
                            })),
                    ))
                    .child(field(
                        "Window title",
                        Input::new(&self.title).small().aria_label("Window title"),
                    ))
                    .child(field(
                        "Highlight lines",
                        Input::new(&self.highlights)
                            .small()
                            .aria_label("Highlight lines"),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("custom-font")
                                    .ghost()
                                    .small()
                                    .flex_1()
                                    .label(
                                        self.document
                                            .font_path
                                            .as_ref()
                                            .and_then(|p| p.file_name())
                                            .map(|n| n.to_string_lossy().into_owned())
                                            .unwrap_or_else(|| "Choose a custom font…".into()),
                                    )
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.choose_font(window, cx)
                                    })),
                            )
                            .when(self.document.font_path.is_some(), |view| {
                                view.child(
                                    Button::new("theme-font")
                                        .ghost()
                                        .xsmall()
                                        .label("Reset")
                                        .tooltip("Restore the theme’s font")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.document.font_path = None;
                                            this.changed(cx);
                                        })),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .p_4()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(section("Export"))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(MUTED))
                                    .child("Rendered locally"),
                            ),
                    )
                    .child(
                        div().flex().gap_1().children(
                            [Format::Png, Format::Svg, Format::Html]
                                .into_iter()
                                .map(|format| {
                                    Button::new(format.extension())
                                        .small()
                                        .flex_1()
                                        .label(format.label())
                                        .selected(self.document.format == format)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.document.format = format;
                                            this.changed(cx);
                                        }))
                                }),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .child("PNG scale"),
                            )
                            .child(div().flex().flex_1().gap_1().children(
                                [1, 2, 3, 4, 5, 6].into_iter().map(|scale| {
                                    Button::new(("scale", scale))
                                        .ghost()
                                        .xsmall()
                                        .flex_1()
                                        .label(format!("{scale}×"))
                                        .selected(self.document.scale == scale)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.document.scale = scale;
                                            this.changed(cx);
                                        }))
                                }),
                            )),
                    )
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
fn section(title: &'static str) -> impl IntoElement {
    div()
        .font_weight(FontWeight::SEMIBOLD)
        .text_size(px(12.))
        .child(title)
}
fn rule() -> impl IntoElement {
    div().h(px(1.)).w_full().bg(rgb(BORDER))
}
fn field(label: &'static str, element: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap_2()
        .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(label))
        .child(element)
}
fn row(label: &'static str, element: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .text_size(px(12.))
        .child(label)
        .child(element)
}
fn canvas_checker(width: f32, height: f32) -> impl IntoElement {
    canvas(
        |bounds, _, _| bounds,
        |bounds, _, window, _| {
            let cell = px(12.);
            for y in 0..(f32::from(bounds.size.height) / 12.).ceil() as u32 {
                for x in 0..(f32::from(bounds.size.width) / 12.).ceil() as u32 {
                    let origin = bounds.origin + point(cell * x as f32, cell * y as f32);
                    let size = size(
                        cell.min(bounds.right() - origin.x),
                        cell.min(bounds.bottom() - origin.y),
                    );
                    window.paint_quad(fill(
                        Bounds::new(origin, size),
                        rgb(if (x + y) % 2 == 0 { 0x30343B } else { 0x282C32 }),
                    ));
                }
            }
        },
    )
    .absolute()
    .w(px(width))
    .h(px(height))
}
