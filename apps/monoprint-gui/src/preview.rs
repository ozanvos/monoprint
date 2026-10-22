use crate::document::{Document, FontCache, SessionStore};
use gpui_kit::RenderImage;
use monoprint::{Renderer, Scale};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

pub struct Preview {
    pub image: Arc<RenderImage>,
    pub width: u32,
    pub height: u32,
}
const LIVE_REFRESH_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy)]
enum Refresh {
    Debounced,
    Live,
    Immediate,
}
pub struct Completion {
    pub revision: u64,
    pub result: Result<Preview, String>,
    pub session_error: Option<String>,
}
#[derive(Default)]
struct Pending {
    latest: Option<(u64, Document, Refresh)>,
    closed: bool,
}

/// One worker, one replaceable pending request. Rendering never blocks the UI.
pub struct PreviewWorker {
    pending: Arc<(Mutex<Pending>, Condvar)>,
}
impl PreviewWorker {
    pub fn new(
        renderer: Renderer,
        session: Arc<SessionStore>,
    ) -> (Self, async_channel::Receiver<Completion>) {
        let pending = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let shared = pending.clone();
        let (tx, rx) = async_channel::bounded(1);
        thread::Builder::new()
            .name("monoprint-preview".into())
            .spawn(move || {
                let mut fonts = FontCache::default();
                let mut comment_fonts = FontCache::default();
                let mut last_completed: Option<Instant> = None;
                loop {
                    let (lock, wake) = &*shared;
                    let mut state = lock.lock().unwrap();
                    while state.latest.is_none() && !state.closed {
                        state = wake.wait(state).unwrap();
                    }
                    if state.closed {
                        break;
                    }
                    // Keep the displayed frame a little longer during a drag.
                    // New sizes replace the pending one without resetting the
                    // refresh deadline, so continuous dragging still reflows.
                    loop {
                        let delay = match state.latest.as_ref().unwrap().2 {
                            Refresh::Debounced => Duration::from_millis(90),
                            Refresh::Live => last_completed
                                .map(|last| LIVE_REFRESH_INTERVAL.saturating_sub(last.elapsed()))
                                .unwrap_or_default(),
                            Refresh::Immediate => Duration::ZERO,
                        };
                        if delay.is_zero() {
                            break;
                        }
                        let (next, timeout) = wake.wait_timeout(state, delay).unwrap();
                        state = next;
                        if state.closed {
                            return;
                        }
                        if timeout.timed_out() {
                            break;
                        }
                    }
                    let (revision, document, _) = state.latest.take().unwrap();
                    drop(state);
                    let session_error = session
                        .save(revision, &document)
                        .err()
                        .map(|e| e.to_string());
                    let result = (|| -> anyhow::Result<Preview> {
                        let font = fonts.load_document(&document, false)?;
                        let comments = comment_fonts.load_document(&document, true)?;
                        let (snippet, style) = document.prepare(font, comments)?;
                        let raster = renderer.bgra(&snippet, &style, Scale::X2)?;
                        let (width, height) = (raster.width(), raster.height());
                        let image = render_image(raster);
                        Ok(Preview {
                            image,
                            width,
                            height,
                        })
                    })()
                    .map_err(|e| format!("{e:#}"));
                    last_completed = Some(Instant::now());
                    if tx
                        .send_blocking(Completion {
                            revision,
                            result,
                            session_error,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("start preview worker");
        (Self { pending }, rx)
    }
    pub fn request(&self, revision: u64, document: Document) {
        self.enqueue(revision, document, Refresh::Debounced);
    }
    pub fn request_live(&self, revision: u64, document: Document) {
        self.enqueue(revision, document, Refresh::Live);
    }
    pub fn finish_live(&self) {
        let (lock, wake) = &*self.pending;
        if let Some((_, _, refresh)) = &mut lock.lock().unwrap().latest {
            *refresh = Refresh::Immediate;
        }
        wake.notify_one();
    }
    fn enqueue(&self, revision: u64, document: Document, refresh: Refresh) {
        let (lock, wake) = &*self.pending;
        lock.lock().unwrap().latest = Some((revision, document, refresh));
        wake.notify_one();
    }
}

/// GPUI takes ownership of the final BGRA allocation without conversion.
fn render_image(raster: monoprint::BgraImage) -> Arc<RenderImage> {
    let (width, height) = (raster.width(), raster.height());
    let pixels = raster.into_pixels();
    let buffer = image::RgbaImage::from_raw(width, height, pixels)
        .expect("renderer guarantees width × height × 4 bytes");
    Arc::new(RenderImage::new([image::Frame::new(buffer)]))
}
impl Drop for PreviewWorker {
    fn drop(&mut self) {
        let (lock, wake) = &*self.pending;
        lock.lock().unwrap().closed = true;
        wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Image, ImageFormat, SvgRenderer};
    use monoprint::{Appearance, Snippet, Style, Theme};

    fn png_preview(
        renderer: &Renderer,
        snippet: &Snippet,
        style: &Style,
        decoder: &SvgRenderer,
    ) -> Arc<RenderImage> {
        Image::from_bytes(
            ImageFormat::Png,
            renderer.png(snippet, style, Scale::X2).unwrap(),
        )
        .to_image_data(decoder.clone())
        .unwrap()
    }

    #[test]
    fn raw_previews_match_png_decode_for_every_theme_appearance() {
        let decoder = SvgRenderer::new(Arc::new(()));
        let renderer = Renderer::new();
        let snippet = Snippet::new("// Preview pixels\nlet value = 42;\nlet ok = value != 0;")
            .language("rust".parse().unwrap())
            .title("example.rs")
            .highlight(2);
        for theme in Theme::all() {
            for appearance in [Appearance::Dark, Appearance::Light] {
                if !theme.supports(appearance) {
                    continue;
                }
                let style = Style::new(theme)
                    .appearance(appearance)
                    .width(640)
                    .padding(32);
                let expected = png_preview(&renderer, &snippet, &style, &decoder);
                let actual = render_image(renderer.bgra(&snippet, &style, Scale::X2).unwrap());
                assert_eq!(actual.size(0), expected.size(0));
                assert_eq!(
                    actual.as_bytes(0),
                    expected.as_bytes(0),
                    "{theme}/{appearance}"
                );
            }
        }
        // Partial alpha, color emoji, fallback glyphs and independently styled comments.
        let snippet =
            Snippet::new("// 中文 👩🏽‍💻\nlet value = \"❤️ 👋🏽\";").language("rust".parse().unwrap());
        let style = Style::default()
            .transparent(true)
            .font_size(23)
            .comment_font(
                "space-mono"
                    .parse::<monoprint::BundledFont>()
                    .unwrap()
                    .font(),
            );
        let expected = png_preview(&renderer, &snippet, &style, &decoder);
        let actual = render_image(renderer.bgra(&snippet, &style, Scale::X2).unwrap());
        assert_eq!(actual.as_bytes(0), expected.as_bytes(0));
    }

    #[test]
    #[ignore = "Manual timing; run alone with --release --ignored --nocapture"]
    fn benchmark_preview_transport() {
        let decoder = SvgRenderer::new(Arc::new(()));
        let renderer = Renderer::new();
        let mut report = Vec::new();
        for (lines, columns) in [(10, 60), (30, 80), (100, 100)] {
            let source = (0..lines)
                .map(|i| {
                    let line =
                        format!("let value_{i:03} = \"hello {i}\"; // benchmark code image ");
                    format!("{line:.<columns$}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            let snippet = Snippet::new(source).language("rust".parse().unwrap());
            for transparent in [false, true] {
                let style = Style::default()
                    .width(columns as u32 * 13 + 192)
                    .transparent(transparent);
                let render = |mode| match mode {
                    0 => png_preview(&renderer, &snippet, &style, &decoder),
                    1 => {
                        // The committed pre-fusion path, including the same in-place swap.
                        let raster = renderer.rgba(&snippet, &style, Scale::X2).unwrap();
                        let (width, height) = (raster.width(), raster.height());
                        let pixels = raster.into_bgra_pixels();
                        Arc::new(RenderImage::new([image::Frame::new(
                            image::RgbaImage::from_raw(width, height, pixels).unwrap(),
                        )]))
                    }
                    2 => render_image(renderer.bgra(&snippet, &style, Scale::X2).unwrap()),
                    _ => unreachable!(),
                };
                let expected = render(0);
                for mode in [1, 2] {
                    assert_eq!(render(mode).as_bytes(0), expected.as_bytes(0));
                }
                let mut timings = [Vec::new(), Vec::new(), Vec::new()];
                // Rotate all six permutations to balance cache/order effects.
                let orders = [
                    [0, 1, 2],
                    [2, 1, 0],
                    [1, 2, 0],
                    [0, 2, 1],
                    [2, 0, 1],
                    [1, 0, 2],
                ];
                for sample in 0..31 {
                    for mode in orders[sample % orders.len()] {
                        let start = Instant::now();
                        let image = std::hint::black_box(render(mode));
                        let ms = start.elapsed().as_secs_f64() * 1000.;
                        assert_eq!(image.as_bytes(0), expected.as_bytes(0));
                        timings[mode].push(ms);
                    }
                }
                let medians = timings.each_ref().map(|times| {
                    let mut sorted = times.clone();
                    sorted.sort_by(f64::total_cmp);
                    sorted[15]
                });
                eprintln!(
                    "{lines}x{columns} @2x transparent={transparent}: PNG {:.3} ms, separate {:.3} ms, fused {:.3} ms; {:.1}% less vs separate",
                    medians[0],
                    medians[1],
                    medians[2],
                    (1. - medians[2] / medians[1]) * 100.
                );
                report.push(serde_json::json!({
                    "lines": lines, "columns": columns, "transparent": transparent,
                    "width": expected.size(0).width.0, "height": expected.size(0).height.0,
                    "samples_ms": {"png": timings[0], "separate": timings[1], "fused": timings[2]},
                    "median_ms": {"png": medians[0], "separate": medians[1], "fused": medians[2]},
                    "pixel_identical": true,
                }));
            }
        }
        if let Ok(path) = std::env::var("MONOPRINT_PREVIEW_BENCH_REPORT") {
            std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        }
    }

    #[test]
    #[ignore = "Manual A/B timing; run alone with --release --ignored --nocapture"]
    fn benchmark_native_highlight_preview_export() {
        use crate::document::{self, Format};
        use monoprint::HighlightBackend;
        let automatic = Renderer::new();
        let giallo = automatic
            .clone()
            .highlight_backend(HighlightBackend::Giallo);
        let renderers = [&automatic, &giallo];
        let decoder = SvgRenderer::new(Arc::new(()));
        let mut report = Vec::new();
        for (language, lines, transparent, comment_font) in [
            ("rust", 10, false, false),
            ("rust", 30, false, false),
            ("rust", 100, false, false),
            ("typescript", 30, false, false),
            ("typescript", 100, false, false),
            ("python", 30, false, false),
            ("hem", 30, false, false),
            ("rust", 30, true, false),
            ("rust", 30, false, true),
            ("python", 30, false, true),
        ] {
            let source = (0..lines)
                .map(|i| match language {
                    "typescript" => format!("const value{i:03}: string = `hello {i:03}`; // code"),
                    "python" => format!("value_{i:03} = 'hello {i:03}'  # code"),
                    _ => format!("let value_{i:03} = \"hello {i:03}\"; // code"),
                })
                .collect::<Vec<_>>()
                .join("\n");
            let document = Document {
                source,
                language: language.into(),
                transparent,
                comment_font_id: comment_font.then(|| "space-mono".to_owned()),
                ..Default::default()
            };
            let preview = |renderer: &Renderer| {
                let font = FontCache::default()
                    .load_document(&document, false)
                    .unwrap();
                let comments = FontCache::default().load_document(&document, true).unwrap();
                let (snippet, style) = document.prepare(font, comments).unwrap();
                render_image(renderer.bgra(&snippet, &style, Scale::X2).unwrap())
            };
            let expected_png =
                renderers.map(|r| document::export(r, &document, Format::Png).unwrap());
            let expected = expected_png.each_ref().map(|png| {
                Image::from_bytes(ImageFormat::Png, png.clone())
                    .to_image_data(decoder.clone())
                    .unwrap()
            });
            assert_eq!(
                expected[0].size(0),
                expected[1].size(0),
                "A/B pixel workload must have identical dimensions"
            );
            for backend in 0..2 {
                assert_eq!(
                    preview(renderers[backend]).as_bytes(0),
                    expected[backend].as_bytes(0)
                );
            }
            let mut previews = [Vec::new(), Vec::new()];
            let mut exports = [Vec::new(), Vec::new()];
            let mut svg = [Vec::new(), Vec::new()];
            let mut html = [Vec::new(), Vec::new()];
            // Alternate both backend and preview/export order. Validation and
            // image comparison are deliberately outside each timed interval.
            for sample in 0..15 {
                let order = if sample % 2 == 0 { [0, 1] } else { [1, 0] };
                for backend in order {
                    let r = renderers[backend];
                    for operation in if sample % 3 == 0 { [1, 0] } else { [0, 1] } {
                        let start = Instant::now();
                        if operation == 0 {
                            let image = std::hint::black_box(preview(r));
                            previews[backend].push(start.elapsed().as_secs_f64() * 1000.);
                            assert_eq!(image.as_bytes(0), expected[backend].as_bytes(0));
                        } else {
                            let png = std::hint::black_box(
                                document::export(r, &document, Format::Png).unwrap(),
                            );
                            exports[backend].push(start.elapsed().as_secs_f64() * 1000.);
                            assert_eq!(png, expected_png[backend]);
                        }
                    }
                    for (format, timings) in [(Format::Svg, &mut svg), (Format::Html, &mut html)] {
                        let start = Instant::now();
                        std::hint::black_box(document::export(r, &document, format).unwrap());
                        timings[backend].push(start.elapsed().as_secs_f64() * 1000.);
                    }
                }
            }
            let median = |samples: &[f64]| {
                let mut sorted = samples.to_vec();
                sorted.sort_by(f64::total_cmp);
                sorted[sorted.len() / 2]
            };
            let medians = |samples: &[Vec<f64>; 2]| serde_json::json!({"auto":median(&samples[0]),"giallo":median(&samples[1])});
            eprintln!(
                "{language}/{lines} transparent={transparent} comments={comment_font}: preview {:.3} -> {:.3} ms; PNG {:.3} -> {:.3} ms",
                median(&previews[1]),
                median(&previews[0]),
                median(&exports[1]),
                median(&exports[0])
            );
            report.push(serde_json::json!({
                "language":language,"lines":lines,"source_bytes":document.source.len(),"transparent":transparent,"comment_font":comment_font,
                "width":expected[0].size(0).width.0,"height":expected[0].size(0).height.0,
                "source":document.source,"theme":document.theme,"font_size":document.font_size,"outer_width":document.width,"padding":document.padding,
                "scale":document.scale,"png_compression":document.png_compression,
                "preview_matches_png":true,"same_pixels_between_backends":expected[0].as_bytes(0)==expected[1].as_bytes(0),
                "median_ms":{"preview":medians(&previews),"png":medians(&exports),"svg":medians(&svg),"html":medians(&html)},
                "samples_ms":{"preview":previews,"png":exports,"svg":svg,"html":html},
            }));
            if let Ok(dir) = std::env::var("MONOPRINT_HIGHLIGHT_BENCH_IMAGES") {
                std::fs::create_dir_all(&dir).unwrap();
                for (backend, png) in expected_png.iter().enumerate() {
                    std::fs::write(
                        format!(
                            "{dir}/{language}-{lines}-{transparent}-{comment_font}-{}.png",
                            ["auto", "giallo"][backend]
                        ),
                        png,
                    )
                    .unwrap();
                }
            }
        }
        let path = std::env::var("MONOPRINT_HIGHLIGHT_BENCH_REPORT").expect("report output path");
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }

    #[test]
    fn rapid_edits_render_only_the_latest_snapshot() {
        let (worker, results) =
            PreviewWorker::new(Renderer::new(), Arc::new(SessionStore::new(None)));
        for revision in 1..=25 {
            worker.request(
                revision,
                Document {
                    source: format!("const n = {revision};"),
                    ..Default::default()
                },
            );
        }
        let completion = results.recv_blocking().unwrap();
        assert_eq!(completion.revision, 25);
        let preview = completion.result.unwrap();
        assert_eq!(preview.image.frame_count(), 1);
        assert_eq!(
            preview.image.as_bytes(0).unwrap().len(),
            preview.width as usize * preview.height as usize * 4
        );
        assert!(results.try_recv().is_err());
    }
}
