//! Short interaction transitions without changing control semantics or layout.
use crate::appearance::Palette;
use gpui_kit::{
    IntoElement,
    base::motion::{self, Transition},
    component::{
        Selectable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
    },
    prelude::*,
    *,
};
use std::{cell::Cell, rc::Rc, time::Duration};

pub const HOVER_DURATION: Duration = Duration::from_millis(100);
pub const SELECTION_DURATION: Duration = Duration::from_millis(160);

#[derive(Clone, Copy)]
pub enum HoverStyle {
    Control,
    Ghost,
    Primary,
    Overlay,
}

pub trait SmoothHover {
    fn smooth_hover(self, palette: Palette, style: HoverStyle) -> MotionButton;
}

impl SmoothHover for Button {
    fn smooth_hover(mut self, palette: Palette, style: HoverStyle) -> MotionButton {
        let id = self.interactivity().element_id.clone().expect("button ID");
        MotionButton {
            id,
            button: self,
            palette,
            style,
            shared_selection: false,
        }
    }
}

#[derive(IntoElement)]
pub struct MotionButton {
    id: ElementId,
    button: Button,
    palette: Palette,
    style: HoverStyle,
    shared_selection: bool,
}

impl Selectable for MotionButton {
    fn selected(mut self, selected: bool) -> Self {
        self.button = self.button.selected(selected);
        self
    }
    fn is_selected(&self) -> bool {
        self.button.is_selected()
    }
}

impl Styled for MotionButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}

impl InteractiveElement for MotionButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.button.interactivity()
    }
}

impl gpui_kit::component::menu::DropdownMenu for MotionButton {}

impl RenderOnce for MotionButton {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = self.palette;
        let hovered = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| false);
        let selected = self.button.is_selected();
        let transparent = rgb(p.control).opacity(0.).into();
        let fallback = match self.style {
            HoverStyle::Control => rgb(p.control).into(),
            HoverStyle::Primary => rgb(p.ink).into(),
            HoverStyle::Ghost | HoverStyle::Overlay => transparent,
        };
        let normal = if self.shared_selection {
            transparent
        } else {
            self.button
                .style()
                .background
                .as_ref()
                .and_then(|bg| bg.color().and_then(|bg| bg.as_solid()))
                .unwrap_or(if selected {
                    rgb(p.control_on).into()
                } else {
                    fallback
                })
        };
        let hover = match self.style {
            HoverStyle::Primary => rgb(if p.light { 0 } else { 0xFFFFFF }).into(),
            HoverStyle::Ghost => rgb(p.surface3)
                .opacity(if p.light { 1. } else { 0.5 })
                .into(),
            HoverStyle::Overlay => rgb(0x7F7F7F).opacity(0.22).into(),
            HoverStyle::Control => rgb(p.control_hover).into(),
        };
        let target = if *hovered.read(cx) && !(self.shared_selection && selected) {
            hover
        } else {
            normal
        };
        let color = motion::transition(
            (self.id, "hover-color"),
            target,
            Transition::new(HOVER_DURATION),
            window,
            cx,
        );
        let foreground = self.button.style().text.color.unwrap_or_else(|| {
            rgb(match self.style {
                HoverStyle::Primary => p.raised,
                HoverStyle::Overlay => p.overlay_ink,
                _ => p.ink2,
            })
            .into()
        });
        let child = self
            .button
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(color)
                    .hover(color)
                    .foreground(foreground)
                    .active(rgb(p.control_on).into()),
            )
            .bg(color)
            .into_any_element();
        HoverTarget { child, hovered }
    }
}

pub fn hover_text<T: Styled + InteractiveElement + IntoElement + 'static>(
    mut content: T,
    palette: Palette,
) -> MotionText<T> {
    let id = content
        .interactivity()
        .element_id
        .clone()
        .expect("interactive text ID");
    MotionText {
        id,
        content,
        palette,
    }
}

#[derive(IntoElement)]
pub struct MotionText<T: Styled + IntoElement + 'static> {
    id: ElementId,
    content: T,
    palette: Palette,
}

impl<T: Styled + IntoElement + 'static> RenderOnce for MotionText<T> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hovered = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| false);
        let color: Hsla = rgb(if *hovered.read(cx) {
            self.palette.ink
        } else {
            self.palette.muted
        })
        .into();
        let color = motion::transition(
            (self.id, "hover-text"),
            color,
            Transition::new(HOVER_DURATION),
            window,
            cx,
        );
        HoverTarget {
            child: self.content.text_color(color).into_any_element(),
            hovered,
        }
    }
}

#[derive(IntoElement)]
pub struct MotionSegments {
    base: Div,
    buttons: Vec<Button>,
    palette: Palette,
}

impl MotionSegments {
    pub fn new(base: Div, palette: Palette) -> Self {
        Self {
            base,
            buttons: Vec::new(),
            palette,
        }
    }

    pub fn children(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
        self.buttons.extend(buttons);
        self
    }
}

impl Styled for MotionSegments {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for MotionSegments {
    fn render(mut self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self
            .buttons
            .first_mut()
            .and_then(|button| button.interactivity().element_id.clone())
            .expect("segmented control ID");
        let selected_bounds = Rc::new(Cell::new(None));
        let paint_bounds = selected_bounds.clone();
        let p = self.palette;
        self.base
            .relative()
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        if let Some(target) = paint_bounds.get() {
                            let target: Bounds<Pixels> = target;
                            let relative = Bounds::new(target.origin - bounds.origin, target.size);
                            let moving = motion::transition(
                                (id, "selection-backdrop"),
                                relative,
                                Transition::new(SELECTION_DURATION),
                                window,
                                cx,
                            );
                            window.paint_quad(
                                fill(
                                    Bounds::new(bounds.origin + moving.origin, moving.size),
                                    rgb(p.control_on),
                                )
                                .corner_radii(px(3.)),
                            );
                        }
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .children(self.buttons.into_iter().map(move |button| {
                let selected = button.is_selected();
                let mut button = button.smooth_hover(p, HoverStyle::Control);
                button.shared_selection = true;
                TrackSelection {
                    child: button.into_any_element(),
                    selected,
                    bounds: selected_bounds.clone(),
                }
            }))
    }
}

// Forward the button's layout unchanged while measuring its actual bounds.
// This covers both content-sized and equally-sized segments, including resize.
struct TrackSelection {
    child: AnyElement,
    selected: bool,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl IntoElement for TrackSelection {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TrackSelection {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
        if self.selected {
            self.bounds.set(Some(bounds));
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

// Observe the native hitbox without adding a second hover handler to a button:
// GPUI Kit's managed tooltip already owns that handler.
struct HoverTarget {
    child: AnyElement,
    hovered: Entity<bool>,
}
impl IntoElement for HoverTarget {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for HoverTarget {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.child.prepaint(window, cx);
        hitbox
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let hovered = hitbox.is_hovered(window);
        if *self.hovered.read(cx) != hovered {
            self.hovered.update(cx, |value, cx| {
                *value = hovered;
                cx.notify();
            });
        }
        self.child.paint(window, cx);
    }
}
