use crate::{appearance::palette, assets::IconName};
use gpui::{prelude::*, *};
use gpui_base::{Selectable, StateStyle};

pub use gpui_base::{
    Collapsible, Scrollbar, StyledExt, TestSupportExt, h_resizable, resizable_panel,
};
#[path = "ui/editor_search.rs"]
pub mod editor_search;
#[path = "ui/picker.rs"]
pub mod picker;
#[path = "ui/surfaces.rs"]
mod surfaces;
pub use surfaces::{DropdownMenu, Popover, PopupMenuItem};

pub fn init(cx: &mut App) {
    gpui_base::init(cx);
    tact::init(cx);
    picker::init(cx);
    gpui_base::apply_system_reduce_motion(cx);
    editor_menu::init(cx);
    editor_search::init(cx);
}

pub fn root(
    view: impl Into<AnyView>,
    window: &mut Window,
    cx: &mut Context<gpui_base::Root>,
) -> gpui_base::Root {
    window.set_rem_size(px(12.));
    gpui_base::Root::new(view, window, cx)
        .font_family("JetBrains Mono")
        .text_size(px(12.))
}

pub fn icon(name: IconName) -> Svg {
    svg().path(name.path()).size(px(16.)).flex_shrink_0()
}

pub struct Tooltip(SharedString);

impl Tooltip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }

    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        gpui_base::Tooltip::new("tooltip")
            .rounded(px(4.))
            .px(px(8.))
            .py(px(5.))
            .bg(rgb(p.raised))
            .text_color(rgb(p.ink))
            .text_size(px(11.))
            .border_1()
            .border_color(rgb(p.border))
            .child(self.0.clone())
    }
}

#[derive(Clone, Copy, Default)]
pub struct ButtonCustomVariant {
    color: Hsla,
    foreground: Option<Hsla>,
    hover: Option<Hsla>,
    active: Option<Hsla>,
}

impl ButtonCustomVariant {
    pub fn new(_: &App) -> Self {
        Self::default()
    }
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }
    pub fn foreground(mut self, color: Hsla) -> Self {
        self.foreground = Some(color);
        self
    }
    pub fn hover(mut self, color: Hsla) -> Self {
        self.hover = Some(color);
        self
    }
    pub fn active(mut self, color: Hsla) -> Self {
        self.active = Some(color);
        self
    }
}

#[derive(Default)]
enum ButtonLook {
    #[default]
    Control,
    Ghost,
    Outline,
    Primary,
    Custom(ButtonCustomVariant),
}

