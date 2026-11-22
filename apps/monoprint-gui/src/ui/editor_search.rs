use super::{Button, icon, install_editor_menu, palette};
use crate::assets::IconName;
use gpui::{prelude::*, *};
use gpui_base::{
    Selectable,
    input::{self, EditorState, InputEvent, InputState},
};
use tact_base::TextField;

#[derive(Default, PartialEq, Eq)]
struct SearchStatus {
    open: bool,
    replace_mode: bool,
    case_insensitive: bool,
    current: Option<usize>,
    matches: usize,
    replaceable: bool,
}

impl SearchStatus {
    fn read(state: &EditorState) -> Self {
        let search = state.search_session();
        Self {
            open: search.open,
            replace_mode: search.replace_mode,
            case_insensitive: search.case_insensitive,
            current: search.matcher.current(),
            matches: search.matcher.len(),
            replaceable: state.is_replaceable(),
        }
    }
}

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "escape",
        input::Escape,
        Some("EditorSearch"),
    )]);
}

pub struct EditorSearch {
    editor: Entity<EditorState>,
    query: Entity<InputState>,
    replacement: Entity<InputState>,
    activation: u64,
    status: SearchStatus,
    _editor: Subscription,
    _query: Subscription,
}

impl EditorSearch {
    pub fn new(editor: Entity<EditorState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut input = |placeholder| {
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder);
                install_editor_menu(&mut input);
                input
            })
        };
        let query = input("Find in source…");
        let replacement = input("Replace with…");
        let editor_observer = cx.observe_in(&editor, window, |this, editor, window, cx| {
            let state = editor.read(cx);
            let activation = state.search_activation_revision();
            let status = SearchStatus::read(state);
            let query = (status.open && activation != this.activation)
                .then(|| state.search_session().query.clone());
            if status != this.status {
                this.status = status;
                cx.notify();
            }
            if let Some(text) = query {
                this.activation = activation;
                this.query.update(cx, |query, cx| {
                    if query.value().as_str() != text {
                        query.replace_all(text, window, cx);
                    }
                    query.focus(window, cx);
                    query.select_all(window, cx);
                });
            }
        });
        let query_observer = cx.subscribe(&query, |this, query, event, cx| {
            if matches!(event, InputEvent::Change) {
                let value = query.read(cx).value().to_string();
                this.editor.update(cx, |editor, cx| {
                    let session = editor.search_session();
                    if session.open && session.query != value {
                        editor.set_search_query(value, session.case_insensitive, cx);
                    }
                });
            }
        });
        Self {
            editor,
            query,
            replacement,
            activation: 0,
            status: SearchStatus::default(),
            _editor: editor_observer,
            _query: query_observer,
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| {
            editor.close_search(cx);
            editor.focus(window, cx);
        });
    }

    fn navigate(&mut self, previous: bool, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| {
            if previous {
                editor.previous_search_match(cx);
            } else {
                editor.next_search_match(cx);
            }
            cx.notify();
        });
    }

    fn replace(&mut self, all: bool, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.replacement.read(cx).value();
        self.editor.update(cx, |editor, cx| {
            if all {
                editor.replace_all_search_matches(&text, window, cx);
            } else {
                editor.replace_current_search_match(&text, window, cx);
            }
        });
    }

    fn field(
        &self,
        id: &'static str,
        state: &Entity<InputState>,
        label: &'static str,
        cx: &Context<Self>,
    ) -> TextField {
        let search = cx.weak_entity();
        TextField::new(id, state)
            .label(label)
            .flex_1()
            .min_w_0()
            .text_size(px(11.))
            .input_row(|row| row.h(px(24.)).px(px(6.)))
            .bg(rgb(palette(cx).control))
            .on_key(move |command, window, cx| {
                if command == tact::editor::Command::Escape {
                    search
                        .update(cx, |search, cx| search.close(window, cx))
                        .is_ok()
                } else {
                    false
                }
            })
    }
}

