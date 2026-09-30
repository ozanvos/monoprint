//! Files exchanged with other macOS apps. Rendering stays in the existing renderer.
use crate::document::{self, Document, Format};
use anyhow::{Result, bail};
use monoprint::Renderer;
use std::path::PathBuf;

#[derive(Clone)]
pub struct DragExport {
    pub revision: u64,
    pub path: PathBuf,
}

/// Keep the file after a drop, document edit, or app exit: recipients may read it later.
/// The operating system manages these files in its per-user temporary directory.
pub fn stage_png(renderer: &Renderer, document: &Document, revision: u64) -> Result<DragExport> {
    let bytes = document::export(renderer, document, Format::Png)?;
    let directory = tempfile::Builder::new()
        .prefix("monoprint-drag-")
        .tempdir()?;
    let mut png = document.clone();
    png.format = Format::Png;
    let path = directory.path().join(png.suggested_name(directory.path())?);
    document::write_atomic(&path, &bytes)?;
    let _ = directory.keep();
    Ok(DragExport { revision, path })
}

/// Decode Launch Services URLs, including escaped spaces and non-ASCII filenames.
pub fn source_url(value: &str) -> Result<PathBuf> {
    let url = url::Url::parse(value)?;
    match url.to_file_path() {
        Ok(path) => Ok(path),
        Err(_) => bail!("Open a local UTF-8 source file."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_urls_decode_without_interpreting_remote_locations() {
        let path = PathBuf::from("/tmp/Monoprint examples/示例 #1.ts");
        let url = url::Url::from_file_path(&path).unwrap();
        assert_eq!(source_url(url.as_str()).unwrap(), path);
        assert!(source_url("https://example.com/code.ts").is_err());
        assert!(source_url("file://example.com/code.ts").is_err());
        assert!(source_url("not a URL").is_err());
    }

    #[test]
    fn staged_drag_is_full_export_and_survives_cache_invalidation() {
        let renderer = Renderer::new();
        let document = Document {
            title: "example.ts".into(),
            format: Format::Svg,
            scale: 1,
            ..Document::default()
        };
        let staged = stage_png(&renderer, &document, 42).unwrap();
        assert_eq!(staged.revision, 42);
        assert_eq!(staged.path.file_name().unwrap(), "example.png");
        let path = staged.path.clone();
        drop(staged);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            document::export(&renderer, &document, Format::Png).unwrap()
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
