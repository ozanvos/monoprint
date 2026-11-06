//! A reproducible POSIX-shell invocation of the current Studio document.
use crate::document::{Document, Format};
use anyhow::{Context, Result, ensure};

/// The small settings preview; source is included only when copying the command.
pub fn invocation(document: &Document) -> Result<String> {
    ensure!(
        !document.source.contains('\0'),
        "Shell commands cannot contain NUL characters."
    );
    document.prepare(None, None)?;
    let mut args = vec![
        if document.diff_patch || document.diff_base.is_some() {
            "monoprint --patch -"
        } else {
            "monoprint -"
        }
        .to_owned(),
    ];
    let mut option = |name: &str, value: &str| -> Result<()> {
        args.push(format!("--{name}={}", quote(value)?));
        Ok(())
    };
    option(
        "output",
        &format!("./monoprint.{}", document.format.extension()),
    )?;
    option("theme", &document.theme)?;
    if document.diff_patch || document.diff_base.is_some() {
        option(
            "context",
            &document
                .diff_context
                .map(|n| n.to_string())
                .unwrap_or("all".into()),
        )?;
    }
    if !document.added.is_empty() {
        option("added", &document.added)?;
    }
    if !document.removed.is_empty() {
        option("removed", &document.removed)?;
    }
    option("diff-numbers", &document.diff_numbers)?;
    option("language", &document.language)?;
    option("background", &document.background()?.to_string())?;
    option("padding", document.padding.trim())?;
    option("font-size", document.font_size.trim())?;
    if !document.width.trim().is_empty() {
        option("width", document.width.trim())?;
    }
    if !document.title.is_empty() {
        option("title", &document.title)?;
    }
    if document.format == Format::Png {
        monoprint::Scale::try_from(document.scale)?;
        let compression: monoprint::PngCompression = document.png_compression.parse()?;
        option("scale", &document.scale.to_string())?;
        option("png-compression", &compression.to_string())?;
    }
    for (name, comments) in [("font", false), ("comment-font", true)] {
        let slot = document.font_slot(comments);
        // Match FontCache's precedence for migrated documents containing both.
        if let Some(id) = slot.id {
            let font: monoprint::BundledFont = id.parse()?;
            option(name, font.id())?;
        } else if let Some(path) = slot.path {
            option(
                &format!("{name}-file"),
                path.to_str()
                    .context("The font path must be valid UTF-8 to copy a command.")?,
            )?;
        }
    }
    let selections = document.render_highlights()?;
    let highlights = selections.lines;
    if !highlights.is_empty() {
        option(
            "highlight",
            &highlights
                .into_iter()
                .map(|(start, end)| {
                    if start == end {
                        start.to_string()
                    } else {
                        format!("{start}-{end}")
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        )?;
    }
    for range in selections.text {
        option("highlight-text", &range.to_string())?;
    }
    if !document.diff_words && (document.diff_base.is_some() || document.diff_patch) {
        args.push("--no-diff-words".into());
    }
    args.push(if document.light { "--light" } else { "--dark" }.into());
    if let Some(numbers) = document.line_numbers {
        args.push(
            if numbers {
                "--line-numbers"
            } else {
                "--no-line-numbers"
            }
            .into(),
        );
    }
    if document.ligatures {
        args.push("--ligatures".into());
    } else {
        args.push("--no-ligatures".into());
    }
    Ok(args.join(" \\\n  "))
}

pub fn command(document: &Document) -> Result<String> {
    let invocation = invocation(document)?;
    let source = if let Some(base) = &document.diff_base {
        monoprint::Diff::compare(base, &document.source)?.patch()
    } else {
        document.source.clone()
    };
    // A quoted delimiter keeps shell substitutions in the patch inert. Keep
    // ANSI-C quoting for captures containing controls, including colored code
    // inside a comparison, so copying never sends raw ESC to the shell.
    if document.diff_base.is_some()
        && !source
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        let mut delimiter = "MONOPRINT_PATCH".to_owned();
        while source.lines().any(|line| line == delimiter) {
            delimiter.push('_');
        }
        return Ok(format!("{invocation} <<'{delimiter}'\n{source}{delimiter}"));
    }
    // printf preserves trailing newlines and does not expand shell syntax in code.
    Ok(format!(
        "printf '%s' {} | \\\n{}",
        quote(&source)?,
        invocation
    ))
}

fn quote(value: &str) -> Result<String> {
    ensure!(
        !value.contains('\0'),
        "Shell commands cannot contain NUL characters."
    );
    if value
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        let mut out = String::from("$'");
        for c in value.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '\'' => out.push_str("\\'"),
                c if c.is_ascii_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
                c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('\'');
        return Ok(out);
    }
    if !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_./,:@%+-".contains(&c))
    {
        return Ok(value.to_owned());
    }
    Ok(format!("'{}'", value.replace('\'', "'\"'\"'")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_formats_and_inherited_settings() {
        let mut document = Document {
            width: String::new(),
            ..Default::default()
        };
        for format in [Format::Png, Format::Svg, Format::Html] {
            document.format = format;
            let command = invocation(&document).unwrap();
            assert!(command.contains(&format!("--output=./monoprint.{}", format.extension())));
            assert_eq!(
                command.contains("--png-compression=balanced"),
                format == Format::Png
            );
            assert_eq!(command.contains("--scale=2"), format == Format::Png);
            assert!(!command.contains("--width"));
            assert!(!command.contains("--font="));
            assert!(!command.contains("line-numbers"));
        }
        document.ligatures = false;
        assert!(invocation(&document).unwrap().contains("--no-ligatures"));
        document.padding = "-".into();
        assert!(command(&document).is_err());
        document.padding = "32".into();
        document.source.push('\0');
        assert!(command(&document).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn shell_round_trip_preserves_source_and_literal_arguments() {
        let document = Document {
            source: "// ' \" $HOME $(exit 19) `exit 20` \\ %s\nconst café = '☕';\n\n".into(),
            title: "-title's $(exit 21) `exit 22`\nsecond line".into(),
            font_path: Some("/tmp/font's $(exit 23).ttf".into()),
            comment_font_id: Some("commit-mono".into()),
            highlights: "1 - 2".into(),
            text_highlights: "1:1-1:3,2:1-2:4".into(),
            line_numbers: Some(false),
            ligatures: true,
            transparent: true,
            ..Default::default()
        };
        let directory = tempfile::tempdir().unwrap();
        let script = format!(
            "monoprint() {{ printf '%s\\0' \"$@\" > args; cat > source; }}\n{}",
            command(&document).unwrap()
        );
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(script)
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("source")).unwrap(),
            document.source
        );
        let args = std::fs::read_to_string(directory.path().join("args")).unwrap();
        let args = args.split('\0').collect::<Vec<_>>();
        for expected in [
            "-",
            "--background=none",
            "--highlight=1-2",
            "--highlight-text=1:1-1:3",
            "--highlight-text=2:1-2:4",
            "--no-line-numbers",
            "--ligatures",
            "--dark",
            "--comment-font=commit-mono",
            "--font-file=/tmp/font's $(exit 23).ttf",
            &format!("--title={}", document.title),
        ] {
            assert!(args.contains(&expected), "missing {expected:?} in {args:?}");
        }
    }
}

#[cfg(test)]
mod change_tests {
    use super::*;
    #[test]
    fn terminal_commands_escape_controls() {
        let source = "\x1b[31mFAIL\x1b[0m\rnext '\\$(`x`)";
        let quoted = quote(source).unwrap();
        assert!(!quoted.contains('\x1b'));
        assert!(quoted.starts_with("$'"));
    }
    #[cfg(unix)]
    #[test]
    fn terminal_commands_roundtrip_in_bash() {
        let source = "\x1b[31mFAIL\x1b[0m\rnext '\\$(`x`)";
        let quoted = quote(source).unwrap();
        let output = std::process::Command::new("/bin/bash")
            .args(["-c", &format!("printf '%s' {quoted}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, source.as_bytes());
    }
    #[test]
    fn comparison_command_contains_patch_and_settings() {
        let document = Document {
            source: "let n = 2;".into(),
            diff_base: Some("let n = 1;".into()),
            language: "rust".into(),
            ..Default::default()
        };
        let command = command(&document).unwrap();
        assert!(command.contains("--patch -"));
        assert!(command.contains("<<'MONOPRINT_PATCH'"));
        assert!(command.contains("-let n = 1;"));
        assert!(command.contains("+let n = 2;"));
    }
    #[cfg(unix)]
    #[test]
    fn comparison_heredoc_preserves_literal_patch() {
        let document = Document {
            source: "$(exit 19) `exit 20` $HOME\nMONOPRINT_PATCH".into(),
            diff_base: Some("old".into()),
            ..Default::default()
        };
        let script = format!("monoprint() {{ cat; }}\n{}", command(&document).unwrap());
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &script])
            .output()
            .unwrap();
        assert!(output.status.success());
        let expected =
            monoprint::Diff::compare(document.diff_base.as_ref().unwrap(), &document.source)
                .unwrap()
                .patch();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}
