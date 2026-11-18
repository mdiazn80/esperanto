//! Client for OpenAI-compatible servers: the local
//! `llama-server` sidecar and remote gateways use the same protocol.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use esperanto_core::{AudioSegment, Language, SourceLanguage};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{InferenceError, InferenceProvider, Transcript, prompt, wav};

#[derive(Clone)]
pub struct OpenAiConfig {
    /// Base URL including the API prefix, e.g. `http://127.0.0.1:8080/v1`.
    pub base_url: String,
    /// Sent as `Authorization: Bearer <token>` when set.
    pub api_token: Option<String>,
    pub model: String,
    pub timeout: Duration,
    /// Send `response_format` with a JSON schema (constrained decoding).
    pub structured_output: bool,
}

impl OpenAiConfig {
    /// Default request timeout: generous for a short phrase, short enough
    /// that a stuck server is noticed.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_token: None,
            model: model.into(),
            timeout: Self::DEFAULT_TIMEOUT,
            structured_output: true,
        }
    }
}

/// Never prints the token.
impl std::fmt::Debug for OpenAiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiConfig")
            .field("base_url", &self.base_url)
            .field("api_token", &self.api_token.as_ref().map(|_| "<redacted>"))
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .field("structured_output", &self.structured_output)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    config: OpenAiConfig,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: OpenAiConfig) -> Result<Self, InferenceError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            // The token must only go to the configured server.
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { config, http })
    }

    /// Sends one chat request and returns the answer's text.
    async fn chat(
        &self,
        content: Value,
        schema: Value,
        max_tokens: u32,
    ) -> Result<String, InferenceError> {
        let mut body = json!({
            "model": self.config.model,
            "temperature": 0,
            // Models sometimes loop on noise or music ("or have to or have
            // to…"); without a cap one request ran for a minute.
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": content }],
        });
        if self.config.structured_output {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": { "name": "answer", "schema": schema },
            });
        }

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let mut request = self.http.post(url).json(&body);
        if let Some(token) = &self.config.api_token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let chat: ChatResponse = response.json().await?;
        Ok(chat
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default())
    }
}

/// Result of [`OpenAiCompatibleProvider::check`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionReport {
    /// Models the server lists (`GET /models`); `None` if it doesn't
    /// support listing.
    pub models: Option<Vec<String>>,
    /// Round trip of a short audio request, in milliseconds.
    pub audio_latency_ms: f64,
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelId>,
}

#[derive(Deserialize)]
struct ModelId {
    id: String,
}

impl OpenAiCompatibleProvider {
    /// Checks that the server is reachable, accepts the token and accepts
    /// audio for the configured model.
    pub async fn check(&self) -> Result<ConnectionReport, InferenceError> {
        let url = format!("{}/models", self.config.base_url.trim_end_matches('/'));
        let mut request = self.http.get(url);
        if let Some(token) = &self.config.api_token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        let status = response.status();
        let models = if status.is_success() {
            response
                .json::<ModelList>()
                .await
                .ok()
                .map(|list| list.data.into_iter().map(|m| m.id).collect())
        } else if matches!(status.as_u16(), 401 | 403) {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Status {
                status: status.as_u16(),
                body,
            });
        } else {
            None // Listing is optional for gateways.
        };

        // Half a second of silence: any parsed answer, even an empty one,
        // proves the server takes audio for this model.
        let silence = AudioSegment {
            samples: vec![0.0; AudioSegment::SAMPLE_RATE as usize / 2],
            sample_rate: AudioSegment::SAMPLE_RATE,
        };
        let started = std::time::Instant::now();
        match self
            .transcribe(&silence, SourceLanguage::Fixed(Language::En))
            .await
        {
            Ok(_) | Err(InferenceError::InvalidAnswer(_)) => Ok(ConnectionReport {
                models,
                audio_latency_ms: started.elapsed().as_secs_f64() * 1000.0,
            }),
            Err(error) => Err(error),
        }
    }
}

impl InferenceProvider for OpenAiCompatibleProvider {
    async fn transcribe(
        &self,
        audio: &AudioSegment,
        source: SourceLanguage,
    ) -> Result<Transcript, InferenceError> {
        let data = BASE64.encode(wav::encode_pcm16(audio));
        let content = json!([
            { "type": "input_audio", "input_audio": { "data": data, "format": "wav" } },
            { "type": "text", "text": prompt::transcription_instruction(source) },
        ]);
        let answer = self
            .chat(
                content,
                prompt::transcription_schema(source),
                prompt::transcription_token_budget(audio.duration_secs()),
            )
            .await?;
        prompt::parse_transcription(source, &answer).ok_or(InferenceError::InvalidAnswer(answer))
    }

