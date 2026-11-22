//! Studio's searchable language and typeface pickers.
use super::*;
use crate::studio::Choice;
use gpui_base::{
    Popup,
    input::{InputEvent, InputState},
};
use std::{cell::Cell, rc::Rc};
use tact::{
    binding::BindExt as _,
    change::{Choice as ListChoice, Source},
    collection::{Item, Items},
    editor::Command,
    listbox::{Listbox, ListboxHandle},
};
use tact_base::TextField;

gpui::actions!(monoprint_picker, [OpenPicker]);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", OpenPicker, Some("MonoprintPicker")),
        KeyBinding::new("up", OpenPicker, Some("MonoprintPicker")),
    ]);
}

pub trait PickerSource: 'static {
    fn options(&self) -> &[Choice];
    fn selected(&self, id: &str) -> Option<&Choice>;
    fn search(&mut self, query: &str);
}

pub struct ChoiceCatalog {
    all: Vec<Choice>,
    visible: Vec<Choice>,
}

impl ChoiceCatalog {
    pub fn new(all: Vec<Choice>) -> Self {
        Self {
            visible: all.clone(),
            all,
        }
    }

    pub fn replace(&mut self, items: Vec<Choice>) {
        *self = Self::new(items);
    }
}

impl PickerSource for ChoiceCatalog {
    fn options(&self) -> &[Choice] {
        &self.visible
    }
    fn selected(&self, id: &str) -> Option<&Choice> {
        self.all.iter().find(|option| option.id == id)
    }
    fn search(&mut self, query: &str) {
        self.visible = self
            .all
            .iter()
            .filter(|choice| choice.matches(query))
            .cloned()
            .collect();
    }
}

#[derive(Clone)]
pub struct PickerSelection(pub String);

pub struct Picker<D: PickerSource> {
    source: D,
    selected: String,
    open: bool,
    query: Entity<InputState>,
    list: Entity<Listbox<String>>,
    trigger: FocusHandle,
    scroll: UniformListScrollHandle,
    _query: Subscription,
    _choose: Subscription,
}

impl<D: PickerSource> EventEmitter<PickerSelection> for Picker<D> {}

impl<D: PickerSource> Focusable for Picker<D> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.trigger.clone()
    }
}

impl<D: PickerSource> Picker<D> {
    pub fn new(source: D, selected: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            super::install_editor_menu(&mut input);
            input
        });
        let input_focus = query.focus_handle(cx);
        let scroll = UniformListScrollHandle::new();
        let list = cx.new(|cx| {
            Listbox::new(Self::items(&source), window, cx)
                .field_focus(input_focus)
                .scroll_handle(&scroll)
        });
        let on_query = cx.subscribe_in(&query, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                let query = state.read(cx).value();
                this.source.search(&query);
                this.reset_results(cx);
            }
        });
        let on_choose = cx.subscribe_in(
            &list,
            window,
            |this, _, choice: &tact::listbox::ListboxEvent<String>, window, cx| {
                if let tact::listbox::ListboxEvent::Choose(choice) = choice {
                    this.choose(choice, window, cx);
                }
            },
        );
        Self {
            source,
            selected,
            open: false,
            query,
            list,
            trigger: cx.focus_handle().tab_stop(true),
            scroll,
            _query: on_query,
            _choose: on_choose,
        }
    }

    pub fn source_mut(&mut self) -> &mut D {
        &mut self.source
    }

    pub fn set_selected_value(&mut self, id: &str, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = id.to_owned();
        self.reset_results(cx);
    }

    fn items(source: &D) -> Items<String> {
        Items::try_new(
            source
                .options()
                .iter()
                .map(|choice| Item::new(choice.id.clone(), choice.name.as_str())),
        )
        .expect("Studio picker choices have unique IDs")
    }

    fn reset_results(&mut self, cx: &mut Context<Self>) {
        let items = Self::items(&self.source);
        let initial = items.iter().next().map(|item| item.key().clone());
        self.list.update(cx, |list, cx| {
            list.replace_items(items, cx);
            if let Some(initial) = initial {
                list.set_highlighted(&initial, Source::Programmatic, cx);
            }
        });
        self.scroll.scroll_to_item_strict(0, ScrollStrategy::Top);
        cx.notify();
    }

    fn choose(&mut self, choice: &ListChoice<String>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.selected = choice.key().clone();
        self.dismiss(window, cx);
        cx.emit(PickerSelection(self.selected.clone()));
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            self.trigger.focus(window, cx);
            cx.notify();
        }
    }

    fn open(&mut self, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.dismiss(window, cx);
            return;
        }
        self.open = true;
        self.source.search("");
        self.reset_results(cx);
        self.list.update(cx, |list, cx| {
            list.set_highlighted(&self.selected, Source::Programmatic, cx);
        });
        self.query.update(cx, |query, cx| {
            query.set_placeholder(placeholder, window, cx);
            query.replace_all("", window, cx);
            query.focus(window, cx);
        });
        cx.notify();
    }
}

