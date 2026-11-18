//! Instructions sent to the model and parsing of its answers.
//!
//! Two requests per segment: transcription of the audio in the
//! spoken language, then translation of the transcript as text. Asking for
//! both in one request made Voxtral mix up the languages.

use esperanto_core::{Language, SourceLanguage};
use serde::Deserialize;
use serde_json::{Value, json};

/// Instruction that accompanies the audio.
pub fn transcription_instruction(source: SourceLanguage) -> String {
    match source {
        SourceLanguage::Fixed(lang) => format!(
            "Transcribe this {name} audio verbatim, in {name}. Do not translate it.",
            name = lang.english_name()
        ),
        // Kept for evaluation; the app requires a fixed language (D19).
        SourceLanguage::Auto => "Transcribe this audio verbatim, in the language it is spoken \
             (English or Spanish). Do not translate it. Also report the language."
            .to_owned(),
    }
}

/// JSON schema for the transcription answer (`response_format`).
pub fn transcription_schema(source: SourceLanguage) -> Value {
    match source {
        SourceLanguage::Fixed(_) => json!({
            "type": "object",
            "properties": { "transcript": { "type": "string" } },
            "required": ["transcript"],
            "additionalProperties": false,
        }),
        SourceLanguage::Auto => json!({
            "type": "object",
            "properties": {
                "language": { "type": "string", "enum": ["en", "es"] },
                "transcript": { "type": "string" },
            },
            "required": ["language", "transcript"],
            "additionalProperties": false,
        }),
    }
}

/// Prompt for translating a transcript.
pub fn translation_prompt(text: &str, from: Language, to: Language) -> String {
    format!(
        "Translate this {from} text into {to}. Reply with the translation only.\n\n{text}",
        from = from.english_name(),
        to = to.english_name()
    )
}

pub fn translation_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "translation": { "type": "string" } },
        "required": ["translation"],
        "additionalProperties": false,
    })
}

/// Most tokens a transcription may use: fast speech is ~4 words/s, and a
/// word is up to ~2 tokens, plus the JSON around it. Caps runaway outputs.
pub fn transcription_token_budget(audio_secs: f64) -> u32 {
    (64.0 + audio_secs.max(0.0) * 16.0).ceil() as u32
}

/// Most tokens a translation may use: about twice the source's tokens
/// (≈ 4 characters each), plus the JSON around it.
pub fn translation_token_budget(text: &str) -> u32 {
    64 + (text.chars().count() as u32).div_ceil(2)
}

/// A parsed transcription answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub text: String,
    /// The spoken language: the fixed one, or what the model reported.
    pub language: Option<Language>,
}

/// Extracts the JSON object from a model answer, tolerating code fences or
/// text around it.
fn json_object(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    (start < end).then(|| &content[start..=end])
}

/// Whether an answer was meant to be JSON (possibly cut off by `max_tokens`).
/// Such answers are never shown as plain text.
fn looks_like_json(content: &str) -> bool {
    let trimmed = content
        .trim_start()
        .trim_start_matches("```json")
        .trim_start_matches("```");
    trimmed.trim_start().starts_with('{')
}

#[derive(Deserialize)]
struct TranscriptionAnswer {
    transcript: String,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Deserialize)]
struct TranslationAnswer {
    translation: String,
}

/// Parses a transcription. Plain text is accepted for servers that ignore
/// `response_format`. Returns `None` for an empty answer.
pub fn parse_transcription(source: SourceLanguage, content: &str) -> Option<Transcript> {
    let parsed = json_object(content)
        .and_then(|json| serde_json::from_str::<TranscriptionAnswer>(json).ok());
    let (text, reported) = match parsed {
        Some(answer) => (answer.transcript, answer.language),
        None if looks_like_json(content) => return None,
        None => (content.to_owned(), None),
    };
    let text = text.trim().to_owned();
    if text.is_empty() {
        return None;
    }
    let language = match source {
        SourceLanguage::Fixed(lang) => Some(lang),
        SourceLanguage::Auto => reported.as_deref().and_then(Language::from_tag),
    };
    Some(Transcript { text, language })
}

/// Parses a translation (JSON or plain text). `None` if empty.
pub fn parse_translation(content: &str) -> Option<String> {
    let text = match json_object(content)
        .and_then(|json| serde_json::from_str::<TranslationAnswer>(json).ok())
    {
        Some(answer) => answer.translation,
        None if looks_like_json(content) => return None,
        None => content.to_owned(),
    };
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ES: SourceLanguage = SourceLanguage::Fixed(Language::Es);

    #[test]
    fn transcription_instruction_names_the_language() {
        let text = transcription_instruction(ES);
        assert!(text.contains("Spanish audio") && text.contains("Do not translate"));
    }

    #[test]
    fn auto_schema_also_asks_for_the_language() {
        assert_eq!(
            transcription_schema(SourceLanguage::Auto)["required"],
            json!(["language", "transcript"])
        );
        assert_eq!(transcription_schema(ES)["required"], json!(["transcript"]));
    }

    #[test]
    fn parses_transcriptions() {
        let t = parse_transcription(ES, "```json\n{\"transcript\": \" Hola \"}\n```").unwrap();
        assert_eq!(t.text, "Hola");
        assert_eq!(t.language, Some(Language::Es));

        let t = parse_transcription(
            SourceLanguage::Auto,
            r#"{"language":"en","transcript":"Hi"}"#,
        )
        .unwrap();
        assert_eq!(t.language, Some(Language::En));

        assert_eq!(parse_transcription(ES, "Hola").unwrap().text, "Hola");
        assert!(parse_transcription(ES, r#"{"transcript":"  "}"#).is_none());
    }

    #[test]
    fn rejects_answers_cut_off_by_the_token_limit() {
        assert!(parse_transcription(ES, r#"{"transcript": "hola que tal"#).is_none());
        assert!(parse_transcription(ES, "```json\n{\"transcript\": \"hola").is_none());
        assert!(parse_translation(r#"{"translation": "hello there"#).is_none());
    }

    #[test]
    fn parses_translations() {
        assert_eq!(
            parse_translation(r#"{"translation":"Hello"}"#).as_deref(),
            Some("Hello")
        );
        assert_eq!(parse_translation(" Hello \n").as_deref(), Some("Hello"));
        assert_eq!(parse_translation(""), None);
    }

    #[test]
    fn token_budgets_grow_with_the_input() {
        assert_eq!(transcription_token_budget(0.0), 64);
        assert_eq!(transcription_token_budget(10.0), 224);
        assert_eq!(translation_token_budget(""), 64);
        assert_eq!(translation_token_budget(&"a".repeat(100)), 114);
    }

    #[test]
    fn translation_prompt_contains_text_and_languages() {
        let prompt = translation_prompt("Hola", Language::Es, Language::En);
        assert!(prompt.contains("Spanish text into English") && prompt.ends_with("Hola"));
    }
}
