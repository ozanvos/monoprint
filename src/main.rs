mod cli;

use anyhow::{Context, Result, bail};
use clap::Parser;
use monoprint::{
    Appearance, BundledFont, Font, Language, LineNumbers, MAX_FONT_BYTES, MAX_SOURCE_BYTES,
    PngOptions, Renderer, Scale, Snippet, Style, Theme,
};
use std::{
    fs,
    io::{self, IsTerminal, Read, Write},
    process::ExitCode,
};

fn read_source(reader: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SOURCE_BYTES {
        bail!("source exceeds 1 MiB; select a smaller snippet");
    }
    String::from_utf8(bytes).context("source is not valid UTF-8")
}

fn run() -> Result<()> {
    let args = cli::Args::parse();
    if args.list_fonts {
        for font in BundledFont::all() {
            println!("{:<24} {}", font.id(), font.name());
        }
        return Ok(());
    }
    if args.list_themes {
        for theme in Theme::all() {
            println!(
                "{:<14} {:<14} {}",
                theme.id(),
                [Appearance::Dark, Appearance::Light]
                    .into_iter()
                    .filter(|&a| theme.supports(a))
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                theme.name()
            );
        }
        return Ok(());
    }
    if args.list_languages {
        for grammar in Language::all() {
            println!("{:<24} {}", grammar.id(), grammar.name());
        }
        return Ok(());
    }
    let output = args
        .output
        .as_ref()
        .context("an output is required: use -o image.png, image.svg, or image.html")?;
    let format = args.output_format()?;
    if args.png_compression.is_some() && format != cli::Format::Png {
        bail!("--png-compression is only valid for PNG output");
    }
    if output.as_os_str() == "-" && format == cli::Format::Png && io::stdout().is_terminal() {
        bail!("PNG output needs a file or pipe; stdout is a terminal");
    }
    if output.as_os_str() != "-" && output.exists() && !args.force {
        bail!(
            "{} already exists; use --force to replace it",
            output.display()
        );
    }
    let source = match &args.input {
        Some(path) if path.as_os_str() != "-" => read_source(
            fs::File::open(path).with_context(|| format!("opening {}", path.display()))?,
        )
        .with_context(|| format!("reading {} as UTF-8", path.display()))?,
        _ => {
            if io::stdin().is_terminal() {
                bail!("provide an input file or pipe UTF-8 source into stdin");
            }
            read_source(io::stdin().lock()).context("reading stdin as UTF-8")?
        }
    };
    let language = match &args.language {
        Some(id) => id.parse()?,
        None => args
            .input
            .as_deref()
            .and_then(Language::for_file_name)
            .unwrap_or_default(),
    };
    let mut snippet = Snippet::new(source)
        .language(language)
        .title(&args.title)
        .subtitle(&args.subtitle);
    for range in args.highlighted_ranges()? {
        snippet = snippet.highlight(range);
    }
    for range in &args.highlight_text {
        snippet = snippet.highlight_text(*range);
    }
    let mut style = Style::new(args.theme.parse()?)
        .transparent(args.transparent)
        .padding(args.padding)
        .font_size(args.font_size)
        .ligatures(args.ligatures)
        .line_numbers(if args.no_line_numbers {
            LineNumbers::Hide
        } else if args.line_numbers {
            LineNumbers::Show
        } else {
            LineNumbers::Auto
        });
    if args.light || args.dark {
        style = style.appearance(if args.light {
            Appearance::Light
        } else {
            Appearance::Dark
        });
    }
    if let Some(background) = args.background {
        style = style.background(background);
    }
    if let Some(width) = args.width {
        style = style.width(width);
    }
    if let Some(font) = args.font {
        style = style.font(font.font());
    }
    if let Some(font) = args.comment_font {
        style = style.comment_font(font.font());
    }
    for (path, comment) in [(&args.font_file, false), (&args.comment_font_file, true)] {
        let Some(path) = path else {
            continue;
        };
        let mut bytes = Vec::new();
        fs::File::open(path)
            .with_context(|| format!("opening font {}", path.display()))?
            .take(MAX_FONT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("reading font {}", path.display()))?;
        let font = Font::from_bytes(bytes)?;
        style = if comment {
            style.comment_font(font)
        } else {
            style.font(font)
        };
    }
    let renderer = Renderer::new();
    let rendered = match format {
        cli::Format::Png => renderer.png(
            &snippet,
            &style,
            PngOptions::new(Scale::try_from(args.scale)?)
                .compression(args.png_compression.unwrap_or_default()),
        )?,
        cli::Format::Svg => renderer.svg(&snippet, &style)?.into_bytes(),
        cli::Format::Html => renderer.html(&snippet, &style)?.into_bytes(),
    };
    if output.as_os_str() == "-" {
        io::stdout().lock().write_all(&rendered)?;
    } else {
        let mut options = fs::OpenOptions::new();
        options.write(true);
        if args.force {
            options.create(true).truncate(true);
        } else {
            options.create_new(true);
        }
        options
            .open(output)
            .with_context(|| format!("opening {}", output.display()))?
            .write_all(&rendered)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("monoprint: {error:#}");
            ExitCode::FAILURE
        }
    }
}
