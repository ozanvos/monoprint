//! A reproducible POSIX-shell invocation of the current Studio document.
use crate::document::{Document, Format, highlight_ranges};
use anyhow::{Context, Result, ensure};

/// The small settings preview; source is included only when copying the command.
pub fn invocation(document: &Document) -> Result<String> {
    ensure!(
        !document.source.contains('\0'),
        "Shell commands cannot contain NUL characters."
    );
    document.prepare(None, None)?;
    let mut args = vec!["monoprint -".to_owned()];
    let mut option = |name: &str, value: &str| -> Result<()> {
        args.push(format!("--{name}={}", quote(value)?));
        Ok(())
    };
    option(
        "output",
        &format!("./monoprint.{}", document.format.extension()),
    )?;
    option("theme", &document.theme)?;
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
    for (name, id, path) in [
        ("font", &document.font_id, &document.font_path),
        (
            "comment-font",
            &document.comment_font_id,
            &document.comment_font_path,
        ),
    ] {
        // Match FontCache's precedence for migrated documents containing both.
        if let Some(id) = id {
            let font: monoprint::BundledFont = id.parse()?;
            option(name, font.id())?;
        } else if let Some(path) = path {
            option(
                &format!("{name}-file"),
                path.to_str()
                    .context("The font path must be valid UTF-8 to copy a command.")?,
            )?;
        }
    }
    let lines = document
        .source
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .count();
    let highlights = highlight_ranges(&document.highlights, lines)?;
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
    for range in document
        .text_highlights
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        option("highlight-text", range)?;
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
    }
    Ok(args.join(" \\\n  "))
}

pub fn command(document: &Document) -> Result<String> {
    // printf preserves trailing newlines and does not expand shell syntax in code.
    Ok(format!(
        "printf '%s' {} | \\\n{}",
        quote(&document.source)?,
        invocation(document)?
    ))
}

fn quote(value: &str) -> Result<String> {
    ensure!(
        !value.contains('\0'),
        "Shell commands cannot contain NUL characters."
    );
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
