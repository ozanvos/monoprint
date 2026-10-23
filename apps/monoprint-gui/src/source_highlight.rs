//! Monoprint's native/fallback syntax projected into GPUI's source editor.
use gpui_kit::{
    Context, FontStyle, FontWeight, HighlightStyle, Rgba, SharedString, StrikethroughStyle, Task,
    UnderlineStyle, Window,
    component::input::{
        EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter,
        InputHighlighterFactory, Rope,
    },
    px,
};
use std::{cell::RefCell, ops::Range, rc::Rc, sync::OnceLock, time::Duration};

type Runs = Vec<(Range<usize>, HighlightStyle)>;

pub fn factory(light: bool) -> InputHighlighterFactory {
    Rc::new(move |language| {
        let language = language.parse::<monoprint::Language>().ok()?;
        Some(Box::new(SourceHighlighter {
            language: language.id().into(),
            light,
            runs: Rc::default(),
            task: None,
        }))
    })
}

struct SourceHighlighter {
    language: SharedString,
    light: bool,
    runs: Rc<RefCell<Runs>>,
    task: Option<Task<()>>,
}

impl InputHighlighter for SourceHighlighter {
    fn language(&self) -> SharedString {
        self.language.clone()
    }

    fn update(
        &mut self,
        edit: Option<InputEdit>,
        text: &Rope,
        _: bool,
        _: &mut Window,
        cx: &mut Context<EditorState>,
    ) {
        // Cancel stale parsing, but keep the last colors aligned with this edit
        // while the debounced replacement is being computed.
        self.task = None;
        if text.len() > monoprint::MAX_SOURCE_BYTES || self.language == "plain" {
            self.runs.borrow_mut().clear();
            return;
        }
        if let Some(edit) = edit {
            apply_edit(
                &mut self.runs.borrow_mut(),
                edit.start_byte..edit.old_end_byte,
                edit.new_end_byte,
            );
        } else {
            self.runs.borrow_mut().clear();
        }
        let text = text.clone();
        let language = self.language.clone();
        let runs = self.runs.clone();
        let light = self.light;
        self.task = Some(cx.spawn(async move |editor, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            let result = cx
                .background_executor()
                .spawn(async move { highlight(&text.to_string(), &language, light) })
                .await;
            match result {
                Ok(result) => {
                    *runs.borrow_mut() = result;
                    let _ = editor.update(cx, |_, cx| cx.notify());
                }
                Err(error) => eprintln!("Could not highlight source: {error:#}"),
            }
        }));
    }

    fn styles(&self, range: &Range<usize>, _: &dyn HighlightStyleResolver) -> Runs {
        visible_runs(&self.runs.borrow(), range)
    }

    fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

fn apply_edit(runs: &mut Runs, replaced: Range<usize>, new_end: usize) {
    let inherited = runs
        .iter()
        .find(|(range, _)| range.contains(&replaced.start))
        .or_else(|| {
            runs.iter()
                .rev()
                .find(|(range, _)| range.end <= replaced.start)
        })
        .map(|(_, style)| *style)
        .unwrap_or_default();
    let mut adjusted = Vec::with_capacity(runs.len() + 1);
    for (range, style) in runs.iter() {
        if range.start >= replaced.start {
            break;
        }
        adjusted.push((range.start..range.end.min(replaced.start), *style));
    }
    // Newly typed text inherits its local style until the highlighter resolves its syntax.
    if new_end > replaced.start {
        adjusted.push((replaced.start..new_end, inherited));
    }
    for (range, style) in runs.iter() {
        if range.end > replaced.end {
            adjusted.push((
                (range.start.max(replaced.end) - replaced.end + new_end)
                    ..(range.end - replaced.end + new_end),
                *style,
            ));
        }
    }
    *runs = adjusted;
}

fn highlight(source: &str, language: &str, light: bool) -> anyhow::Result<Runs> {
    static RENDERER: OnceLock<monoprint::Renderer> = OnceLock::new();
    let spans = RENDERER
        .get_or_init(monoprint::Renderer::new)
        .highlight_source(
            source,
            language.parse()?,
            if light {
                monoprint::Appearance::Light
            } else {
                monoprint::Appearance::Dark
            },
        )?;
    Ok(spans
        .into_iter()
        .map(|span| {
            let style = editor_style(&span);
            (span.range, style)
        })
        .collect())
}

