#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod appearance;
mod assets;
mod canvas_resize;
mod cli_command;
mod document;
mod handoff;
mod interaction_motion;
mod language_picker;
#[cfg(target_os = "macos")]
mod material;
mod platform_keys;
mod preview;
mod sandbox;
mod selection;
mod source_highlight;
mod source_lines;
mod studio;
mod updates;
mod view;
mod viewport;
use gpui_kit::{
    component::{Root, input},
    *,
};
use studio::*;
fn main() {
    updates::startup();
    let session = document::session_path();
    let (mut document, mut error) = match session
        .as_ref()
        .filter(|p| p.exists())
        .map(|p| document::load_session(p))
    {
        Some(Ok(doc)) => (doc, None),
        Some(Err(e)) => (
            document::Document::default(),
            Some(format!("Could not restore the previous session: {e:#}")),
        ),
        None => (document::Document::default(), None),
    };
    let mut source_name = "Untitled snippet".to_owned();
    if let Some(path) = std::env::args_os().nth(1).map(std::path::PathBuf::from) {
        match document::read_source(&path) {
            Ok(source) => {
                document.source = source;
                document.language = monoprint::Language::for_file_name(&path)
                    .unwrap_or_default()
                    .id()
                    .into();
                source_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                document.title = source_name.clone();
                document.highlights.clear();
                document.text_highlights.clear();
            }
            Err(e) => error = Some(format!("{e:#}")),
        }
    }
    // Register before launch: Finder may deliver its open event before the window exists.
    let (open_files, incoming_files) = async_channel::unbounded();
    let application = gpui_kit::application().with_assets(assets::Assets);
    application.on_open_urls(move |urls| {
        let _ = open_files.try_send(urls);
    });
    application.run(move |cx| {
        gpui_kit::init(cx);
        platform_keys::init(cx);
        appearance::init(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &OpenPrivacy, cx| cx.open_url("https://monoprint.7ea.st/privacy/"));
        cx.on_action(|_: &OpenSupport, cx| cx.open_url("https://monoprint.7ea.st/support/"));
        cx.bind_keys([
            KeyBinding::new("secondary-q", Quit, None),
            KeyBinding::new("secondary-o", OpenSource, Some("Monoprint")),
            KeyBinding::new("secondary-s", ExportImage, Some("Monoprint")),
            KeyBinding::new("secondary-shift-c", CopyImage, Some("Monoprint")),
            KeyBinding::new("secondary-shift-l", ToggleUiAppearance, Some("Monoprint")),
            KeyBinding::new("escape", input::Escape, Some("ThemeBrowser")),
        ]);
        let mut file_items = vec![
            MenuItem::action("Open Source…", OpenSource),
            MenuItem::action("Export Image…", ExportImage),
        ];
        if cfg!(all(target_os = "macos", not(feature = "app-store"))) {
            file_items.extend([
                MenuItem::separator(),
                MenuItem::action("Set Up Shortcuts…", ShowShortcuts),
            ]);
        }
        let mut app_items = Vec::new();
        if !cfg!(feature = "app-store") {
            app_items.extend([
                MenuItem::action("Check for Updates…", CheckForUpdates),
                MenuItem::action("Toggle Automatic Update Checks", ToggleAutomaticUpdates),
                MenuItem::separator(),
            ]);
        }
        if cfg!(target_os = "linux") {
            app_items.extend([
                MenuItem::action("Install for This User…", InstallForUser),
                MenuItem::action("Uninstall Monoprint…", UninstallApp),
                MenuItem::separator(),
            ]);
        }
        app_items.push(MenuItem::action("Quit Monoprint", Quit));
        cx.set_menus(vec![
            Menu {
                disabled: false,
                name: "Monoprint".into(),
                items: app_items,
            },
            Menu {
                disabled: false,
                name: "File".into(),
                items: file_items,
            },
            Menu {
                disabled: false,
                name: "Edit".into(),
                items: vec![
                    MenuItem::action("Undo", input::Undo),
                    MenuItem::action("Redo", input::Redo),
                    MenuItem::separator(),
                    MenuItem::action("Cut", input::Cut),
                    MenuItem::action("Copy", input::Copy),
                    MenuItem::action("Paste", input::Paste),
                    MenuItem::action("Select All", input::SelectAll),
                    MenuItem::separator(),
                    MenuItem::action("Copy Image", CopyImage),
                ],
            },
            Menu {
                disabled: false,
                name: "View".into(),
                items: vec![MenuItem::action(
                    "Toggle Light / Dark Interface",
                    ToggleUiAppearance,
                )],
            },
            Menu {
                disabled: false,
                name: "Help".into(),
                items: vec![
                    MenuItem::action("Monoprint Support", OpenSupport),
                    MenuItem::action("Privacy Policy", OpenPrivacy),
                ],
            },
        ]);
        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1100.), px(700.))),
                window_background: if cfg!(target_os = "macos") {
                    // The unmodified AppKit material is installed below.
                    WindowBackgroundAppearance::Transparent
                } else {
                    WindowBackgroundAppearance::Opaque
                },
                titlebar: Some(TitlebarOptions {
                    title: Some("Monoprint".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(13.), px(14.))),
                }),
                // Route custom-header gestures through GPUI instead of AppKit,
                // which otherwise also maximizes when a header button is double-clicked.
                app_owns_titlebar_drag: true,
                ..Default::default()
            },
            |window, cx| {
                #[cfg(target_os = "macos")]
                if let Err(error) = material::install(window) {
                    window.set_background_appearance(WindowBackgroundAppearance::Opaque);
                    eprintln!("Could not install the window material: {error:#}");
                }
                let studio =
                    cx.new(|cx| Studio::new(document, error, source_name, session, window, cx));
                studio.update(cx, |studio, cx| {
                    studio.listen_for_sources(incoming_files, window, cx);
                });
                let focus = studio.read(cx).focus.clone();
                window.focus(&focus, cx);
                window.on_window_should_close(cx, |_, cx| {
                    cx.quit();
                    true
                });
                // Root otherwise paints an opaque theme background over the material.
                cx.new(|cx| Root::new(studio, window, cx).bg(transparent_black()))
            },
        )
        .expect("open Monoprint window");
        cx.activate(true);
    });
}