impl Render for EditorSearch {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = &self.status;
        if !status.open {
            return Empty.into_any_element();
        }
        let replace_mode = status.replace_mode;
        let case_insensitive = status.case_insensitive;
        let match_count = format!(
            "{}/{}",
            status.current.map_or(0, |index| index + 1),
            status.matches
        );
        let empty = status.matches == 0;
        let replaceable = status.replaceable;
        let p = palette(cx);
        div()
            .id("source-find-panel")
            .key_context("EditorSearch")
            .on_action(cx.listener(|this, _: &input::Search, _, cx| {
                this.editor
                    .update(cx, |editor, cx| editor.open_search(false, cx));
            }))
            .on_action(cx.listener(|this, _: &input::Replace, _, cx| {
                this.editor
                    .update(cx, |editor, cx| editor.open_search(true, cx));
            }))
            .on_action(cx.listener(|this, _: &input::Escape, window, cx| {
                if this.query.focus_handle(cx).is_focused(window)
                    || this.replacement.focus_handle(cx).is_focused(window)
                {
                    cx.propagate();
                } else {
                    this.close(window, cx);
                }
            }))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(px(4.))
            .p(px(6.))
            .bg(rgb(p.surface))
            .border_b_1()
            .border_color(rgb(p.border))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        self.field("source-find", &self.query, "Find in source", cx)
                            .on_submit(cx.listener(|this, _, _, cx| this.navigate(false, cx))),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child(match_count),
                    )
                    .child(
                        Button::new("source-find-case")
                            .ghost()
                            .xsmall()
                            .label("Aa")
                            .toggled(!case_insensitive)
                            .selected(!case_insensitive)
                            .accessibility_label("Match case")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.editor.update(cx, |editor, cx| {
                                    let session = editor.search_session();
                                    editor.set_search_query(
                                        session.query.clone(),
                                        !session.case_insensitive,
                                        cx,
                                    );
                                });
                            })),
                    )
                    .child(
                        Button::new("source-find-prev")
                            .ghost()
                            .xsmall()
                            .icon(
                                icon(IconName::ChevronDown)
                                    .size(px(13.))
                                    .with_transformation(Transformation::rotate(radians(
                                        std::f32::consts::PI,
                                    ))),
                            )
                            .accessibility_label("Previous match")
                            .disabled(empty)
                            .on_click(cx.listener(|this, _, _, cx| this.navigate(true, cx))),
                    )
                    .child(
                        Button::new("source-find-next")
                            .ghost()
                            .xsmall()
                            .icon(icon(IconName::ChevronDown).size(px(13.)))
                            .accessibility_label("Next match")
                            .disabled(empty)
                            .on_click(cx.listener(|this, _, _, cx| this.navigate(false, cx))),
                    )
                    .when(replaceable, |row| {
                        row.child(
                            Button::new("source-find-show-replace")
                                .ghost()
                                .xsmall()
                                .label("Replace")
                                .toggled(replace_mode)
                                .selected(replace_mode)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.editor.update(cx, |editor, cx| {
                                        editor.set_search_replace_mode(!replace_mode, cx)
                                    });
                                })),
                        )
                    })
                    .child(
                        Button::new("source-find-close")
                            .ghost()
                            .xsmall()
                            .icon(icon(IconName::X).size(px(13.)))
                            .accessibility_label("Close find")
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    ),
            )
            .when(replace_mode, |view| {
                view.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            self.field("source-replacement", &self.replacement, "Replace with", cx)
                                .on_submit(cx.listener(|this, _, window, cx| {
                                    this.replace(false, window, cx)
                                })),
                        )
                        .child(
                            Button::new("source-replace")
                                .xsmall()
                                .label("Replace")
                                .disabled(empty)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace(false, window, cx)
                                })),
                        )
                        .child(
                            Button::new("source-replace-all")
                                .xsmall()
                                .label("Replace all")
                                .disabled(empty)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace(true, window, cx)
                                })),
                        ),
                )
            })
            .into_any_element()
    }
}
