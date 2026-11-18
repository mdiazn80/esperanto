//! The inference provider a session uses, local or remote.
//!
//! If the local `llama-server` dies mid-session (it can crash, or be killed
//! by the system under memory pressure), it is restarted and the request
//! retried once. Remote providers have nothing to restart.

use std::sync::Arc;

use esperanto_core::{AudioSegment, Language, SourceLanguage};
use esperanto_inference::{
    InferenceError, InferenceProvider, OpenAiCompatibleProvider, Transcript,
};
use esperanto_models::ModelEntry;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::error::ErrorCode;
use crate::models::ModelsState;
use crate::{providers, server};

pub struct Engine {
    app: AppHandle,
    /// The local model, restarted if needed; `None` for a remote provider.
    local: Option<ModelEntry>,
    provider: Mutex<Arc<OpenAiCompatibleProvider>>,
}

impl Engine {
    pub fn local(app: AppHandle, model: ModelEntry, provider: OpenAiCompatibleProvider) -> Self {
        Self {
            app,
            local: Some(model),
            provider: Mutex::new(Arc::new(provider)),
        }
    }

    pub fn remote(app: AppHandle, provider: OpenAiCompatibleProvider) -> Self {
        Self {
            app,
            local: None,
            provider: Mutex::new(Arc::new(provider)),
        }
    }

    /// Code reported to the UI for a failed request.
    pub fn error_code(&self, error: &InferenceError) -> ErrorCode {
        if self.local.is_some() {
            ErrorCode::InferenceFailed
        } else {
            providers::inference_error_code(error)
        }
    }

    pub async fn transcribe(
        &self,
        audio: &AudioSegment,
        source: SourceLanguage,
    ) -> Result<Transcript, InferenceError> {
        let provider = self.current().await;
        match provider.transcribe(audio, source).await {
            Err(error) if is_unreachable(&error) => match self.restart(&provider).await {
                Some(fresh) => fresh.transcribe(audio, source).await,
                None => Err(error),
            },
            other => other,
        }
    }

    pub async fn translate(
        &self,
        text: &str,
        from: Language,
        to: Language,
    ) -> Result<String, InferenceError> {
        let provider = self.current().await;
        match provider.translate(text, from, to).await {
            Err(error) if is_unreachable(&error) => match self.restart(&provider).await {
                Some(fresh) => fresh.translate(text, from, to).await,
                None => Err(error),
            },
            other => other,
        }
    }

    async fn current(&self) -> Arc<OpenAiCompatibleProvider> {
        Arc::clone(&*self.provider.lock().await)
    }

    /// Restarts the local server unless another worker already did.
    async fn restart(
        &self,
        failed: &Arc<OpenAiCompatibleProvider>,
    ) -> Option<Arc<OpenAiCompatibleProvider>> {
        let model = self.local.as_ref()?;
        let mut provider = self.provider.lock().await;
        if !Arc::ptr_eq(&provider, failed) {
            return Some(Arc::clone(&provider));
        }
        log::warn!("llama-server unreachable; restarting it");
        let models = self.app.state::<ModelsState>();
        match server::provider(&self.app, models.store(), model).await {
            Ok(fresh) => {
                *provider = Arc::new(fresh);
                Some(Arc::clone(&provider))
            }
            Err(error) => {
                log::error!("could not restart llama-server: {error}");
                None
            }
        }
    }
}

/// The server can't be reached at all (not a slow or failed request: those
/// aren't fixed by restarting it).
fn is_unreachable(error: &InferenceError) -> bool {
    matches!(error, InferenceError::Http(e) if e.is_connect())
}
