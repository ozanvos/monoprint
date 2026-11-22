//! Short interaction transitions without changing control semantics or layout.
use crate::appearance::Palette;
use crate::{
    assets::IconName,
    ui::{Button, ButtonCustomVariant},
};
use gpui::{prelude::*, *};
use gpui_base::{
    Selectable,
    motion::{self, Transition},
};
use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};
use tact::motion::{self as tact_motion, Easing, Spring, Transition as TactTransition};

pub const HOVER_DURATION: Duration = Duration::from_millis(100);
pub const SELECTION_DURATION: Duration = Duration::from_millis(160);
pub const ENTRANCE_DURATION: Duration = Duration::from_millis(150);
const STAGGER: Duration = Duration::from_millis(14);
// Long enough for the last staggered tile to begin; later mounts are scrolling.
const REFLOW_WINDOW: Duration = Duration::from_millis(400);

// Near-critical springs: decisive, with no visible bounce on large surfaces.
fn glide() -> TactTransition {
    spring(420., 40.)
}

fn spring(stiffness: f32, damping: f32) -> TactTransition {
    // Match the original unit-mass spring and frequency-scaled rest tolerance.
    TactTransition::spring(Spring::new(stiffness, damping).rest(0.001, 0.001 * stiffness.sqrt()))
}

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
            pop_icon: None,
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
    pop_icon: Option<IconName>,
}

impl MotionButton {
    pub(super) fn menu_name(&self) -> SharedString {
        self.button.menu_name()
    }

    pub(super) fn decorative(mut self) -> Self {
        self.button = self.button.decorative();
        self
    }

    /// Sets the button icon and springs it in whenever it changes, such as
    /// Copy → Check, without moving the label or resizing the button.
    pub fn pop_icon(mut self, icon: IconName) -> Self {
        self.pop_icon = Some(icon);
        self
    }
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
            (self.id.clone(), "hover-color"),
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
        if let Some(icon) = self.pop_icon {
            let transformation = pop((self.id.clone(), "pop-icon"), Some(icon), window, cx);
            self.button = self
                .button
                .icon(crate::ui::icon(icon).with_transformation(transformation));
        }
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

/// An icon that springs in when it appears or changes. Absent icons take no
/// space, so a row's gap is unchanged until the icon is shown.
#[derive(IntoElement)]
pub struct PopIcon {
    id: ElementId,
    icon: Option<IconName>,
    size: Pixels,
    color: Hsla,
}

impl PopIcon {
    pub fn new(
        id: impl Into<ElementId>,
        icon: Option<IconName>,
        size: Pixels,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            size,
            color,
        }
    }
}

impl RenderOnce for PopIcon {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let transformation = pop(self.id, self.icon, window, cx);
        match self.icon {
            Some(icon) => crate::ui::icon(icon)
                .size(self.size)
                .text_color(self.color)
                .with_transformation(transformation)
                .into_any_element(),
            None => div().absolute().into_any_element(),
        }
    }
}

struct PopState {
    shown: Option<IconName>,
    generation: u64,
}

// The first icon is at rest. A changed icon starts a fresh spring.
fn pop(
    id: impl Into<ElementId>,
    icon: Option<IconName>,
    window: &mut Window,
    cx: &mut App,
) -> Transformation {
    let id = id.into();
    let state = window.use_keyed_state(id.clone(), cx, |_, _| PopState {
        shown: icon,
        generation: 0,
    });
    let generation = state.update(cx, |state, _| {
        if state.shown != icon {
            state.shown = icon;
            state.generation += 1;
        }
        state.generation
    });
    let rest = [1., 0.];
    let initial = if generation == 0 { rest } else { [0.5, -14.] };
    let [scale, angle] = tact_motion::animate_from(
        (id, format!("pop-{generation}")),
        initial,
        rest,
        spring(560., 30.),
        window,
        cx,
    );
    Transformation::scale(size(scale, scale)).with_rotation(radians(angle.to_radians()))
}

/// Fades an overlay in from a few pixels toward its anchor. Remounting replays
/// it; dismissal is immediate so the overlay never lingers over editing.
#[derive(IntoElement)]
pub struct Entrance {
    id: ElementId,
    child: AnyElement,
    from_y: f32,
    delay: Duration,
    animate: bool,
    fill: bool,
}

impl Entrance {
    pub fn new(id: impl Into<ElementId>, from_y: f32, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            child: child.into_any_element(),
            from_y,
            delay: Duration::ZERO,
            animate: true,
            fill: false,
        }
    }

    /// Mount at rest instead, e.g. for content that only scrolled into view.
    pub fn animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Let the child size itself against the entrance's parent.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl RenderOnce for Entrance {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rest = [0., 1.];
        let transition = TactTransition::tween(
            ENTRANCE_DURATION,
            // Linear x control points give the original cubic ease-out.
            Easing::CubicBezier(1. / 3., 1., 2. / 3., 1.),
        )
        .delay(self.delay);
        let [y, opacity] = if self.animate {
            tact_motion::animate_from(self.id, [self.from_y, 0.], rest, transition, window, cx)
        } else {
            rest
        };
        div()
            .when(self.fill, Styled::size_full)
            .relative()
            .top(px(y))
            .opacity(opacity)
            .child(self.child)
    }
}

/// The preview's displayed rect. Zoom presets spring between sizes; pinching,
/// panning and resizing follow the pointer exactly.
#[derive(Default)]
pub struct ZoomMotion {
    springing: bool,
}

