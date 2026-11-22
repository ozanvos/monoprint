use gpui::{App, Pixels, Point, Window};
use gpui_base::input::{NativeMenu, NativeMenuItem};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send, sel};
use objc2_app_kit::{NSMenu, NSMenuItem, NSView};
use objc2_foundation::{NSPoint, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::cell::Cell;

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "MonoprintEditorMenuTarget"]
    #[ivars = Cell<Option<usize>>]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(choose:))]
        fn choose(&self, sender: &NSMenuItem) {
            self.ivars().set(Some(sender.tag() as usize));
        }
    }
);

fn track(view: &NSView, entries: &NativeMenu, position: Point<Pixels>) -> Option<usize> {
    let marker = MainThreadMarker::new()?;
    let target = MenuTarget::alloc().set_ivars(Cell::new(None));
    let target: Retained<MenuTarget> = unsafe { msg_send![super(target), init] };
    let menu = NSMenu::new(marker);
    menu.setAutoenablesItems(false);
    for (index, item) in entries.items.iter().enumerate() {
        match item {
            NativeMenuItem::Separator => menu.addItem(&NSMenuItem::separatorItem(marker)),
            NativeMenuItem::Action {
                label, disabled, ..
            } => {
                let item = NSMenuItem::new(marker);
                item.setTitle(&NSString::from_str(label));
                item.setEnabled(!disabled);
                item.setTag(index as isize);
                unsafe {
                    item.setTarget(Some(&target as &AnyObject));
                    item.setAction(Some(sel!(choose:)));
                }
                menu.addItem(&item);
            }
        }
    }
    let point = NSPoint::new(
        f32::from(position.x).into(),
        view.bounds().size.height - f64::from(f32::from(position.y)),
    );
    menu.popUpMenuPositioningItem_atLocation_inView(None, point, Some(view));
    target.ivars().get()
}

pub(super) fn show(menu: NativeMenu, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let Ok(raw) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(raw) = raw.as_raw() else {
        return;
    };
    // Retain the view while the AppKit tracking loop runs. That loop must
    // run outside the GPUI window update which opened the context menu.
    let Some(view) = (unsafe { Retained::retain(raw.ns_view.as_ptr().cast::<NSView>()) }) else {
        return;
    };
    let handle = Window::window_handle(window);
    cx.spawn(async move |cx| {
        let selected = track(&view, &menu, position);
        cx.update(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(NativeMenuItem::Action { action, .. }) =
                    selected.and_then(|index| menu.items.get(index))
                {
                    window.dispatch_action(action.boxed_clone(), cx);
                }
                window.refresh();
            });
        });
    })
    .detach();
}
