//! Private Node-API adapter. The public interface lives in js/index.ts.
mod errors;
mod options;

use monoprint::{Font, Language, Renderer};
use napi::{Env, Task, bindgen_prelude::*};
use napi_derive::napi;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[napi]
pub struct NativeFont {
    inner: Font,
}

#[napi(object, object_from_js = false)]
pub struct FontResult {
    pub font: Option<NativeFont>,
    pub failure: Option<String>,
}

#[napi]
pub fn create_font(bytes: Uint8Array) -> FontResult {
    let result = if bytes.len() > monoprint::MAX_FONT_BYTES {
        Err(monoprint::InvalidOption::FontTooLarge { len: bytes.len() }.into())
    } else {
        // JS may mutate or detach the input immediately after returning.
        Font::from_bytes(bytes.as_ref())
    };
    match result {
        Ok(inner) => FontResult {
            font: Some(NativeFont { inner }),
            failure: None,
        },
        Err(error) => FontResult {
            font: None,
            failure: Some(errors::encode(error)),
        },
    }
}

#[napi]
pub struct NativeRenderer {
    inner: Renderer,
}

#[napi]
impl NativeRenderer {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Renderer::new(),
        }
    }

    #[napi]
    pub fn render(
        &self,
        source: String,
        options: String,
        font: Option<&NativeFont>,
    ) -> AsyncTask<RenderTask> {
        AsyncTask::new(RenderTask {
            renderer: self.inner.clone(),
            source,
            options,
            font: font.map(|font| font.inner.clone()),
        })
    }

    #[napi]
    pub fn render_sync(
        &self,
        source: String,
        options: String,
        font: Option<&NativeFont>,
    ) -> RenderResult {
        RenderTask {
            renderer: self.inner.clone(),
            source,
            options,
            font: font.map(|font| font.inner.clone()),
        }
        .execute()
        .into()
    }
}

impl Default for NativeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[napi(object, object_from_js = false)]
pub struct RenderResult {
    pub data: Option<Either<Buffer, String>>,
    pub failure: Option<String>,
}

pub enum Output {
    Png(Vec<u8>),
    Text(String),
    Failure(String),
}

impl From<Output> for RenderResult {
    fn from(output: Output) -> Self {
        match output {
            Output::Png(bytes) => Self {
                data: Some(Either::A(bytes.into())),
                failure: None,
            },
            Output::Text(text) => Self {
                data: Some(Either::B(text)),
                failure: None,
            },
            Output::Failure(error) => Self {
                data: None,
                failure: Some(error),
            },
        }
    }
}

pub struct RenderTask {
    renderer: Renderer,
    source: String,
    options: String,
    font: Option<Font>,
}

impl RenderTask {
    fn execute(&self) -> Output {
        // Keep unexpected backend panics from unwinding across the C ABI.
        match catch_unwind(AssertUnwindSafe(|| {
            options::render(
                &self.renderer,
                &self.source,
                &self.options,
                self.font.clone(),
            )
        })) {
            Ok(Ok(output)) => output,
            Ok(Err(failure)) => Output::Failure(failure),
            Err(_) => Output::Failure(errors::internal("native renderer panicked")),
        }
    }
}

impl Task for RenderTask {
    type Output = Output;
    type JsValue = RenderResult;
    fn compute(&mut self) -> napi::Result<Output> {
        Ok(self.execute())
    }
    fn resolve(&mut self, _: Env, output: Output) -> napi::Result<RenderResult> {
        Ok(output.into())
    }
}

#[napi]
pub fn language_for_file_name(filename: String) -> Option<String> {
    Language::for_file_name(filename).map(|language| language.id().to_owned())
}

#[napi]
pub fn catalog() -> String {
    use monoprint::{Appearance, LineNumberPolicy, Theme};
    let themes: Vec<_> = Theme::all()
        .map(|theme| {
            let (toggleable, default) = match theme.line_numbers() {
                LineNumberPolicy::Fixed(value) => (false, value),
                LineNumberPolicy::Toggleable { default } => (true, default),
                _ => (false, false),
            };
            let appearances: Vec<_> = [Appearance::Dark, Appearance::Light]
                .into_iter()
                .filter(|appearance| theme.supports(*appearance))
                .map(|a| a.to_string())
                .collect();
            serde_json::json!({ "id": theme.id(), "name": theme.name(), "appearances": appearances,
            "lineNumbers": { "toggleable": toggleable, "default": default } })
        })
        .collect();
    let languages: Vec<_> = Language::all().map(|language| serde_json::json!({
        "id": language.id(), "name": language.name(), "aliases": language.aliases().collect::<Vec<_>>()
    })).collect();
    serde_json::json!({ "themes": themes, "languages": languages }).to_string()
}
