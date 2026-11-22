use gpui::{prelude::*, *};
use gpui_base::{
    Root, RootPlugin,
    input::{NativeMenu, NativeMenuItem},
};
use std::{collections::HashMap, rc::Rc};
use tact::{
    binding::BindExt as _,
    change::{Pointer, Source},
    collection::{Item, Items},
    menu::{Menu, MenuChoice, MenuHandle},
};

use crate::appearance::palette;

#[derive(Default)]
struct Menus(HashMap<WindowId, WeakEntity<EditorMenu>>);
impl Global for Menus {}

pub(super) fn init(cx: &mut App) {
    cx.set_global(Menus::default());
    Root::register_plugin(cx, |window, cx| {
        let id = window.window_handle().window_id();
        cx.global_mut::<Menus>()
            .0
            .retain(|_, menu| menu.upgrade().is_some());
        let menu = cx.weak_entity();
        cx.global_mut::<Menus>().0.insert(id, menu);
        EditorMenu {
            items: Rc::new(Vec::new()),
            menu: cx.new(|cx| Menu::new(Items::empty(), window, cx)),
        }
    });
}

pub(super) fn show(menu: NativeMenu, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let id = window.window_handle().window_id();
    let editor_menu = cx.global::<Menus>().0.get(&id).cloned();
    if let Some(editor_menu) = editor_menu {
        let _ = editor_menu.update(cx, |overlay, cx| {
            let items = Items::try_new(menu.items.iter().enumerate().filter_map(
                |(index, entry)| match entry {
                    NativeMenuItem::Separator => None,
                    NativeMenuItem::Action {
                        label, disabled, ..
                    } => Some(Item::new(index, label.as_str()).disabled(*disabled)),
                },
            ))
            .expect("editor menu entries have unique indices");
            overlay.items = Rc::new(menu.items);
            overlay.menu.update(cx, |menu, cx| {
                menu.replace_items(items, cx);
                menu.open_at(position, Source::Pointer(Pointer::Mouse), window, cx);
            });
            cx.notify();
        });
    }
}

struct EditorMenu {
    items: Rc<Vec<NativeMenuItem>>,
    menu: Entity<Menu<usize>>,
}
impl RootPlugin for EditorMenu {}

impl Render for EditorMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let items = self.items.clone();
        let surface = self.menu.surface(window, cx).label("Edit").on_choose(
            move |choice: &MenuChoice<usize>, window, cx| {
                if let NativeMenuItem::Action { action, .. } = &items[*choice.key()] {
                    window.dispatch_action(action.boxed_clone(), cx);
                }
            },
        );
        let rows = surface.items();
        if !surface.is_mounted() {
            return Empty.into_any_element();
        }
        tact::place::at_point(
            surface.anchor(),
            div()
                .id("editor-context-menu")
                .bind(surface)
                .p(px(4.))
                .min_w(px(120.))
                .rounded(px(4.))
                .shadow_md()
                .bg(rgb(p.raised))
                .text_color(rgb(p.ink))
                .children(self.items.iter().enumerate().map(|(index, entry)| {
                    match entry {
                        NativeMenuItem::Separator => div()
                            .h(px(1.))
                            .my(px(3.))
                            .bg(rgb(p.border))
                            .into_any_element(),
                        NativeMenuItem::Action { label, .. } => {
                            let item = rows.item(&index, window, cx);
                            item.div()
                                .text_size(px(11.))
                                .p(px(5.))
                                .rounded(px(3.))
                                .when(item.is_disabled(), |row| row.text_color(rgb(p.muted)))
                                .when(item.is_highlighted(), |row| row.bg(rgb(p.surface3)))
                                .child(label.clone())
                                .into_any_element()
                        }
                    }
                })),
        )
        .into_any_element()
    }
}