#[derive(IntoElement)]
pub struct Button {
    base: gpui_base::Button,
    id: ElementId,
    size: f32,
    look: ButtonLook,
    label: Option<SharedString>,
    accessibility_name: Option<SharedString>,
    icon: Option<Svg>,
    tooltip: Option<SharedString>,
    selected: bool,
    open: bool,
    decorative: bool,
    toggled: Option<bool>,
    loading: bool,
    focus: Option<FocusHandle>,
    style: StyleRefinement,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: gpui_base::Button::new(id.clone()),
            id,
            size: 32.,
            look: ButtonLook::default(),
            label: None,
            accessibility_name: None,
            icon: None,
            tooltip: None,
            selected: false,
            open: false,
            decorative: false,
            toggled: None,
            loading: false,
            focus: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn xsmall(mut self) -> Self {
        self.size = 20.;
        self
    }
    pub fn small(mut self) -> Self {
        self.size = 24.;
        self
    }
    pub fn ghost(mut self) -> Self {
        self.look = ButtonLook::Ghost;
        self
    }
    pub fn outline(mut self) -> Self {
        self.look = ButtonLook::Outline;
        self
    }
    pub fn primary(mut self) -> Self {
        self.look = ButtonLook::Primary;
        self
    }
    pub fn custom(mut self, look: ButtonCustomVariant) -> Self {
        self.look = ButtonLook::Custom(look);
        self
    }
    pub fn icon(mut self, icon: Svg) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn tooltip(mut self, label: impl Into<SharedString>) -> Self {
        self.tooltip = Some(label.into());
        self
    }
    pub fn toggled(mut self, toggled: bool) -> Self {
        self.toggled = Some(toggled);
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.accessibility_name = Some(label.clone());
        self.base = self.base.accessibility_label(label);
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    pub(super) fn menu_name(&self) -> SharedString {
        self.accessibility_name
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_else(|| "Options".into())
    }
    pub(super) fn decorative(mut self) -> Self {
        self.decorative = true;
        self.base = self.base.focusable(false).role(None::<Role>);
        self
    }
    pub fn role(mut self, role: Role) -> Self {
        self.base = self.base.role(role);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    fn is_selected(&self) -> bool {
        self.selected || self.open
    }
    fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
    fn is_open(&self) -> bool {
        self.open
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
    fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
}
impl StatefulInteractiveElement for Button {}
impl ParentElement for Button {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let hover: Hsla = rgb(p.control_hover).into();
        let pressed: Hsla = rgb(p.control_on).into();
        let text: Hsla = rgb(p.ink2).into();
        let (background, foreground, hover, pressed) = match self.look {
            ButtonLook::Control | ButtonLook::Outline => {
                (rgb(p.control).into(), text, hover, pressed)
            }
            ButtonLook::Ghost => (transparent_black(), text, hover, pressed),
            ButtonLook::Primary => (
                rgb(p.ink).into(),
                rgb(p.raised).into(),
                rgb(if p.light { 0 } else { 0xFFFFFF }).into(),
                pressed,
            ),
            ButtonLook::Custom(style) => (
                style.color,
                style.foreground.unwrap_or(text),
                style.hover.unwrap_or(hover),
                style.active.unwrap_or(pressed),
            ),
        };
        let focus = self.focus.unwrap_or_else(|| {
            window
                .use_keyed_state((self.id, "button-focus"), cx, |_, cx| {
                    cx.focus_handle().tab_stop(true)
                })
                .read(cx)
                .clone()
        });
        let selected = self.selected || self.open;
        let background = if selected && !matches!(self.look, ButtonLook::Custom(_)) {
            pressed
        } else {
            background
        };
        let label = self.label;
        self.base
            .when(self.loading, |button| button.disabled(true))
            .track_focus(&focus)
            .when(!self.decorative, |button| {
                button.on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    gpui_base::GlobalState::suppress_text_selection(cx);
                })
            })
            .selected(selected)
            .when_some(self.toggled, |button, toggled| {
                button.aria_toggled(if toggled {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .h(px(self.size))
            .px(px(if self.size <= 24. { 5. } else { 9. }))
            .rounded(px(4.))
            .text_size(px(if self.size <= 24. { 11. } else { 12. }))
            .bg(background)
            .text_color(foreground)
            .when(matches!(self.look, ButtonLook::Outline), |button| {
                button.border_1().border_color(rgb(p.strong))
            })
            .hover(|style| style.bg(hover))
            .active(|style| style.bg(pressed))
            .focus_visible(|style| style.border_1().border_color(rgb(p.accent)))
            .styles(|styles| styles.disabled(|style: StateStyle| style.opacity(0.5)))
            .refine_style(&self.style)
            .when_some(self.tooltip, |button, label| {
                button.tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
            })
            .children(self.icon)
            .children(label)
    }
}

pub fn switch(id: &'static str, checked: bool, p: crate::appearance::Palette) -> gpui_base::Switch {
    gpui_base::Switch::new(id)
        .checked(checked)
        .w(px(27.))
        .h(px(15.))
        .p(px(2.))
        .rounded_full()
        .flex()
        .items_center()
        .when(checked, |view| view.justify_end())
        .bg(rgb(p.strong))
        .styles(|styles| styles.checked(|style| style.bg(rgb(p.accent))))
        .child(
            gpui_base::SwitchThumb::new(checked)
                .size(px(11.))
                .rounded_full()
                .bg(white()),
        )
}

pub fn switch_listener(
    handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static {
    move |checked, _, window, cx| handler(&checked, window, cx)
}

#[path = "ui/editor_menu.rs"]
mod editor_menu;

pub use editor_menu::install as install_editor_menu;

#[cfg(target_os = "linux")]
#[derive(IntoElement)]
pub struct TitleBar {
    base: Div,
    close: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

#[cfg(target_os = "linux")]
impl TitleBar {
    pub fn new() -> Self {
        Self {
            base: div(),
            close: None,
        }
    }
    pub fn on_close_window(
        mut self,
        close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.close = Some(Box::new(close));
        self
    }
}

#[cfg(target_os = "linux")]
impl Styled for TitleBar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

#[cfg(target_os = "linux")]
impl ParentElement for TitleBar {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}

#[cfg(target_os = "linux")]
impl RenderOnce for TitleBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .flex()
            .items_center()
            .h(px(32.))
            .px(px(6.))
            .border_b_1()
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                Button::new("minimize-window")
                    .ghost()
                    .small()
                    .label("−")
                    .accessibility_label("Minimize")
                    .on_click(|_, window, _| window.minimize_window()),
            )
            .child(
                Button::new("maximize-window")
                    .ghost()
                    .small()
                    .label("□")
                    .accessibility_label("Maximize")
                    .on_click(|_, window, _| window.zoom_window()),
            )
            .child(
                Button::new("close-window")
                    .ghost()
                    .small()
                    .label("×")
                    .accessibility_label("Close")
                    .when_some(self.close, |button, close| button.on_click(close)),
            )
    }
}
