mod assets;
mod cli;
mod effects;
mod highlight;
mod render;

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::{
    fs,
    io::{self, IsTerminal, Read, Write},
    process::ExitCode,
};

fn run() -> Result<()> {
    let args = cli::Args::parse();
    if args.list_themes {
        for theme in assets::themes().values() {
            println!(
                "{:<14} {:<14} {}",
                theme.id,
                theme.syntax.keys().cloned().collect::<Vec<_>>().join(","),
                theme.name
            );
        }
        return Ok(());
    }
    if args.list_languages {
        for grammar in assets::grammars() {
            println!("{:<24} {}", grammar.name, grammar.display_name);
        }
        println!("{:<24} Plain Text", "plain");
        return Ok(());
    }
    let output = args
        .output
        .as_ref()
        .context("an output is required: use -o image.png, image.svg, or image.html")?;
    let format = args.output_format()?;
    if output.as_os_str() == "-" && format == cli::Format::Png && io::stdout().is_terminal() {
        bail!("PNG output needs a file or pipe; stdout is a terminal");
    }
    if output.as_os_str() != "-" && output.exists() && !args.force {
        bail!(
            "{} already exists; use --force to replace it",
            output.display()
        );
    }
    let mut source = String::new();
    match &args.input {
        Some(path) if path.as_os_str() != "-" => {
            source = fs::read_to_string(path)
                .with_context(|| format!("reading {} as UTF-8", path.display()))?
        }
        _ => {
            if io::stdin().is_terminal() {
                bail!("provide an input file or pipe UTF-8 source into stdin");
            }
            io::stdin()
                .read_to_string(&mut source)
                .context("reading stdin as UTF-8")?;
        }
    }
    if source.len() > 1_048_576 {
        bail!("source exceeds 1 MiB; select a smaller snippet");
    }
    let source = source.replace("\r\n", "\n").replace('\r', "\n");
    let rendered = render::export(&args, &source, format)?;
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
            eprintln!("code-images: {error:#}");
            ExitCode::FAILURE
        }
    }
}
