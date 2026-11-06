use monoprint::{Context, Diff, DiffNumbers, Language, Renderer, Snippet, Style, ansi};
#[test]
fn terminal_visible_positions_and_styles() {
    let r = Renderer::new();
    let source = "\x1b[31;44;1m界\x1b[0m ok\rX\x1b[K\n\x1b[38:2::10:20:30mtest";
    assert_eq!(ansi::strip(source), "X\ntest");
    let html = r
        .html(
            &Snippet::new(source)
                .language(Language::ANSI)
                .highlight_text(monoprint::TextRange::new((2, 1), (2, 3))),
            &Style::default(),
        )
        .unwrap();
    assert!(html.to_lowercase().contains("#0a141e"));
    assert!(!html.contains('\x1b'));
    assert!(!html.contains("界"));
    let spans = r
        .highlight_source(source, Language::ANSI, monoprint::Appearance::Dark)
        .unwrap();
    assert_eq!(spans.last().unwrap().range.end, 6);
}
#[test]
fn compare_preserves_independent_lexers_and_word_ranges() {
    let r = Renderer::new();
    let d = Diff::compare("/* old\nlet old = 10;", "let new = 20;\nlet next = 30;")
        .unwrap()
        .context(Context::All);
    let html = r
        .html(
            &Snippet::diff(d).language("rust".parse().unwrap()),
            &Style::default()
                .diff_numbers(DiffNumbers::Both)
                .diff_words(false),
        )
        .unwrap();
    assert!(html.contains("diff-added"));
    assert!(html.contains("diff-removed"));
    assert!(html.contains("data-sign=\"−\""));
    let spans = r
        .highlight_source(
            "let new = 20;",
            "rust".parse().unwrap(),
            monoprint::Appearance::Dark,
        )
        .unwrap();
    let added = html.split("class=\"line diff-added\"").nth(1).unwrap();
    assert!(added.contains(&format!("color:{};", spans[0].foreground)));
}
#[test]
fn patch_validation_and_numbering() {
    let patch = "--- a/x.rs\n+++ b/x.rs\n@@ -10,2 +10,2 @@\n same\n-old\n+new\n@@ -20 +20 @@\n-last\n+next\n";
    let diff = Diff::from_patch(patch).unwrap();
    assert_eq!(diff.path(), Some("x.rs"));
    assert_eq!(diff.displayed_source(), "same\nold\nnew\nlast\nnext");
    let html = Renderer::new()
        .html(
            &Snippet::diff(diff).language("rust".parse().unwrap()),
            &Style::default(),
        )
        .unwrap();
    assert!(html.contains("8 unchanged lines"));
    assert!(html.contains("data-number=\"20\""));
    for bad in [
        "@@ -1,2 +1 @@\n-a\n+b",
        "Binary files a and b differ",
        "@@ -1,2 +1,2 @@\n-a\n\\ No newline at end of file\n+b\n c",
        "@@ -1 +1 @@\n-a\n+b\n\\ No newline at end of file\n@@ -3 +3 @@\n-c\n+d",
        "@@ -1 +1 @@\n-a\n+b\n@@ -1 +1 @@\n-c\n+d",
    ] {
        assert!(Diff::from_patch(bad).is_err(), "{bad}");
    }
    assert!(Diff::from_patch(&format!("{patch}\n--- a/y\n+++ b/y\n@@ -1 +1 @@\n-a\n+b")).is_err());
}
#[test]
fn marks_validate_and_number() {
    let r = Renderer::new();
    let style = Style::default();
    assert!(
        r.html(
            &Snippet::diff(Diff::marked("one\ntwo").added(1).removed(1)),
            &style
        )
        .is_err()
    );
    assert!(
        r.html(
            &Snippet::diff(Diff::marked("one").added(usize::MAX)),
            &style
        )
        .is_err()
    );
    let html = r
        .html(
            &Snippet::diff(
                Diff::marked("one\ntwo\nthree")
                    .removed(1)
                    .added(2)
                    .first_line(120),
            )
            .language("rust".parse().unwrap()),
            &style,
        )
        .unwrap();
    assert!(html.contains("data-number=\"120\""));
    assert!(html.contains("data-number=\"121\""));
}
#[test]
fn all_formats_render_changed_and_terminal_text() {
    let r = Renderer::new();
    for theme in [
        "monoprint",
        "github-dark-default",
        "single-ink",
        "proof-marks",
        "wayfinding",
    ] {
        let style = Style::new(theme.parse().unwrap());
        for snippet in [
            Snippet::new("\x1b[31mFAIL\x1b[0m ok").language(Language::ANSI),
            Snippet::diff(Diff::compare("let x = 1;", "let x = 2;").unwrap())
                .language("rust".parse().unwrap()),
        ] {
            assert!(r.svg(&snippet, &style).unwrap().contains("<svg"));
            assert!(
                r.png(&snippet, &style, monoprint::Scale::X1)
                    .unwrap()
                    .starts_with(b"\x89PNG")
            );
        }
    }
}

#[cfg(feature = "cli")]
#[test]
fn cli_terminal_and_patch_stdin() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    fn run(source: &str, args: &[&str]) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_monoprint"))
            .args(args)
            .args(["-o", "-", "--format", "html"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    let terminal = run("\x1b[31mfail\x1b[0m", &["-l", "terminal"]);
    assert!(terminal.status.success());
    assert!(!terminal.stdout.contains(&27));
    let patch = run(
        "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n",
        &["--patch", "-"],
    );
    assert!(
        patch.status.success(),
        "{}",
        String::from_utf8_lossy(&patch.stderr)
    );
    assert!(String::from_utf8_lossy(&patch.stdout).contains("diff-added"));
    assert!(
        !run("x", &["--added", "1", "--removed", "1"])
            .status
            .success()
    );
    assert!(!run("x", &["--diff", "-", "-"]).status.success());
}

#[test]
fn zero_count_patch_gaps_and_comparison_copy_roundtrip() {
    let patch = "--- a/x.rs\n+++ b/x.rs\n@@ -0,0 +1 @@\n+first\n@@ -10 +11 @@\n-old\n+new\n";
    let d = Diff::from_patch(patch).unwrap();
    assert!(
        d.rows()
            .unwrap()
            .iter()
            .any(|r| r.kind == monoprint::RowKind::Fold { hidden: 9 })
    );
    for (old, new) in [("", ""), ("same", "same"), ("a\nb\n", "a\nc"), ("", "new")] {
        let d = Diff::compare(old, new).unwrap().context(Context::All);
        let p = Diff::from_patch(&d.patch()).unwrap().context(Context::All);
        assert_eq!(p.displayed_source(), d.displayed_source());
    }
}
