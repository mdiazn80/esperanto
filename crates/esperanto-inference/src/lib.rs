//! Inference providers for Esperanto.
//!
//! Local (`llama-server` sidecar) and remote (API gateway with a token)
//! providers speak the same OpenAI-compatible protocol, so a single client
//! covers both.

pub mod openai;
pub mod prompt;
pub mod wav;

use std::future::Future;

use esperanto_core::{AudioSegment, Language, SourceLanguage, SpeechResult, SpeechTask};

pub use openai::{ConnectionReport, OpenAiCompatibleProvider, OpenAiConfig};
pub use prompt::Transcript;

/// Errors keep the raw server body or model answer for callers that need
/// it, but their `Display` (what ends up in logs and in messages to the
/// frontend) only shows a short preview of server bodies and never the
/// model's answer, which may contain transcribed speech.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server answered {status}: {}", preview(body))]
    Status { status: u16, body: String },
    #[error("the model's answer could not be used ({} characters)", .0.chars().count())]
    InvalidAnswer(String),
}

/// First characters of a server error body, on one line.
fn preview(body: &str) -> String {
    const MAX: usize = 200;
    let line: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    match line.char_indices().nth(MAX) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line,
    }
}

/// Speech-to-text and translation (two steps).
pub trait InferenceProvider: Send + Sync {
    /// Transcribes `audio` in the spoken language.
    fn transcribe(
        &self,
        audio: &AudioSegment,
        source: SourceLanguage,
    ) -> impl Future<Output = Result<Transcript, InferenceError>> + Send;

    /// Translates text.
    fn translate(
        &self,
        text: &str,
        from: Language,
        to: Language,
    ) -> impl Future<Output = Result<String, InferenceError>> + Send;
}

/// Runs a whole [`SpeechTask`]: transcription, then translation when the
/// target differs from the spoken language.
pub async fn run_task<P: InferenceProvider>(
    provider: &P,
    task: &SpeechTask,
) -> Result<SpeechResult, InferenceError> {
    let transcript = provider
        .transcribe(&task.audio, task.source_language)
        .await?;
    let translation = match transcript.language.and_then(|from| {
        esperanto_core::translation_target(task.target_language, from).map(|to| (from, to))
    }) {
        Some((from, to)) => Some(provider.translate(&transcript.text, from, to).await?),
        None => None,
    };
    Ok(SpeechResult {
        transcript: transcript.text,
        detected_language: transcript.language,
        translation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_never_display_the_answer_and_shorten_bodies() {
        let answer = InferenceError::InvalidAnswer("hola, esto es privado".into());
        assert!(!answer.to_string().contains("privado"));
        let status = InferenceError::Status {
            status: 500,
            body: format!("<html>\n{}</html>", "x".repeat(1000)),
        };
        let shown = status.to_string();
        assert!(shown.len() < 260 && shown.ends_with('…'), "{shown}");
    }
}
