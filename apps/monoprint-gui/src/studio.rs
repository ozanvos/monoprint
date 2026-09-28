use crate::{
    document::{self, Document, Format},
    preview::{Preview, PreviewWorker},
};
use gpui_kit::{
    component::{
        input::{
            EditorState, InputEvent, InputState, RopeExt, TextDecoration, TextDecorationCollection,
        },
        select::{SearchableVec, SelectEvent, SelectItem, SelectState},
    },
    *,
};
use monoprint::{BundledFont, Language, Renderer, Theme};
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

pub struct HighlightMenu {
    pub range: std::ops::Range<usize>,
    pub caret: usize,
    pub anchor: Bounds<Pixels>,
}

pub struct Studio {
    pub document: Document,
    pub editor: Entity<EditorState>,
    pub title: Entity<InputState>,
    pub source_marks: TextDecorationCollection,
    pub highlight_menu: Option<HighlightMenu>,
    pub advanced: bool,
    pub settings_scroll: ScrollHandle,
    pub width: Entity<InputState>,
    pub padding: Entity<InputState>,
    pub language: Entity<Choices>,
    pub code_typeface: Entity<Choices>,
    pub comment_typeface: Entity<Choices>,
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
    last_selection: std::ops::Range<usize>,
    last_scroll: Point<Pixels>,
    last_selection_anchor: Option<Bounds<Pixels>>,
    last_source_bounds: Bounds<Pixels>,
    last_visible_rows: Option<std::ops::Range<usize>>,
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
        let source_marks = editor.update(cx, |state, cx| {
            state.create_decorations_collection(Vec::new(), cx)
        });
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
        let code_typeface = choices(
            typeface_items(&document, false),
            document.typeface_selection(false),
            window,
            cx,
        );
        let comment_typeface = choices(
            typeface_items(&document, true),
            document.typeface_selection(true),
            window,
            cx,
        );
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
                    this.highlight_menu = None;
                    let track = crate::selection::resolve(
                        &this.document.source,
                        &this.document.text_highlights,
                    )
                    .is_some();
                    this.document.source = entity.read(cx).value().to_string();
                    this.document.highlights =
                        crate::selection::format_lines(this.line_highlights(cx));
                    if track {
                        // GPUI's decorations already followed this edit, including Unicode,
                        // insertions and deletions. Persist the same positions for export.
                        this.document.text_highlights = crate::selection::format(
                            &this.document.source,
                            this.source_marks.get_ranges(cx),
                        );
                    } else {
                        this.sync_source_marks(cx);
                    }
                    this.changed(cx);
                }
            }),
        );
        subscriptions.push(cx.observe(&editor, |this, editor, cx| {
            let editor = editor.read(cx);
            let selection = editor.selected_range();
            let scroll = editor.scroll_offset();
            let bounds = editor.input_bounds();
            let visible_rows = editor.visible_row_range();
            let cursor = editor.cursor();
            let anchor = if selection.is_empty() {
                None
            } else {
                editor.range_to_bounds(&(cursor..cursor))
            };
            if this.highlight_menu.as_ref().is_some_and(|menu| {
                !selection.is_empty()
                    || cursor != menu.caret
                    || scroll != this.last_scroll
                    || bounds != this.last_source_bounds
            }) {
                this.highlight_menu = None;
                cx.notify();
            }
            if selection != this.last_selection
                || scroll != this.last_scroll
                || anchor != this.last_selection_anchor
                || bounds != this.last_source_bounds
                || visible_rows != this.last_visible_rows
            {
                this.last_selection = selection;
                this.last_scroll = scroll;
                this.last_selection_anchor = anchor;
                this.last_source_bounds = bounds;
                this.last_visible_rows = visible_rows;
                // Selection/scroll changes only update the toolbar, never render an image.
                cx.notify();
            }
        }));
        for (field, kind) in [(&title, 0), (&width, 1), (&padding, 2)] {
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
                        1 => this.document.width = value,
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
        for (field, comments) in [(&code_typeface, false), (&comment_typeface, true)] {
            subscriptions.push(cx.subscribe_in(
                field,
                window,
                move |this, _, event, window, cx| {
                    let SelectEvent::Confirm(Some(id)) = event else {
                        return;
                    };
                    if id == "custom" {
                        // Keep the current choice visible until a file is accepted.
                        this.sync_typeface(comments, window, cx);
                        this.choose_font(comments, window, cx);
                    } else {
                        this.document.select_typeface(comments, id);
                        this.sync_typeface(comments, window, cx);
                        this.changed(cx);
                    }
                },
            ));
        }
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
                                || theme.family().to_lowercase().contains(&query)
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
            source_marks,
            highlight_menu: None,
            advanced: false,
            settings_scroll: ScrollHandle::new(),
            width,
            padding,
            language,
            code_typeface,
            comment_typeface,
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
            last_selection: 0..0,
            last_scroll: point(px(0.), px(0.)),
            last_selection_anchor: None,
            last_source_bounds: Bounds::default(),
            last_visible_rows: None,
        };
        this.sync_source_marks(cx);
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
        self.clear_highlights(cx);
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
        self.document.text_highlights.clear();
        self.source_marks.clear(cx);
        self.highlight_menu = None;
        self.document.title = self.source_name.clone();
        self.language.update(cx, |state, cx| {
            state.set_selected_value(&language.id().to_owned(), window, cx)
        });
        self.title.update(cx, |state, cx| {
            state.set_value(self.source_name.clone(), window, cx)
        });
        self.editor.update(cx, |state, cx| {
            state.set_highlighter(language.id(), cx);
            state.set_value(source, window, cx);
        });
        self.changed(cx);
    }
    pub fn sync_typeface(&self, comments: bool, window: &mut Window, cx: &mut Context<Self>) {
        let field = if comments {
            &self.comment_typeface
        } else {
            &self.code_typeface
        };
        field.update(cx, |state, cx| {
            state.set_items(
                SearchableVec::new(typeface_items(&self.document, comments)),
                window,
                cx,
            );
            state.set_selected_value(
                &self.document.typeface_selection(comments).to_owned(),
                window,
                cx,
            );
        });
    }

    pub fn choose_font(&mut self, comments: bool, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                if comments {
                    "Choose a comment typeface (TTF or OTF)"
                } else {
                    "Choose a code typeface (TTF or OTF)"
                }
                .into(),
            ),
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
            let _ = this.update_in(cx, |this, window, cx| match result {
                Ok(Some(path)) => {
                    if comments {
                        this.document.comment_font_id = None;
                        this.document.comment_font_path = Some(path);
                    } else {
                        this.document.font_id = None;
                        this.document.font_path = Some(path);
                    }
                    this.sync_typeface(comments, window, cx);
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
    pub fn highlight_selection(&mut self, whole_lines: bool, cx: &mut Context<Self>) {
        let range = self.editor.read(cx).selected_range();
        if range.is_empty() {
            self.notice = Some("Select text in the source editor first.".into());
            cx.notify();
            return;
        }
        let source = self.editor.read(cx).value().to_string();
        if !whole_lines {
            let ranges = crate::selection::toggle(self.source_marks.get_ranges(cx), range);
            self.document.text_highlights = crate::selection::format(&source, ranges);
            self.sync_source_marks(cx);
            self.changed(cx);
        } else {
            self.toggle_line_highlights(crate::selection::selected_lines(&source, range), cx);
        }
    }
    pub fn line_highlights(&self, cx: &App) -> Vec<std::ops::Range<usize>> {
        crate::selection::lines(
            &self.document.highlights,
            self.editor.read(cx).text().lines_len(),
        )
    }
    pub fn toggle_line_highlights(
        &mut self,
        lines: std::ops::Range<usize>,
        cx: &mut Context<Self>,
    ) {
        self.document.highlights = crate::selection::format_lines(crate::selection::toggle(
            self.line_highlights(cx),
            lines,
        ));
        self.changed(cx);
    }
    pub fn open_highlight_menu(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        if !editor.selected_range().is_empty() {
            return;
        }
        let Some(offset) = crate::source_lines::character_at_point(editor, position) else {
            return;
        };
        let Some(range) = crate::selection::containing(self.source_marks.get_ranges(cx), offset)
        else {
            return;
        };
        let height = editor.line_height().unwrap_or(px(18.));
        self.highlight_menu = Some(HighlightMenu {
            range,
            caret: editor.cursor(),
            anchor: Bounds::new(
                point(position.x, position.y - height / 2.),
                size(px(0.), height),
            ),
        });
        cx.notify();
    }
    pub fn remove_clicked_highlight(&mut self, cx: &mut Context<Self>) {
        let Some(menu) = self.highlight_menu.take() else {
            return;
        };
        let ranges = self.source_marks.get_ranges(cx);
        if crate::selection::covers(&ranges, &menu.range) {
            self.document.text_highlights = crate::selection::format(
                &self.document.source,
                crate::selection::toggle(ranges, menu.range),
            );
            self.sync_source_marks(cx);
            self.changed(cx);
        } else {
            cx.notify();
        }
    }
    pub fn sync_source_marks(&self, cx: &mut Context<Self>) {
        let ranges =
            crate::selection::resolve(&self.document.source, &self.document.text_highlights)
                .unwrap_or_default();
        let style = HighlightStyle {
            background_color: Some(rgb(crate::appearance::ACCENT).opacity(0.18).into()),
            ..Default::default()
        };
        self.source_marks.set(
            ranges
                .into_iter()
                .map(|range| TextDecoration::new(range, style))
                .collect(),
            cx,
        );
    }

    pub fn clear_highlights(&mut self, cx: &mut Context<Self>) {
        self.highlight_menu = None;
        self.document.highlights.clear();
        self.document.text_highlights.clear();
        self.source_marks.clear(cx);
        self.changed(cx);
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

fn typeface_items(document: &Document, comments: bool) -> Vec<Choice> {
    let mut items = vec![Choice {
        id: String::new(),
        name: if comments {
            "Same as code"
        } else {
            "Theme typeface"
        }
        .into(),
    }];
    items.extend(BundledFont::all().map(|font| Choice {
        id: font.id().into(),
        name: font.name().into(),
    }));
    let path = if comments {
        &document.comment_font_path
    } else {
        &document.font_path
    };
    let label = path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| format!("Custom · {}", name.to_string_lossy()))
        .unwrap_or_else(|| "Choose font file…".into());
    items.push(Choice {
        id: "custom".into(),
        name: label.into(),
    });
    items
}
impl Focusable for Studio {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