    async fn translate(
        &self,
        text: &str,
        from: Language,
        to: Language,
    ) -> Result<String, InferenceError> {
        let content = json!(prompt::translation_prompt(text, from, to));
        let answer = self
            .chat(
                content,
                prompt::translation_schema(),
                prompt::translation_token_budget(text),
            )
            .await?;
        prompt::parse_translation(&answer).ok_or(InferenceError::InvalidAnswer(answer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_task;
    use esperanto_core::SpeechTask;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn task(target: Option<Language>) -> SpeechTask {
        SpeechTask {
            audio: AudioSegment {
                samples: vec![0.0; 1_600],
                sample_rate: 16_000,
            },
            source_language: SourceLanguage::Fixed(Language::Es),
            target_language: target,
        }
    }

    fn answer(content: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{ "message": { "role": "assistant", "content": content } }]
        }))
    }

    async fn provider(server: &MockServer, token: Option<&str>) -> OpenAiCompatibleProvider {
        let mut config = OpenAiConfig::new(format!("{}/v1/", server.uri()), "voxtral");
        config.api_token = token.map(str::to_owned);
        OpenAiCompatibleProvider::new(config).unwrap()
    }

    #[tokio::test]
    async fn transcribes_then_translates_the_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer secret"))
            .and(body_string_contains("input_audio"))
            .respond_with(answer(r#"{"transcript":"Hola mundo"}"#))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_string_contains("Spanish text into English"))
            .and(body_string_contains("Hola mundo"))
            .respond_with(answer(r#"{"translation":"Hello world"}"#))
            .expect(1)
            .mount(&server)
            .await;

        let provider = provider(&server, Some("secret")).await;
        let result = run_task(&provider, &task(Some(Language::En)))
            .await
            .unwrap();
        assert_eq!(result.transcript, "Hola mundo");
        assert_eq!(result.detected_language, Some(Language::Es));
        assert_eq!(result.translation.as_deref(), Some("Hello world"));

        let requests = server.received_requests().await.unwrap();
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["input_audio"]["format"], "wav");
        let wav = BASE64
            .decode(content[0]["input_audio"]["data"].as_str().unwrap())
            .unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert!(
            content[1]["text"]
                .as_str()
                .unwrap()
                .contains("Spanish audio")
        );
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["temperature"], 0);
        assert_eq!(body["max_tokens"], 64 + 2); // 0.1 s of audio
    }

    #[tokio::test]
    async fn skips_translation_when_not_needed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(answer(r#"{"transcript":"Hola"}"#))
            .expect(2)
            .mount(&server)
            .await;
        let provider = provider(&server, None).await;
        // No target, and target equal to the spoken language: one request each.
        assert!(
            run_task(&provider, &task(None))
                .await
                .unwrap()
                .translation
                .is_none()
        );
        assert!(
            run_task(&provider, &task(Some(Language::Es)))
                .await
                .unwrap()
                .translation
                .is_none()
        );
    }

    #[tokio::test]
    async fn check_lists_models_and_probes_audio() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", "Bearer secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [{ "id": "voxtral" }, { "id": "other" }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_string_contains("input_audio"))
            .respond_with(answer(r#"{"transcript":""}"#))
            .expect(1)
            .mount(&server)
            .await;
        let report = provider(&server, Some("secret"))
            .await
            .check()
            .await
            .unwrap();
        assert_eq!(
            report.models,
            Some(vec!["voxtral".to_owned(), "other".to_owned()])
        );
        assert!(report.audio_latency_ms >= 0.0);
    }

    #[tokio::test]
    async fn check_tolerates_servers_without_a_model_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(answer(r#"{"transcript":"hi"}"#))
            .mount(&server)
            .await;
        let report = provider(&server, None).await.check().await.unwrap();
        assert_eq!(report.models, None);
    }

    #[tokio::test]
    async fn check_reports_a_rejected_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid api key"))
            .mount(&server)
            .await;
        let error = provider(&server, Some("wrong"))
            .await
            .check()
            .await
            .unwrap_err();
        assert!(matches!(error, InferenceError::Status { status: 401, .. }));
    }

    #[tokio::test]
    async fn check_reports_models_without_audio_support() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(400).set_body_string("audio input is not supported"),
            )
            .mount(&server)
            .await;
        let error = provider(&server, None).await.check().await.unwrap_err();
        assert!(matches!(error, InferenceError::Status { status: 400, .. }));
    }

    #[tokio::test]
    async fn reports_http_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("bad token"))
            .mount(&server)
            .await;
        let provider = provider(&server, None).await;
        let error = run_task(&provider, &task(None)).await.unwrap_err();
        assert!(matches!(error, InferenceError::Status { status: 401, .. }));
    }

    #[tokio::test]
    async fn reports_empty_answers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(answer(r#"{"transcript":""}"#))
            .mount(&server)
            .await;
        let provider = provider(&server, None).await;
        let error = run_task(&provider, &task(None)).await.unwrap_err();
        assert!(matches!(error, InferenceError::InvalidAnswer(_)));
    }
}
