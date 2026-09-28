use crate::{
    document::{self, Document, Format},
    preview::{Preview, PreviewWorker},
};
use gpui_kit::{
    component::{
        input::{EditorState, InputEvent, InputState},
        select::{SearchableVec, SelectEvent, SelectItem, SelectState},
    },
    *,
};
use monoprint::{Language, Renderer, Theme};
use std::{path::PathBuf, sync::Arc, time::Duration};

gpui_kit::actions!(
    monoprint_gui,
    [OpenSource, ExportImage, CopyImage, PasteSource, Quit]
);

#[derive(Clone)]
pub struct Choice {
    pub id: String,
    pub name: SharedString,
}
impl SelectItem for Choice {
    type Value = String;
    fn title(&self) -> SharedString {
        self.name.clone()
    }
    fn value(&self) -> &String {
        &self.id
    }
    fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q) || self.id.contains(&q)
    }
}
pub type Choices = SelectState<SearchableVec<Choice>>;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum CopyState {
    #[default]
    Idle,
    Copying,
    Copied,
}

pub struct Studio {
    pub document: Document,
    pub editor: Entity<EditorState>,
    pub title: Entity<InputState>,
    pub highlights: Entity<InputState>,
    pub width: Entity<InputState>,
    pub padding: Entity<InputState>,
    pub language: Entity<Choices>,
    pub theme_search: Entity<InputState>,
    pub theme_scroll: UniformListScrollHandle,
    pub filtered_themes: Vec<usize>,
    pub focus: FocusHandle,
    pub preview: Option<(Arc<Image>, Preview)>,
    pub revision: u64,
    pub preview_revision: u64,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub exporting: bool,
    pub copy_state: CopyState,
    pub zoom: bool,
    pub wrap: bool,
    pub source_name: String,
    pub last_export: Option<PathBuf>,
    pub thumbnails: Vec<(Theme, Arc<Image>)>,
    _render_task: Task<()>,
    _copy_feedback_task: Option<Task<()>>,
    renderer: Renderer,
    worker: PreviewWorker,
    _subscriptions: Vec<Subscription>,
    _quit: Subscription,
}

