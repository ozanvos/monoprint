use super::*;
use gpui_base::PopoverState;
use std::{rc::Rc, time::Duration};
use tact::{
    binding::BindExt as _,
    collection::{Item, Items},
    menu::{Menu, MenuHandle},
};

fn panel(p: crate::appearance::Palette) -> Div {
    div()
        .rounded(px(4.))
        .bg(rgb(p.raised))
        .text_color(rgb(p.ink))
}

type PopoverContent =
    Box<dyn FnOnce(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement>;

#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    anchor: Anchor,
    style: StyleRefinement,
    trigger: Option<Box<dyn FnOnce(bool) -> AnyElement>>,
    content: Option<PopoverContent>,
    children: Vec<AnyElement>,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            anchor: Anchor::TopLeft,
            style: StyleRefinement::default(),
            trigger: None,
            content: None,
            children: Vec::new(),
        }
    }
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.style = style;
        self
    }
    pub fn trigger(mut self, trigger: impl IntoElement + Selectable + 'static) -> Self {
        self.trigger = Some(Box::new(|open| trigger.open(open).into_any_element()));
        self
    }
    pub fn content<E: IntoElement>(
        mut self,
        builder: impl FnOnce(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    ) -> Self {
        self.content = Some(Box::new(|state, window, cx| {
            builder(state, window, cx).into_any_element()
        }));
        self
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for Popover {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let trigger = self.trigger.expect("a Studio popover has a trigger");
        gpui_base::Popover::new(self.id)
            .anchor(self.anchor)
            .offset(px(6.))
            .refine_style(&self.style)
            .trigger_with(move |open, _, _| trigger(open))
            .content(move |state, window, cx| {
                let body = self.content.map(|content| content(state, window, cx));
                panel(p).shadow_md().children(body).children(self.children)
            })
    }
}

pub(super) fn dropdown(
    id: impl Into<ElementId>,
    surface: impl IntoElement + Styled + ParentElement + 'static,
    p: crate::appearance::Palette,
) -> impl IntoElement {
    let radius = px(4.);
    let ring: Hsla = rgb(p.ink).opacity(0.1).into();
    surface
        .relative()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| paint_popup_shadow(bounds, radius, ring, window),
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
        .with_animation(
            id,
            Animation::new(Duration::from_millis(150)).with_easing(|t| 1. - (1. - t).powi(3)),
            |surface, progress| surface.top(px(-8. * (1. - progress))).opacity(progress),
        )
}
// A rectangular content mask excludes the panel's middle. Pixel-height strips
// follow the four rounded corners, preserving the resting shadow shape without
// painting its ink underneath the fading panel. Masks are disjoint.
fn paint_popup_shadow(bounds: Bounds<Pixels>, radius: Pixels, ring: Hsla, window: &mut Window) {
    let mask = window.content_mask().bounds;
    let radius = radius
        .min(bounds.size.width / 2.)
        .min(bounds.size.height / 2.);
    let ink = hsla(0., 0., 0., 0.1);
    let shadows = [
        BoxShadow::new(px(0.), px(0.), ring).spread_radius(px(1.)),
        BoxShadow::new(px(0.), px(4.), ink)
            .blur_radius(px(3.))
            .spread_radius(px(-1.)),
        BoxShadow::new(px(0.), px(2.), ink)
            .blur_radius(px(2.))
            .spread_radius(px(-2.)),
    ];
    let corners = Corners::all(radius);
    let step = 1. / window.scale_factor();
    let mut paint_band = |band: Bounds<Pixels>| {
        if band.size.width <= px(0.) || band.size.height <= px(0.) {
            return;
        }
        window.with_content_mask(Some(ContentMask { bounds: band }), |window| {
            window.paint_drop_shadows(bounds, corners, &shadows);
        });
    };
    paint_band(Bounds::from_corners(
        mask.origin,
        point(mask.right(), bounds.top()),
    ));
    paint_band(Bounds::from_corners(
        point(mask.left(), bounds.bottom()),
        mask.bottom_right(),
    ));
    paint_band(Bounds::from_corners(
        point(mask.left(), bounds.top()),
        point(bounds.left(), bounds.bottom()),
    ));
    paint_band(Bounds::from_corners(
        point(bounds.right(), bounds.top()),
        point(mask.right(), bounds.bottom()),
    ));
    let radius = f32::from(radius);
    let mut y = 0.;
    while y < radius {
        let height = step.min(radius - y);
        let inset = radius
            - (radius * radius - (radius - y - height / 2.).powi(2))
                .max(0.)
                .sqrt();
        let width = px(inset);
        for top in [bounds.top() + px(y), bounds.bottom() - px(y + height)] {
            for left in [bounds.left(), bounds.right() - width] {
                paint_band(Bounds::new(point(left, top), size(width, px(height))));
            }
        }
        y += height;
    }
}

type MenuAction = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct PopupMenuItem {
    text: SharedString,
    checked: bool,
    disabled: bool,
    action: Option<MenuAction>,
}

impl PopupMenuItem {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            checked: false,
            disabled: false,
            action: None,
        }
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub fn on_activate(mut self, action: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

#[derive(Default)]
pub struct PopupMenu {
    rows: Vec<Option<PopupMenuItem>>,
    #[cfg(target_os = "linux")]
    context: Option<FocusHandle>,
}

impl PopupMenu {
    pub fn item(mut self, item: PopupMenuItem) -> Self {
        self.rows.push(Some(item));
        self
    }
    #[cfg(target_os = "linux")]
    pub fn separator(mut self) -> Self {
        self.rows.push(None);
        self
    }
    #[cfg(target_os = "linux")]
    pub fn action_context(mut self, focus: FocusHandle) -> Self {
        self.context = Some(focus);
        self
    }
    #[cfg(target_os = "linux")]
    pub fn menu(self, text: impl Into<SharedString>, action: Box<dyn Action>) -> Self {
        self.menu_with_disabled(text, action, false)
    }
    #[cfg(target_os = "linux")]
    pub fn menu_with_disabled(
        self,
        text: impl Into<SharedString>,
        action: Box<dyn Action>,
        disabled: bool,
    ) -> Self {
        self.menu_with_check_and_disabled(text, false, action, disabled)
    }
    #[cfg(target_os = "linux")]
    pub fn menu_with_check_and_disabled(
        self,
        text: impl Into<SharedString>,
        checked: bool,
        action: Box<dyn Action>,
        disabled: bool,
    ) -> Self {
        let mut item = PopupMenuItem::new(text).checked(checked);
        item.disabled = disabled;
        let focus = self.context.clone();
        item.action = Some(Rc::new(move |window, cx| {
            if let Some(focus) = &focus {
                focus.focus(window, cx);
            }
            window.dispatch_action(action.boxed_clone(), cx);
        }));
        self.item(item)
    }
}

type BuildMenu = Box<dyn FnOnce(PopupMenu, &mut Window, &mut App) -> PopupMenu>;

#[derive(IntoElement)]
pub struct ActionMenu<B: DropdownMenu> {
    id: ElementId,
    anchor: Anchor,
    button: B,
    build: BuildMenu,
}

impl<B: DropdownMenu> RenderOnce for ActionMenu<B> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let rows = Rc::new((self.build)(PopupMenu::default(), window, cx).rows);
        let items = Items::try_new(rows.iter().enumerate().filter_map(|(index, row)| {
            row.as_ref()
                .map(|row| Item::new(index, row.text.as_str()).disabled(row.disabled))
        }))
        .expect("each menu item has a unique index");
        let initial = items.clone();
        let behavior = tact::keyed((self.id.clone(), "menu"), window, cx, move |window, cx| {
            Menu::new(initial, window, cx)
        });
        if !behavior.read(cx).items().iter().eq(items.iter()) {
            behavior.update(cx, |menu, cx| menu.replace_items(items, cx));
        }
        let label = self.button.menu_name();
        let trigger = behavior.trigger(label.clone(), window, cx);
        let open = trigger.is_open();
        let show_focus = trigger.is_focus_visible();
        let invoke = rows.clone();
        let surface = behavior.surface(window, cx).label(label).on_choose(
            move |choice: &tact::menu::MenuChoice<usize>, window, cx| {
                if let Some(action) = invoke[*choice.key()]
                    .as_ref()
                    .and_then(|row| row.action.as_ref())
                {
                    action(window, cx);
                }
            },
        );
        let mounted = surface.is_mounted();
        let item_bindings = surface.items();
        let panel = div()
            .id("studio-menu")
            .bind(surface)
            .rounded(px(4.))
            .bg(rgb(p.raised))
            .text_color(rgb(p.ink))
            .shadow_md()
            .min_w(px(126.))
            .p(px(4.))
            .children(rows.iter().enumerate().map(|(index, row)| {
                let Some(row) = row else {
                    return div()
                        .h(px(1.))
                        .my(px(3.))
                        .bg(rgb(p.border))
                        .into_any_element();
                };
                let item = item_bindings.item(&index, window, cx);
                let highlighted = item.is_highlighted();
                let item = if row.checked {
                    item.checkbox(true)
                } else {
                    item
                };
                item.div()
                    .flex()
                    .items_center()
                    .h(px(25.))
                    .gap(px(7.))
                    .px(px(7.))
                    .rounded(px(3.))
                    .text_size(px(11.))
                    .text_color(rgb(if row.disabled { p.muted } else { p.ink }))
                    .when(highlighted, |row| row.bg(rgb(p.surface3)))
                    .child(div().w(px(13.)).when(row.checked, |slot| {
                        slot.child(icon(IconName::Check).size(px(12.)))
                    }))
                    .child(row.text.clone())
                    .into_any_element()
            }));
        div()
            .relative()
            .child(
                div()
                    .id((self.id, "menu-trigger"))
                    .bind(trigger)
                    .relative()
                    .child(self.button.open(open).into_menu_decoration())
                    .when(show_focus, |view| {
                        view.child(
                            div()
                                .absolute()
                                .inset_0()
                                .rounded(px(4.))
                                .border_1()
                                .border_color(rgb(p.accent)),
                        )
                    }),
            )
            .when(mounted, |view| {
                view.child(tact::place::below(panel).gap(px(6.)).align(
                    if matches!(self.anchor, Anchor::TopRight | Anchor::BottomRight) {
                        tact::place::Align::End
                    } else {
                        tact::place::Align::Start
                    },
                ))
            })
    }
}

