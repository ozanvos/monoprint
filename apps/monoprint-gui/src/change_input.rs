//! Studio actions for terminal captures and unified changes.
use crate::studio::Studio;
use gpui_kit::{Context, PathPromptOptions, PromptLevel, Window};
impl Studio {
    pub fn sync_input_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.document.editor_source();
        let factory = crate::source_highlight::document_factory(
            crate::appearance::palette(cx).light,
            &self.document,
        );
        self.editor.update(cx, |state, cx| {
            state.set_highlighter_factory(factory, cx);
            state.set_highlighter(&self.document.language, cx);
            state.set_value(source, window, cx);
        });
        self.language.update(cx, |state, cx| {
            state.set_selected_value(&self.document.language, window, cx)
        });
        self.changed(cx);
    }
    pub fn use_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.document.language = "ansi".into();
        self.clear_diff(cx);
        self.sync_input_mode(window, cx);
    }
    pub fn edit_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = window.prompt(
            PromptLevel::Warning,
            "Edit as plain text?",
            Some("Terminal colors will be removed from this document."),
            &["Cancel", "Edit as plain text"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(1) {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.document.source = monoprint::ansi::strip(&this.document.source);
                    this.document.language = "plain".into();
                    this.sync_input_mode(window, cx);
                });
            } else {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.language.update(cx, |state, cx| {
                        state.set_selected_value(&this.document.language, window, cx)
                    });
                });
            }
        })
        .detach();
    }
    pub fn stop_diff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_patch = self.document.diff_patch;
        self.clear_diff(cx);
        if was_patch {
            self.document.language = "diff".into();
            self.sync_input_mode(window, cx);
        }
    }
    pub fn clear_diff(&mut self, cx: &mut Context<Self>) {
        self.document.diff_base = None;
        self.document.diff_patch = false;
        self.document.added.clear();
        self.document.removed.clear();
        self.changed(cx);
    }
    pub fn compare_clipboard(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|v| v.text())
            && text.len() <= monoprint::MAX_SOURCE_BYTES
        {
            self.clear_diff(cx);
            self.document.diff_base = Some(text);
            self.changed(cx);
        }
    }
    pub fn choose_base(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose previous version".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = dialog.await else {
                return;
            };
            let Some(path) = paths.first() else {
                return;
            };
            let path = path.clone();
            let result = cx
                .background_executor()
                .spawn(async move { crate::document::read_source(&path) })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(source) => {
                    this.clear_diff(cx);
                    this.document.diff_base = Some(source);
                    this.changed(cx);
                }
                Err(e) => {
                    this.notice = Some(e.to_string());
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub fn mark_change(&mut self, added: bool, cx: &mut Context<Self>) {
        let source = self.editor.read(cx).value().to_string();
        let selected =
            crate::selection::selected_lines(&source, self.editor.read(cx).selected_range());
        self.document.diff_base = None;
        self.document.diff_patch = false;
        let count = source.split('\n').count();
        let (target, other) = if added {
            (&mut self.document.added, &mut self.document.removed)
        } else {
            (&mut self.document.removed, &mut self.document.added)
        };
        *target = crate::selection::format_lines(crate::selection::toggle(
            crate::selection::lines(target, count),
            selected.clone(),
        ));
        let mut kept = Vec::new();
        for range in crate::selection::lines(other, count) {
            for n in range {
                if !selected.contains(&n) {
                    kept.push(n..n + 1);
                }
            }
        }
        *other = crate::selection::format_lines(kept);
        self.changed(cx);
    }
    pub fn show_patch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match monoprint::Diff::from_patch(&self.document.source) {
            Ok(diff) => {
                self.document.language = diff
                    .path()
                    .and_then(monoprint::Language::for_file_name)
                    .unwrap_or_default()
                    .id()
                    .into();
                self.clear_diff(cx);
                self.document.diff_patch = true;
                self.sync_input_mode(window, cx);
            }
            Err(e) => {
                self.notice = Some(e.to_string());
                cx.notify();
            }
        }
    }
}

