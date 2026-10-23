use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use monoprint::LineRange;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Png,
    Svg,
    Html,
}

#[derive(Debug, Parser)]
#[command(version, about = "Offline code images with native Rust rendering")]
pub struct Args {
    /// UTF-8 source file; omit or use - to read stdin.
    pub input: Option<PathBuf>,
    /// Output path (.png, .svg, .html), or - for stdout with --format.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long, value_enum)]
    pub format: Option<Format>,
    /// Theme ID; see --list-themes.
    #[arg(long, default_value = "monoprint")]
    pub theme: String,
    /// Use the theme's light variant.
    #[arg(long, conflicts_with = "dark")]
    pub light: bool,
    /// Use the theme's dark variant (the default when available).
    #[arg(long)]
    pub dark: bool,
    /// Remove the decorative background, preserving the code window.
    #[arg(long, conflicts_with = "background")]
    pub transparent: bool,
    /// Outer canvas: theme, solid, lattice, halftone, or none.
    #[arg(long)]
    pub background: Option<monoprint::Background>,
    #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(0..=512))]
    pub padding: u32,
    /// Outer image width in CSS pixels; code wraps to fit.
    #[arg(long, value_parser = clap::value_parser!(u32).range(80..=8192))]
    pub width: Option<u32>,
    /// Code font size in CSS pixels; line height and line numbers scale with it.
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u32).range(8..=72))]
    pub font_size: u32,
    /// PNG pixel ratio (SVG and HTML retain CSS dimensions).
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(1..=6))]
    pub scale: u32,
    /// Lossless PNG compression: fast (default) or balanced (smaller files).
    #[arg(long, value_name = "MODE")]
    pub png_compression: Option<monoprint::PngCompression>,
    #[arg(long, default_value = "")]
    pub title: String,
    /// Legacy subtitle option; nonempty values are no longer supported.
    #[arg(long, default_value = "", hide = true)]
    pub subtitle: String,
    /// Language ID or alias; otherwise infer from the filename.
    #[arg(short, long)]
    pub language: Option<String>,
    /// Show line numbers in any theme (except for plain text).
    #[arg(long, conflicts_with = "no_line_numbers")]
    pub line_numbers: bool,
    /// Hide line numbers in any theme. Without either flag, use the theme default.
    #[arg(long)]
    pub no_line_numbers: bool,
    /// Highlight source lines, e.g. 1,3-5.
    #[arg(long, default_value = "")]
    pub highlight: String,
    /// Highlight exact text: one-based line:column-line:column, end exclusive.
    /// Repeat for multiple ranges. Columns count Unicode characters, not bytes.
    #[arg(long, value_name = "RANGE")]
    pub highlight_text: Vec<monoprint::TextRange>,
    /// Bundled code typeface ID; see --list-fonts. Defaults to the theme's font.
    #[arg(long, conflicts_with = "font_file")]
    pub font: Option<monoprint::BundledFont>,
    /// Bundled comment typeface ID; defaults to the code font.
    #[arg(long, conflicts_with = "comment_font_file")]
    pub comment_font: Option<monoprint::BundledFont>,
    /// Override the code font with a locally supplied TTF or OTF font.
    #[arg(long)]
    pub font_file: Option<PathBuf>,
    /// Separate TTF/OTF typeface for grammar-recognized comments and docstrings.
    #[arg(long)]
    pub comment_font_file: Option<PathBuf>,
    /// Enable coding ligatures supported by the selected fonts (on by default).
    #[arg(long, conflicts_with = "no_ligatures")]
    pub ligatures: bool,
    /// Disable coding ligatures.
    #[arg(long)]
    pub no_ligatures: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long, conflicts_with = "list_languages")]
    pub list_themes: bool,
    #[arg(long)]
    pub list_languages: bool,
    /// List bundled typeface IDs and family names.
    #[arg(long, conflicts_with_all = ["list_themes", "list_languages"])]
    pub list_fonts: bool,
}

impl Args {
    pub fn output_format(&self) -> Result<Format> {
        let output = self
            .output
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--output is required"))?;
        if output.as_os_str() == "-" {
            return self
                .format
                .ok_or_else(|| anyhow::anyhow!("stdout requires --format png, svg, or html"));
        }
        let inferred = match output
            .extension()
            .and_then(|x| x.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => Some(Format::Png),
            Some("svg") => Some(Format::Svg),
            Some("html" | "htm") => Some(Format::Html),
            _ => None,
        };
        if let (Some(explicit), Some(inferred)) = (self.format, inferred)
            && explicit != inferred
        {
            bail!("--format conflicts with the output extension");
        }
        self.format
            .or(inferred)
            .ok_or_else(|| anyhow::anyhow!("use a .png/.svg/.html extension or specify --format"))
    }

    pub fn highlighted_ranges(&self) -> Result<Vec<LineRange>> {
        let mut result = Vec::new();
        if self.highlight.is_empty() {
            return Ok(result);
        }
        for item in self.highlight.split(',') {
            let item = item.trim();
            let (start, end) = item.split_once('-').unwrap_or((item, item));
            let start: usize = start
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid highlighted line: {item}"))?;
            let end: usize = end
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid highlighted line: {item}"))?;
            result.push((start..=end).into());
        }
        Ok(result)
    }
}