impl Studio {
    pub fn new(
        mut document: Document,
        startup_error: Option<String>,
        source_name: String,
        session: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let theme = document.theme.parse::<Theme>().unwrap_or_default();
        document.select_theme(theme);
        let editor = cx.new(|cx| {
            let mut editor = EditorState::new(window, cx)
                .language(&document.language)
                .default_value(document.source.clone())
                .soft_wrap(true);
            editor.set_highlighter_factory(crate::source_highlight::factory(), cx);
            editor
        });
        let title = input(&document.title, "Optional filename", window, cx);
        let highlights = input(&document.highlights, "e.g. 2, 4-6", window, cx);
        let width = input(&document.width, "Auto", window, cx);
        let padding = input(&document.padding, "64", window, cx);
        let mut languages = Language::all()
            .map(|l| Choice {
                id: l.id().into(),
                name: l.name().into(),
            })
            .collect::<Vec<_>>();
        languages.sort_by_key(|a| a.name.to_lowercase());
        let language = choices(languages, &document.language, window, cx);
        let theme_search = input("", "Find a theme…", window, cx);
        let thumbnails = crate::appearance::thumbnails();
        let theme_scroll = UniformListScrollHandle::new();
        if let Some(index) = thumbnails.iter().position(|(item, _)| *item == theme) {
            theme_scroll.scroll_to_item(index / 2, ScrollStrategy::Center);
        }
        let session = Arc::new(document::SessionStore::new(session));
        let renderer = Renderer::new();
        let (worker, results) = PreviewWorker::new(renderer.clone(), session.clone());
        let render_task = cx.spawn(async move |this, cx| {
            while let Ok(completion) = results.recv().await {
                if this
                    .update(cx, |this, cx| {
                        if completion.revision != this.revision {
                            return;
                        }
                        this.preview_revision = completion.revision;
                        match completion.result {
                            Ok(preview) => {
                                let image = Arc::new(Image::from_bytes(
                                    ImageFormat::Png,
                                    preview.png.clone(),
                                ));
                                this.preview = Some((image, preview));
                                this.error = None;
                            }
                            Err(error) => {
                                this.error = Some(error);
                            }
                        }
                        if let Some(error) = completion.session_error {
                            this.notice = Some(format!("Session could not be saved: {error}"));
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(
            cx.subscribe_in(&editor, window, |this, entity, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.document.source = entity.read(cx).value().to_string();
                    this.changed(cx);
                }
            }),
        );
        for (field, kind) in [(&title, 0), (&highlights, 1), (&width, 2), (&padding, 3)] {
            subscriptions.push(cx.subscribe_in(
                field,
                window,
                move |this, entity, event, _, cx| {
                    if !matches!(event, InputEvent::Change) {
                        return;
                    }
                    let value = entity.read(cx).value().to_string();
                    match kind {
                        0 => this.document.title = value,
                        1 => this.document.highlights = value,
                        2 => this.document.width = value,
                        _ => this.document.padding = value,
                    }
                    this.changed(cx);
                },
            ));
        }
        subscriptions.push(cx.subscribe_in(&language, window, |this, _, event, _, cx| {
            let SelectEvent::Confirm(Some(id)) = event else {
                return;
            };
            this.document.language = id.clone();
            this.editor
                .update(cx, |state, cx| state.set_highlighter(id, cx));
            this.changed(cx);
        }));
        subscriptions.push(
            cx.subscribe_in(&theme_search, window, |this, entity, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = entity.read(cx).value().trim().to_lowercase();
                    this.filtered_themes = this
                        .thumbnails
                        .iter()
                        .enumerate()
                        .filter(|(_, (theme, _))| {
                            theme.name().to_lowercase().contains(&query)
                                || theme.id().contains(&query)
                        })
                        .map(|(index, _)| index)
                        .collect();
                    this.theme_scroll
                        .scroll_to_item_strict(0, ScrollStrategy::Top);
                    cx.notify();
                }
            }),
        );
        // Flush the latest editing state on normal application quit, including the
        // last keystroke if the debounced preview has not started yet.
        let quit_session = session.clone();
        let quit = cx.on_app_quit(move |this, cx| {
            let doc = this.document.clone();
            let store = quit_session.clone();
            let revision = this.revision;
            cx.background_executor().spawn(async move {
                if let Err(error) = store.save(revision, &doc) {
                    eprintln!("Could not save the session: {error:#}");
                }
            })
        });
        let mut this = Self {
            document,
            editor,
            title,
            highlights,
            width,
            padding,
            language,
            theme_search,
            theme_scroll,
            filtered_themes: (0..thumbnails.len()).collect(),
            focus: cx.focus_handle(),
            preview: None,
            revision: 0,
            preview_revision: 0,
            error: None,
            notice: None,
            exporting: false,
            copy_state: CopyState::Idle,
            zoom: false,
            wrap: true,
            source_name,
            last_export: None,
            thumbnails,
            renderer,
            worker,
            _render_task: render_task,
            _copy_feedback_task: None,
            _subscriptions: subscriptions,
            _quit: quit,
        };
        this.changed(cx);
        this.notice = startup_error;
        this
    }

    pub fn changed(&mut self, cx: &mut Context<Self>) {
        self.revision += 1;
        self.error = None;
        self.notice = None;
        self.worker.request(self.revision, self.document.clone());
        cx.notify();
    }
    pub fn select_theme(&mut self, theme: Theme, _window: &mut Window, cx: &mut Context<Self>) {
        self.document.select_theme(theme);
        self.changed(cx);
    }
    pub fn paste(&mut self, _: &PasteSource, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) else {
            self.notice = Some("Copy some code, then paste it here.".into());
            cx.notify();
            return;
        };
        if text.len() > monoprint::MAX_SOURCE_BYTES {
            self.notice = Some("Paste a snippet smaller than 1 MiB.".into());
            cx.notify();
            return;
        }
        // Replace is deliberate for the toolbar action; Cmd-V in the editor inserts.
        self.editor.update(cx, |state, cx| {
            state.replace_all(text, window, cx);
            state.focus(window, cx);
        });
    }
    pub fn open(&mut self, _: &OpenSource, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open source file".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = async {
                let Some(paths) = dialog.await?? else {
                    return Ok(None);
                };
                let Some(path) = paths.into_iter().next() else {
                    return Ok(None);
                };
                let source = cx
                    .background_executor()
                    .spawn({
                        let path = path.clone();
                        async move { document::read_source(&path) }
                    })
                    .await?;
                Ok::<_, anyhow::Error>(Some((path, source)))
            }
            .await;
            let _ = this.update_in(cx, |this, window, cx| match result {
                Ok(Some((path, source))) => this.install_source(path, source, window, cx),
                Ok(None) => {}
                Err(e) => {
                    this.notice = Some(format!("{e:#}"));
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub(crate) fn install_source(
        &mut self,
        path: PathBuf,
        source: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.document.source = source.clone();
        self.source_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let language = Language::for_file_name(&path).unwrap_or_default();
        self.document.language = language.id().into();
        self.document.highlights.clear();
        self.document.title = self.source_name.clone();
        self.language.update(cx, |state, cx| {
            state.set_selected_value(&language.id().to_owned(), window, cx)
        });
        self.title.update(cx, |state, cx| {
            state.set_value(self.source_name.clone(), window, cx)
        });
        self.highlights
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.editor.update(cx, |state, cx| {
            state.set_highlighter(language.id(), cx);
            state.set_value(source, window, cx);
        });
        self.changed(cx);
    }
    pub fn choose_font(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a TTF or OTF font".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = async {
                let Some(paths) = dialog.await?? else {
                    return Ok(None);
                };
                let Some(path) = paths.into_iter().next() else {
                    return Ok(None);
                };
                cx.background_executor()
                    .spawn({
                        let path = path.clone();
                        async move { document::FontCache::default().load(Some(&path)) }
                    })
                    .await?;
                Ok::<_, anyhow::Error>(Some(path))
            }
            .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Some(path)) => {
                    this.document.font_path = Some(path);
                    this.changed(cx);
                }
                Ok(None) => {}
                Err(e) => {
                    this.notice = Some(format!("{e:#}"));
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub fn export_file(&mut self, _: &ExportImage, window: &mut Window, cx: &mut Context<Self>) {
        if self.exporting || self.copy_state == CopyState::Copying {
            return;
        }
        let document = self.document.clone();
        let directory = self
            .last_export
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let dialog = cx.prompt_for_new_path(&directory, Some(&document.suggested_name()));
        let renderer = self.renderer.clone();
        self.exporting = true;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = async {
                let Some(path) = dialog.await?? else {
                    return Ok(None);
                };
                let path = document::export_path(path, document.format)?;
                cx.background_executor()
                    .spawn(async move {
                        let bytes = document::export(&renderer, &document, document.format)?;
                        document::write_atomic(&path, &bytes)?;
                        Ok::<_, anyhow::Error>(Some(path))
                    })
                    .await
            }
            .await;
            let _ = this.update(cx, |this, cx| {
                this.exporting = false;
                match result {
                    Ok(Some(path)) => {
                        this.notice = Some(format!(
                            "Saved {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                        this.last_export = Some(path);
                    }
                    Ok(None) => {}
                    Err(e) => this.notice = Some(format!("Export failed: {e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub fn copy_image(&mut self, _: &CopyImage, window: &mut Window, cx: &mut Context<Self>) {
        if self.exporting || self.copy_state == CopyState::Copying {
            return;
        }
        self._copy_feedback_task = None;
        self.copy_state = CopyState::Copying;
        cx.notify();
        let document = self.document.clone();
        let renderer = self.renderer.clone();
        let task = cx
            .background_executor()
            .spawn(async move { document::export(&renderer, &document, Format::Png) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.copy_state = CopyState::Idle;
                match result {
                    Ok(bytes) => {
                        cx.write_to_clipboard(ClipboardItem::new_image(&Image::from_bytes(
                            ImageFormat::Png,
                            bytes,
                        )));
                        this.copy_state = CopyState::Copied;
                        this._copy_feedback_task = Some(cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_secs(2)).await;
                            let _ = this.update(cx, |this, cx| {
                                this.copy_state = CopyState::Idle;
                                cx.notify();
                            });
                        }));
                    }
                    Err(e) => this.notice = Some(format!("Could not copy image: {e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn input(
    value: &str,
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<Studio>,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .default_value(value.to_owned())
            .placeholder(placeholder.to_owned())
    })
}
fn choices(
    items: Vec<Choice>,
    selected: &str,
    window: &mut Window,
    cx: &mut Context<Studio>,
) -> Entity<Choices> {
    let index = items
        .iter()
        .position(|i| i.id == selected)
        .map(|row| gpui_kit::component::IndexPath::default().row(row));
    cx.new(|cx| SelectState::new(SearchableVec::new(items), index, window, cx).searchable(true))
}
impl Focusable for Studio {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