fn editor_style(span: &monoprint::SyntaxSpan) -> HighlightStyle {
    let color = Rgba::try_from(span.foreground.as_str()).expect("bundled RGBA palette");
    HighlightStyle {
        color: Some(color.into()),
        font_weight: span.bold.then_some(FontWeight::BOLD),
        font_style: span.italic.then_some(FontStyle::Italic),
        underline: span.underline.then_some(UnderlineStyle {
            thickness: px(1.),
            ..Default::default()
        }),
        strikethrough: span.strikethrough.then_some(StrikethroughStyle {
            thickness: px(1.),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn visible_runs(runs: &[(Range<usize>, HighlightStyle)], range: &Range<usize>) -> Runs {
    let mut result = Vec::new();
    if range.is_empty() {
        return result;
    }
    let mut offset = range.start;
    let start = runs.partition_point(|(run, _)| run.end <= range.start);
    for (run, style) in &runs[start..] {
        if run.start >= range.end {
            break;
        }
        let start = run.start.max(offset);
        let end = run.end.min(range.end);
        if offset < start {
            result.push((offset..start, HighlightStyle::default()));
        }
        result.push((start..end, *style));
        offset = end;
    }
    if offset < range.end {
        result.push((offset..range.end, HighlightStyle::default()));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::rgb;

    #[test]
    fn pending_edits_keep_colors_and_unicode_ranges_aligned() {
        let mut source = "// café 🦀\r\nlet café = \"日本語\";\r\n// fin".to_owned();
        let mut runs = highlight(&source, "rust", false).unwrap();
        let replace = |source: &mut String, runs: &mut Runs, range: Range<usize>, text: &str| {
            let tail = source.rfind("// fin").unwrap();
            let styles = |runs: &Runs, tail, end| {
                visible_runs(runs, &(tail..end))
                    .into_iter()
                    .map(|(range, style)| (range.start - tail..range.end - tail, style))
                    .collect::<Runs>()
            };
            let before = styles(runs, tail, source.len());
            apply_edit(runs, range.clone(), range.start + text.len());
            source.replace_range(range, text);
            for (range, _) in runs.iter() {
                assert!(source.get(range.clone()).is_some());
                assert!(!range.is_empty());
            }
            for pair in runs.windows(2) {
                assert!(pair[0].0.end <= pair[1].0.start);
            }
            assert_eq!(
                styles(runs, source.rfind("// fin").unwrap(), source.len()),
                before
            );
            assert!(
                runs.iter()
                    .any(|(_, style)| style.color.is_some_and(|color| color.s > 0.3))
            );
        };
        replace(&mut source, &mut runs, 0..0, "// 新\r\n");
        let offset = source.find("café =").unwrap() + 3;
        replace(&mut source, &mut runs, offset..offset, "🦀");
        let offset = source.find("日本語").unwrap();
        replace(
            &mut source,
            &mut runs,
            offset..offset + "日本語".len(),
            "hello",
        );
        replace(&mut source, &mut runs, 0.."// 新\r\n".len(), "");
        apply_edit(&mut runs, 0..source.len(), 0);
        assert!(runs.is_empty());
    }

    #[test]
    fn shared_highlighter_colors_native_and_fallback_languages_in_both_appearances() {
        for light in [false, true] {
            for (language, source) in [
                ("rust", "// Comment\nfn main() { let x = \"hello\"; }"),
                (
                    "typescript",
                    "// Comment\nfunction greet() { return \"hello\"; }",
                ),
                ("ruby", "# Comment\ndef greet\n  puts \"hello\"\nend"),
                ("shellscript", "# Comment\necho \"hello\""),
                (
                    "hem",
                    "// Comment\neffect greet(): string { raise \"hello\" }",
                ),
            ] {
                let runs = highlight(source, language, light).unwrap();
                let colors = runs
                    .iter()
                    .map(|(_, style)| style.color.unwrap())
                    .collect::<Vec<_>>();
                assert!(
                    colors.iter().any(|color| color.s > 0.3),
                    "{language} must have colored syntax in light={light}"
                );
                assert!(
                    colors.iter().any(|color| *color != colors[0]),
                    "{language} must distinguish syntax in light={light}"
                );
                assert_eq!(runs.last().unwrap().0.end, source.len());
            }
        }
    }

    #[test]
    fn unicode_and_original_line_endings_keep_correct_byte_ranges() {
        let source = "// café 🦀\r\n\r\nlet café = \"日本語\";\r// fin\n";
        let runs = highlight(source, "rust", false).unwrap();
        let comment = source.find("// fin").unwrap();
        assert!(
            runs.iter().any(|(range, style)| range.start == comment
                && style.color == Some(rgb(0x9e9e98).into()))
        );
        for (range, _) in &runs {
            assert!(source.get(range.clone()).is_some());
        }
        let range = source.find("café =").unwrap()..source.len();
        let visible = visible_runs(&runs, &range);
        assert_eq!(visible.first().unwrap().0.start, range.start);
        assert_eq!(visible.last().unwrap().0.end, range.end);
        for pair in visible.windows(2) {
            assert_eq!(pair[0].0.end, pair[1].0.start);
        }
        assert!(visible_runs(&runs, &(comment..comment)).is_empty());
        assert_eq!(
            visible_runs(&[], &(0..8)),
            vec![(0..8, HighlightStyle::default())]
        );
    }
}