#[derive(IntoElement)]
pub struct PickerView<D: PickerSource> {
    id: &'static str,
    state: Entity<Picker<D>>,
    label: &'static str,
    placeholder: &'static str,
}

impl<D: PickerSource> PickerView<D> {
    pub fn new(
        id: &'static str,
        state: &Entity<Picker<D>>,
        label: &'static str,
        placeholder: &'static str,
    ) -> Self {
        Self {
            id,
            state: state.clone(),
            label,
            placeholder,
        }
    }
}

impl<D: PickerSource> RenderOnce for PickerView<D> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let picker = self.state.read(cx);
        let selected = picker
            .source
            .selected(&picker.selected)
            .map(|choice| choice.name.clone())
            .unwrap_or_default();
        let placeholder = self.placeholder;
        let trigger = Button::new(self.id)
            .outline()
            .xsmall()
            .h(px(26.))
            .w_full()
            .px(px(7.))
            .justify_between()
            .text_size(px(11.))
            .text_color(rgb(p.ink))
            .track_focus(&picker.trigger)
            .role(Role::ComboBox)
            .aria_expanded(picker.open)
            .aria_value(selected.clone())
            .accessibility_label(self.label)
            .key_context("MonoprintPicker")
            .on_action(window.listener_for(
                &self.state,
                move |picker, _: &OpenPicker, window, cx| {
                    if !picker.open {
                        picker.open(placeholder, window, cx);
                    }
                },
            ))
            .child(div().min_w_0().flex_1().truncate().child(selected))
            .child(
                icon(IconName::ChevronDown)
                    .size(px(13.))
                    .text_color(rgb(p.muted)),
            )
            .on_click(
                window.listener_for(&self.state, move |picker, _, window, cx| {
                    picker.open(placeholder, window, cx)
                }),
            );
        let trigger_bounds = Rc::new(Cell::new(Bounds::default()));
        let capture = trigger_bounds.clone();
        let popup = Popup::new((ElementId::from(self.id), "popup"), trigger)
            .on_position(move |_, bounds| capture.set(bounds))
            .offset(px(6.))
            .w_full();
        if !picker.open {
            return popup;
        }

        let field = picker.list.field(self.label, cx);
        let options = picker.source.options();
        let row_height = if options
            .first()
            .is_some_and(|choice| choice.preview.is_some())
        {
            42.
        } else {
            26.
        };
        let count = options.len();
        let rows = field.items();
        let state = self.state.clone();
        let list = div()
            .id("picker-options")
            .bind(picker.list.options(self.label))
            .h(px((count as f32 * row_height).min(240.)))
            .child(
                uniform_list("picker-scroll", count, move |range, window, cx| {
                    let picker = state.read(cx);
                    range
                        .filter_map(|index| picker.source.options().get(index))
                        .map(|choice| {
                            let item = rows.item(&choice.id, window, cx);
                            div()
                                .id((
                                    ElementId::from("picker-option"),
                                    SharedString::from(choice.id.clone()),
                                ))
                                .bind(&item)
                                .h(px(row_height))
                                .flex()
                                .items_center()
                                .px(px(8.))
                                .gap(px(6.))
                                .rounded(px(3.))
                                .text_size(px(11.))
                                .when(item.is_highlighted(), |row| row.bg(rgb(p.surface3)))
                                .hover(|style| style.bg(rgb(p.surface)))
                                .child(div().flex_1().min_w_0().child(choice.menu_item(cx)))
                                .when(choice.id == picker.selected, |row| {
                                    row.child(icon(IconName::Check).size(px(12.)))
                                })
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&picker.scroll),
            );
        let dismiss = self.state.clone();
        let search = TextField::new("picker-search", &picker.query)
            .label(self.label)
            .listbox(&picker.list)
            .on_key(move |command, window, cx| {
                if command == Command::Escape {
                    dismiss.update(cx, |picker, cx| picker.dismiss(window, cx));
                    true
                } else {
                    false
                }
            })
            .input_row(|row| row.h(px(26.)).px(px(7.)))
            .bg(rgb(p.control))
            .bind(&field);
        let surface = div()
            .id("picker-surface")
            .occlude()
            .w(px(240.))
            .p(px(4.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .rounded(px(4.))
            .bg(rgb(p.raised))
            .text_color(rgb(p.ink))
            .on_mouse_down_out(window.listener_for(
                &self.state,
                move |picker, event: &MouseDownEvent, window, cx| {
                    if !trigger_bounds.get().contains(&event.position) {
                        picker.dismiss(window, cx);
                    }
                },
            ))
            .child(search)
            .child(list)
            .when(count == 0, |menu| {
                menu.child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(11.))
                        .text_color(rgb(p.muted))
                        .child("No results"),
                )
            });
        popup.content(super::surfaces::dropdown(self.id, surface, p))
    }
}
