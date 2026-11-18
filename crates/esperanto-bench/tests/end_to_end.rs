//! Runs a two-clip test set against a mock OpenAI-compatible server.

use esperanto_bench::{manifest, report, run};
use esperanto_inference::{OpenAiCompatibleProvider, OpenAiConfig};
use serde_json::json;
use wiremock::matchers::{body_string_contains, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn answer(content: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{ "message": { "content": content.to_string() } }]
    }))
}

fn write_silence(path: &std::path::Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..32_000 {
        writer.write_sample(0_i16).unwrap();
    }
    writer.finalize().unwrap();
}

#[tokio::test]
async fn measures_transcription_and_translation() {
    let dir = std::env::temp_dir().join(format!("esperanto-bench-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    write_silence(&dir.join("en.wav"));
    write_silence(&dir.join("es.wav"));
    std::fs::write(
        dir.join("manifest.jsonl"),
        [
            json!({"id": "1", "audio": "en.wav", "language": "en",
                   "transcript": "the cat sat on the mat",
                   "translations": {"es": "El gato se sentó en la alfombra."}}),
            json!({"id": "1", "audio": "es.wav", "language": "es",
                   "transcript": "el gato se sentó en la alfombra",
                   "translations": {"en": "The cat sat on the mat."}}),
        ]
        .map(|v| v.to_string())
        .join("\n"),
    )
    .unwrap();

    let server = MockServer::start().await;
    // Translations of the transcripts (text-only requests).
    Mock::given(method("POST"))
        .and(body_string_contains("text into Spanish"))
        .respond_with(answer(
            json!({ "translation": "El gato se sentó en la alfombra." }),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("text into English"))
        .respond_with(answer(json!({ "translation": "The cat sat on a mat." })))
        .mount(&server)
        .await;
    // Transcriptions: one word wrong in English, perfect in Spanish.
    Mock::given(method("POST"))
        .and(body_string_contains("English audio"))
        .respond_with(answer(json!({ "transcript": "the cat sat on the hat" })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("Spanish audio"))
        .respond_with(answer(
            json!({ "transcript": "El gato se sentó en la alfombra." }),
        ))
        .mount(&server)
        .await;

    let provider =
        OpenAiCompatibleProvider::new(OpenAiConfig::new(format!("{}/v1", server.uri()), "mock"))
            .unwrap();
    let items = manifest::load(&dir.join("manifest.jsonl")).unwrap();
    let options = run::RunOptions {
        translate: true,
        fixed_source: true,
        warmup: false,
    };
    let records = run::run(&provider, &items, &options).await.unwrap();
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(|r| r.error.is_none()));
    assert!(records.iter().all(|r| (r.audio_secs - 2.0).abs() < 1e-9));

    let rows = report::summarize(&records);
    let row = |task: &str| rows.iter().find(|r| r.task == task).unwrap();
    assert!((row("transcribe en").wer.unwrap() - 100.0 / 6.0).abs() < 1e-9);
    assert_eq!(row("transcribe es").wer, Some(0.0));
    assert!((row("translate en→es").chrf.unwrap() - 100.0).abs() < 1e-9);
    assert!(row("translate es→en").chrf.unwrap() < 100.0);
    // With a fixed spoken language the reported language is always right.
    assert_eq!(row("transcribe es").language_id, Some(100.0));

    let markdown = report::to_markdown("mock", &rows);
    assert!(markdown.contains("| translate es→en | 1 | 0 |"));
    std::fs::remove_dir_all(dir).unwrap();
}