impl ZoomMotion {
    pub fn spring(&mut self) {
        self.springing = true;
    }

    pub fn follow(&mut self) {
        self.springing = false;
    }

    pub fn bounds(
        &mut self,
        target: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Bounds<Pixels> {
        const ID: &str = "preview-zoom";
        let target = rect_target(target);
        if self.springing {
            let frame = tact_motion::animation(ID, target, glide(), window, cx);
            self.springing = frame.is_moving();
            rect_bounds(frame.value())
        } else {
            rect_bounds(tact_motion::jump(ID, target, window, cx))
        }
    }
}

fn rect_target(bounds: Bounds<Pixels>) -> [f32; 4] {
    [
        bounds.origin.x.into(),
        bounds.origin.y.into(),
        bounds.size.width.into(),
        bounds.size.height.into(),
    ]
}

fn rect_bounds([x, y, width, height]: [f32; 4]) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
}

/// Which theme tiles enter after a search or filter change. Tiles that keep
/// their grid position stay still; the rest fade up in reading order.
#[derive(Default)]
pub struct ThemeMotion {
    generation: u64,
    entrances: HashMap<usize, (u64, Instant, Duration)>,
}

impl ThemeMotion {
    pub fn reflow(&mut self, previous: &[usize], next: &[usize]) {
        self.generation += 1;
        let now = Instant::now();
        for (position, &index) in next.iter().enumerate() {
            if previous.get(position) == Some(&index) {
                continue;
            }
            // While typing, let a tile that is still entering finish instead of
            // restarting it on every keystroke.
            let entering = self
                .entrances
                .get(&index)
                .is_some_and(|(_, at, _)| now - *at < REFLOW_WINDOW);
            if !entering {
                let delay = STAGGER * position.min(11) as u32;
                self.entrances.insert(index, (self.generation, now, delay));
            }
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn tile(&self, index: usize, theme: &'static str, child: impl IntoElement) -> Entrance {
        let (generation, at, delay) = self.entrances.get(&index).copied().unwrap_or((
            0,
            Instant::now() - REFLOW_WINDOW,
            Duration::ZERO,
        ));
        Entrance::new(ElementId::NamedInteger(theme.into(), generation), 6., child)
            .animate(at.elapsed() < REFLOW_WINDOW)
            .delay(delay)
            .fill()
    }
}

/// Measures the selected tile, for a ring painted above the whole gallery.
pub fn track_selection(
    child: impl IntoElement,
    selected: bool,
    bounds: &Rc<Cell<Option<Bounds<Pixels>>>>,
) -> impl IntoElement {
    TrackSelection {
        child: child.into_any_element(),
        selected,
        bounds: bounds.clone(),
    }
}

struct RingState {
    theme: Option<&'static str>,
    generation: u64,
    gliding: bool,
}

/// One selection ring shared by every theme tile: choosing a theme glides it
/// to the new tile, even from a row that has scrolled away. It moves in the
/// gallery's content coordinates, so scrolling never lags behind, and it snaps
/// when a search or filter rebuilds the grid.
pub fn selection_ring(
    id: impl Into<ElementId>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    selected: Option<&'static str>,
    generation: u64,
    scroll: UniformListScrollHandle,
    color: Hsla,
) -> impl IntoElement {
    let id = id.into();
    canvas(
        |_, _, _| (),
        move |_, _, window, cx| {
            let Some(measured) = bounds.get() else {
                return;
            };
            let offset = scroll.0.borrow().base_handle.offset();
            let content = Bounds::new(measured.origin - offset, measured.size);
            let state =
                window.use_keyed_state((id.clone(), "selection-state"), cx, |_, _| RingState {
                    theme: selected,
                    generation,
                    gliding: false,
                });
            let gliding = state.update(cx, |state, _| {
                if state.generation != generation {
                    state.generation = generation;
                    state.gliding = false;
                } else if state.theme != selected {
                    state.gliding = true;
                }
                state.theme = selected;
                state.gliding
            });
            let transition = if gliding {
                glide()
            } else {
                TactTransition::instant()
            };
            let frame = tact_motion::animation(
                (id, format!("ring-{generation}")),
                rect_target(content),
                transition,
                window,
                cx,
            );
            if gliding && !frame.is_moving() {
                state.update(cx, |state, _| state.gliding = false);
            }
            let ring = rect_bounds(frame.value());
            window.paint_quad(quad(
                Bounds::new(ring.origin + offset, ring.size),
                px(5.),
                transparent_black(),
                px(1.),
                color,
                BorderStyle::Solid,
            ));
        },
    )
    .absolute()
    .inset_0()
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
// The tooltip already owns that handler.
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

#[cfg(test)]
mod tests {
    use super::{STAGGER, ThemeMotion};

    #[test]
    fn only_reflowed_tiles_enter_and_typing_does_not_restart_them() {
        let mut motion = ThemeMotion::default();
        motion.reflow(&[0, 1, 2, 3], &[0, 2, 3]);
        assert!(!motion.entrances.contains_key(&0), "kept its position");
        let (generation, _, delay) = motion.entrances[&3];
        assert_eq!(delay, STAGGER * 2, "staggered in reading order");
        motion.reflow(&[0, 2, 3], &[3]);
        assert_eq!(motion.entrances[&3].0, generation, "still entering");
        assert_eq!(motion.generation(), 2);
    }
}
