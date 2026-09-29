use crate::document::{Document, FontCache, SessionStore};
use gpui_kit::{Image, ImageFormat, RenderImage, SvgRenderer};
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
                let decoder = SvgRenderer::new(Arc::new(()));
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
                        let png = renderer.png(&snippet, &style, Scale::X2)?;
                        // Decode before publishing: GPUI can paint this image
                        // immediately, without an empty async-loading frame.
                        let image = Image::from_bytes(ImageFormat::Png, png)
                            .to_image_data(decoder.clone())?;
                        let dimensions = image.size(0);
                        Ok(Preview {
                            image,
                            width: dimensions.width.0.try_into()?,
                            height: dimensions.height.0.try_into()?,
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
