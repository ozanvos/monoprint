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
    pub elapsed: Duration,
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
                    let start = Instant::now();
                    let result = (|| -> anyhow::Result<Preview> {
                        let font = fonts.load_selection(
                            document.font_id.as_deref(),
                            document.font_path.as_deref(),
                        )?;
                        let comments = comment_fonts.load_selection(
                            document.comment_font_id.as_deref(),
                            document.comment_font_path.as_deref(),
                        )?;
                        let (snippet, style) = document.prepare(font, comments)?;
                        let raster = renderer.rgba(&snippet, &style, Scale::X2)?;
                        let (width, height) = (raster.width(), raster.height());
                        let image = render_image(raster);
                        Ok(Preview {
                            image,
                            width,
                            height,
                            elapsed: start.elapsed(),
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

/// GPUI takes straight-alpha BGRA. Swap channels in the owned raster allocation,
/// then move it into the frame; no PNG round trip or second pixel buffer.
fn render_image(raster: monoprint::RasterImage) -> Arc<RenderImage> {
    let (width, height) = (raster.width(), raster.height());
    let pixels = raster.into_bgra_pixels();
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
                let actual = render_image(renderer.rgba(&snippet, &style, Scale::X2).unwrap());
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
        let actual = render_image(renderer.rgba(&snippet, &style, Scale::X2).unwrap());
        assert_eq!(actual.as_bytes(0), expected.as_bytes(0));
    }

    #[test]
    #[ignore = "Manual timing; run alone with --release --ignored --nocapture"]
    fn benchmark_preview_transport() {
        let decoder = SvgRenderer::new(Arc::new(()));
        let renderer = Renderer::new();
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
            let style = Style::default().width(columns as u32 * 13 + 192);
            let render = |raw| {
                if raw {
                    render_image(renderer.rgba(&snippet, &style, Scale::X2).unwrap())
                } else {
                    png_preview(&renderer, &snippet, &style, &decoder)
                }
            };
            let expected = render(false);
            assert_eq!(render(true).as_bytes(0), expected.as_bytes(0));
            let mut timings = [Vec::new(), Vec::new()];
            for sample in 0..15 {
                for mode in if sample % 2 == 0 { [0, 1] } else { [1, 0] } {
                    let start = Instant::now();
                    let image = std::hint::black_box(render(mode == 1));
                    let ms = start.elapsed().as_secs_f64() * 1000.;
                    assert_eq!(image.as_bytes(0), expected.as_bytes(0));
                    timings[mode].push(ms);
                }
            }
            for times in &mut timings {
                times.sort_by(f64::total_cmp);
            }
            eprintln!(
                "{lines}x{columns} @2x: PNG {:.2} ms, raw {:.2} ms; {:.1}% faster (median of 15)",
                timings[0][7],
                timings[1][7],
                (1. - timings[1][7] / timings[0][7]) * 100.
            );
        }
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
