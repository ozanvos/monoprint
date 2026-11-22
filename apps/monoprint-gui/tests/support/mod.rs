use gpui::*;
use gpui_base::test_support::{self, ElementSnapshot};

pub trait TestWindowExt {
    fn find(&self, id: impl Into<ElementId>) -> ElementSnapshot;
    fn try_find(&self, id: impl Into<ElementId>) -> Option<ElementSnapshot>;
    fn render_frame(&mut self, cx: &mut App);
    fn click(&mut self, id: impl Into<ElementId>, cx: &mut App);
    fn hover(&mut self, id: impl Into<ElementId>, cx: &mut App);
    fn scroll(&mut self, id: impl Into<ElementId>, delta: ScrollDelta, cx: &mut App);
    fn drag(&mut self, from: Point<Pixels>, to: Point<Pixels>, cx: &mut App);
    fn press(&mut self, key: &str, cx: &mut App);
    fn input(&mut self, text: &str, cx: &mut App);
}

fn move_pointer(
    window: &mut Window,
    position: Point<Pixels>,
    pressed_button: Option<MouseButton>,
    cx: &mut App,
) {
    window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button,
            modifiers: window.modifiers(),
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn mouse_down(window: &mut Window, position: Point<Pixels>, cx: &mut App) {
    window.dispatch_event(
        MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: window.modifiers(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn mouse_up(window: &mut Window, position: Point<Pixels>, cx: &mut App) {
    window.dispatch_event(
        MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers: window.modifiers(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

impl TestWindowExt for Window {
    fn find(&self, id: impl Into<ElementId>) -> ElementSnapshot {
        let id = id.into();
        self.try_find(id.clone()).unwrap_or_else(|| {
            panic!(
                "element {id:?} is missing from {}",
                test_support::registered_paths(self)
            )
        })
    }
    fn try_find(&self, id: impl Into<ElementId>) -> Option<ElementSnapshot> {
        test_support::find(self, &[], &id.into())
    }
    fn render_frame(&mut self, cx: &mut App) {
        self.refresh();
        self.draw(cx).clear(cx);
    }
    fn click(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        self.render_frame(cx);
        let position = self.find(id).bounds().center();
        move_pointer(self, position, None, cx);
        mouse_down(self, position, cx);
        mouse_up(self, position, cx);
    }
    fn hover(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        self.render_frame(cx);
        move_pointer(self, self.find(id).bounds().center(), None, cx);
    }
    fn scroll(&mut self, id: impl Into<ElementId>, delta: ScrollDelta, cx: &mut App) {
        self.render_frame(cx);
        let position = self.find(id).bounds().center();
        move_pointer(self, position, None, cx);
        self.dispatch_event(
            ScrollEvent {
                position,
                delta,
                modifiers: self.modifiers(),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        self.render_frame(cx);
    }
    fn drag(&mut self, from: Point<Pixels>, to: Point<Pixels>, cx: &mut App) {
        self.render_frame(cx);
        move_pointer(self, from, None, cx);
        mouse_down(self, from, cx);
        for step in 1..=8 {
            let fraction = step as f32 / 8.;
            move_pointer(
                self,
                from + (to - from) * fraction,
                Some(MouseButton::Left),
                cx,
            );
        }
        mouse_up(self, to, cx);
    }
    fn press(&mut self, key: &str, cx: &mut App) {
        self.render_frame(cx);
        let keystroke = Keystroke::parse_with_os(key, cx.platform_info().os).unwrap();
        if matches!(keystroke.key.as_str(), "enter" | "tab") {
            self.dispatch_event(
                KeyDownEvent {
                    keystroke: keystroke.clone(),
                    is_held: false,
                    prefer_character_input: false,
                }
                .to_platform_input(),
                cx,
            );
        } else {
            self.dispatch_keystroke(keystroke.clone().with_simulated_ime(), cx);
        }
        self.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
        self.render_frame(cx);
    }
    fn input(&mut self, text: &str, cx: &mut App) {
        self.render_frame(cx);
        for character in text.chars() {
            let text = character.to_string();
            let mut keystroke = Keystroke::parse_with_os(&text, cx.platform_info().os).unwrap();
            keystroke.key_char = Some(text);
            self.dispatch_keystroke(keystroke, cx);
            self.render_frame(cx);
        }
    }
}
