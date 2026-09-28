mod appearance;
mod document;
#[cfg(target_os = "macos")]
mod material;
mod preview;
mod source_highlight;
mod studio;
mod view;
use gpui_kit::{
    component::{Root, input},
    *,
};
use studio::*;
fn main() {
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
            }
            Err(e) => error = Some(format!("{e:#}")),
        }
    }
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            appearance::init(cx);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([
                KeyBinding::new("secondary-q", Quit, None),
                KeyBinding::new("secondary-o", OpenSource, Some("Monoprint")),
                KeyBinding::new("secondary-s", ExportImage, Some("Monoprint")),
                KeyBinding::new("secondary-shift-c", CopyImage, Some("Monoprint")),
            ]);
            cx.set_menus(vec![
                Menu {
                    disabled: false,
                    name: "Monoprint".into(),
                    items: vec![MenuItem::action("Quit Monoprint", Quit)],
                },
                Menu {
                    disabled: false,
                    name: "File".into(),
                    items: vec![
                        MenuItem::action("Open Source…", OpenSource),
                        MenuItem::action("Export Image…", ExportImage),
                    ],
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
                        traffic_light_position: Some(point(px(14.), px(21.))),
                    }),
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