/// Keep line marks attached to surviving lines across an editor replacement.
pub fn remap_marks(old: &str, new: &str, marks: &str) -> String {
    if marks.is_empty() || old == new {
        return marks.to_owned();
    }
    let count = old.split('\n').count();
    let marked = crate::selection::lines(marks, count);
    let Ok(rows) = monoprint::Diff::compare(old, new)
        .and_then(|diff| diff.context(monoprint::Context::All).rows())
    else {
        return String::new();
    };
    let is_marked = |line: usize| marked.iter().any(|range| range.contains(&(line - 1)));
    let mut out = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        if rows[i].kind == monoprint::RowKind::Context {
            if let (Some(old), Some(new)) = (rows[i].old, rows[i].new)
                && is_marked(old)
            {
                out.push(new - 1..new);
            }
            i += 1;
            continue;
        }
        let start = i;
        while i < rows.len() && rows[i].kind != monoprint::RowKind::Context {
            i += 1;
        }
        let removed: Vec<_> = rows[start..i].iter().filter_map(|r| r.old).collect();
        let added: Vec<_> = rows[start..i].iter().filter_map(|r| r.new).collect();
        for (index, old) in removed.iter().enumerate() {
            if is_marked(*old)
                && let Some(new) = added.get(index)
            {
                // Split lines inherit their original mark. Distinct old
                // marks never expand across the same replacement lines.
                let end = if index + 1 == removed.len() {
                    *added.last().unwrap()
                } else {
                    *new
                };
                out.push(new - 1..end);
            }
        }
    }
    // diff_lines omits the editor's empty row after a final newline.
    if old.ends_with('\n') && new.ends_with('\n') && is_marked(count) {
        let end = new.split('\n').count();
        out.push(end - 1..end);
    }
    crate::selection::format_lines(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marks_follow_insertions_deletions_and_replacements() {
        assert_eq!(remap_marks("a\nb\nc", "new\na\nb\nc", "2"), "3");
        assert_eq!(remap_marks("a\nb\nc", "a\nc", "2"), "");
        assert_eq!(remap_marks("a\nb\nc", "a\nchanged\nc", "2"), "2");
        assert_eq!(remap_marks("a\nb", "A\nB", "1"), "1");
        assert_eq!(remap_marks("a\nb", "A\nB", "2"), "2");
        assert_eq!(remap_marks("a", "A\nB", "1"), "1-2");
        assert_eq!(remap_marks("a\nkeep\nb", "A\nkeep\nB", "2"), "2");
        assert_eq!(remap_marks("a\n", "new\na\n", "2"), "3");
    }
}

#[cfg(test)]
mod document_tests {
    use crate::document::Document;
    #[test]
    fn terminal_raw_and_diff_base_survive_serialization() {
        let original = Document {
            source: "\x1b[31mFAIL\x1b[0m".into(),
            language: "ansi".into(),
            ..Default::default()
        };
        let restored: Document =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        assert_eq!(restored.source, original.source);
        assert_eq!(restored.editor_source(), "FAIL");
        let original = Document {
            source: "new".into(),
            diff_base: Some("old".into()),
            ..Default::default()
        };
        let restored: Document =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        assert_eq!(restored.diff_base, original.diff_base);
        assert!(restored.prepare(None, None).is_ok());
    }
}

#[cfg(test)]
mod selection_tests {
    use crate::document::Document;
    #[test]
    fn comparison_selections_follow_the_new_side() {
        let document = Document {
            source: "new".into(),
            diff_base: Some("old".into()),
            highlights: "1".into(),
            text_highlights: "1:1-1:3".into(),
            ..Default::default()
        };
        let marks = document.render_highlights().unwrap();
        assert_eq!(marks.lines, [(2, 2)]);
        assert_eq!(marks.text[0], monoprint::TextRange::new((2, 1), (2, 3)));
        let command = crate::cli_command::command(&document).unwrap();
        assert!(command.contains("--highlight=2"));
        assert!(command.contains("--highlight-text=2:1-2:3"));
    }
}
