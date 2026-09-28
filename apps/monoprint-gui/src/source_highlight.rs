//! Giallo tokens projected into GPUI's editor, independent of the export theme.
use giallo::{HighlightOptions, Registry, ThemeVariant};
use gpui_kit::{
    Context, FontStyle, FontWeight, HighlightStyle, SharedString, StrikethroughStyle, Task,
    UnderlineStyle, Window,
    component::input::{
        EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter,
        InputHighlighterFactory, Rope,
    },
    px, rgb,
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
        _: Option<InputEdit>,
        text: &Rope,
        _: bool,
        _: &mut Window,
        cx: &mut Context<EditorState>,
    ) {
        // Dropping the previous task cancels its apply step, including when the
        // user switches languages. Never paint old byte offsets onto a new edit.
        self.task = None;
        self.runs.borrow_mut().clear();
        if text.len() > monoprint::MAX_SOURCE_BYTES || self.language == "plain" {
            return;
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
                .spawn(async move { highlight(&text.to_string(), &language) })
                .await;
            match result {
                Ok(mut result) => {
                    if light {
                        for (_, style) in &mut result {
                            style.color = Some(
                                if style.color == Some(rgb(0x909090).into()) {
                                    rgb(0x7C7C77)
                                } else if style.color == Some(rgb(0xB8B8B8).into()) {
                                    rgb(0x4A4A47)
                                } else {
                                    rgb(0x2A2A28)
                                }
                                .into(),
                            );
                        }
                    }
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

fn highlight(source: &str, language: &str) -> anyhow::Result<Runs> {
    // The same reviewed grammar/theme artifact used by Monoprint's renderer.
    // Load once, off the UI thread; no grammar downloads or extra language set.
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    let registry = REGISTRY
        .get_or_init(|| {
            Registry::load(include_bytes!("../../../assets/registry.zst"))
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| anyhow::anyhow!(error.clone()))?;
    let options = HighlightOptions::new(language, ThemeVariant::Single("ink-mono-dark"))
        .merge_whitespace(false);
    let code = registry.highlight(source, &options)?;
    let mut result = Vec::new();
    let mut offset = 0;
    for line in code.tokens {
        for token in line {
            let end = offset + token.text.len();
            let ThemeVariant::Single(style) = token.style else {
                unreachable!()
            };
            if end > offset {
                result.push((offset..end, editor_style(style)));
            }
            offset = end;
        }
        // Giallo normalizes CRLF and CR to LF. Keep offsets in the original
        // editor buffer, including Unicode and empty lines.
        if source.as_bytes().get(offset) == Some(&b'\r') {
            offset += 1;
        }
        if source.as_bytes().get(offset) == Some(&b'\n') {
            offset += 1;
        }
    }
    Ok(result)
}

fn editor_style(style: giallo::Style) -> HighlightStyle {
    let hex = style.foreground.as_hex();
    // Mono's #666 comments suit a black export, but need more contrast on our
    // dark editor. Preserve its three tonal roles with a readable comment floor.
    let color = match hex.as_str() {
        "#666666" => rgb(0x909090),
        "#A7A7A7" => rgb(0xb8b8b8),
        _ => rgb(0xeeeeee),
    };
    let font = style.font_style;
    HighlightStyle {
        color: Some(color.into()),
        font_weight: font
            .contains(giallo::FontStyle::BOLD)
            .then_some(FontWeight::BOLD),
        font_style: font
            .contains(giallo::FontStyle::ITALIC)
            .then_some(FontStyle::Italic),
        underline: font
            .contains(giallo::FontStyle::UNDERLINE)
            .then_some(UnderlineStyle {
                thickness: px(1.),
                ..Default::default()
            }),
        strikethrough: font.contains(giallo::FontStyle::STRIKETHROUGH).then_some(
            StrikethroughStyle {
                thickness: px(1.),
                ..Default::default()
            },
        ),
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

    #[test]
    fn giallo_covers_languages_beyond_the_kit_grammars_in_monochrome() {
        for (language, source) in [
            ("rust", "// Comment\nfn main() { let x = \"hello\"; }"),
            (
                "typescript",
                "// Comment\nfunction greet() { return \"hello\"; }",
            ),
            ("ruby", "# Comment\ndef greet\n  puts \"hello\"\nend"),
            ("shellscript", "# Comment\necho \"hello\""),
        ] {
            let runs = highlight(source, language).unwrap();
            let colors = runs
                .iter()
                .map(|(_, style)| style.color.unwrap())
                .collect::<Vec<_>>();
            assert!(colors.iter().all(|color| color.s == 0.));
            assert!(
                colors.iter().any(|color| *color != colors[0]),
                "{language} must have tonal syntax"
            );
            assert_eq!(runs.last().unwrap().0.end, source.len());
        }
    }

    #[test]
    fn unicode_and_original_line_endings_keep_correct_byte_ranges() {
        let source = "// café 🦀\r\n\r\nlet café = \"日本語\";\r// fin\n";
        let runs = highlight(source, "rust").unwrap();
        let comment = source.find("// fin").unwrap();
        assert!(
            runs.iter().any(|(range, style)| range.start == comment
                && style.color == Some(rgb(0x909090).into()))
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