pub trait DropdownMenu: Sized + Selectable + IntoElement + InteractiveElement + 'static {
    fn into_menu_decoration(self) -> Self;
    fn menu_name(&self) -> SharedString;

    #[cfg(target_os = "linux")]
    fn dropdown_menu(
        self,
        build: impl FnOnce(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static,
    ) -> ActionMenu<Self> {
        self.dropdown_menu_with_anchor(Anchor::TopLeft, build)
    }

    fn dropdown_menu_with_anchor(
        mut self,
        anchor: Anchor,
        build: impl FnOnce(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static,
    ) -> ActionMenu<Self> {
        let id = self
            .interactivity()
            .element_id
            .clone()
            .expect("menu button ID");
        ActionMenu {
            id,
            anchor,
            button: self,
            build: Box::new(build),
        }
    }
}

impl DropdownMenu for Button {
    fn menu_name(&self) -> SharedString {
        Button::menu_name(self)
    }
    fn into_menu_decoration(self) -> Self {
        self.decorative()
    }
}

impl DropdownMenu for crate::interaction_motion::MotionButton {
    fn menu_name(&self) -> SharedString {
        crate::interaction_motion::MotionButton::menu_name(self)
    }
    fn into_menu_decoration(self) -> Self {
        self.decorative()
    }
}
