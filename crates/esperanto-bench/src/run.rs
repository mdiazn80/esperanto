//! Runs every clip of a test set through a provider and records the
//! answers, timings and per-clip metrics.

use std::time::Instant;

use esperanto_core::{Language, SourceLanguage, SpeechTask};
use esperanto_inference::{InferenceProvider, run_task};
use serde::Serialize;

use crate::manifest::Item;
use crate::metrics::word_errors;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Transcription only.
    Transcribe,
    /// Transcription, then translation of the transcript.
    Translate,
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Also measure transcription + translation into the other language.
    pub translate: bool,
    /// Tell the model the spoken language (what the app does, D19) instead
    /// of asking it to detect it.
    pub fixed_source: bool,
    /// Run the first clip once before measuring (loads the model, warms caches).
    pub warmup: bool,
}

/// One request: what was asked, what came back, and how long it took.
#[derive(Debug, Clone, Serialize)]
pub struct Record {
    pub id: String,
    pub mode: Mode,
    pub language: Language,
    pub target: Option<Language>,
    pub audio_secs: f64,
    pub latency_secs: f64,
    pub transcript: Option<String>,
    pub translation: Option<String>,
    pub reference_translation: Option<String>,
    pub detected_language: Option<Language>,
    pub word_errors: usize,
    pub reference_words: usize,
    pub error: Option<String>,
}

fn other_language(language: Language) -> Language {
    match language {
        Language::En => Language::Es,
        Language::Es => Language::En,
    }
}

pub async fn run<P: InferenceProvider>(
    provider: &P,
    items: &[Item],
    options: &RunOptions,
) -> anyhow::Result<Vec<Record>> {
    let mut jobs = Vec::new();
    for item in items {
        jobs.push((item, None));
        if options.translate {
            let target = other_language(item.language);
            if item.translations.contains_key(&target) {
                jobs.push((item, Some(target)));
            }
        }
    }

    if options.warmup
        && let Some((item, _)) = jobs.first()
    {
        eprintln!("warming up with {}…", item.id);
        let task = build_task(item, None, options)?;
        let _ = run_task(provider, &task).await;
    }

    let mut records = Vec::with_capacity(jobs.len());
    for (index, (item, target)) in jobs.iter().enumerate() {
        let task = build_task(item, *target, options)?;
        let started = Instant::now();
        let outcome = run_task(provider, &task).await;
        let latency_secs = started.elapsed().as_secs_f64();

        let record = match outcome {
            Ok(result) => {
                let (errors, words) = word_errors(&item.transcript, &result.transcript);
                Record {
                    id: item.id.clone(),
                    mode: mode(*target),
                    language: item.language,
                    target: *target,
                    audio_secs: task.audio.duration_secs(),
                    latency_secs,
                    transcript: Some(result.transcript),
                    translation: result.translation,
                    reference_translation: target.and_then(|t| item.translations.get(&t).cloned()),
                    detected_language: result.detected_language,
                    word_errors: errors,
                    reference_words: words,
                    error: None,
                }
            }
            Err(error) => Record {
                id: item.id.clone(),
                mode: mode(*target),
                language: item.language,
                target: *target,
                audio_secs: task.audio.duration_secs(),
                latency_secs,
                transcript: None,
                translation: None,
                reference_translation: target.and_then(|t| item.translations.get(&t).cloned()),
                detected_language: None,
                word_errors: 0,
                reference_words: 0,
                error: Some(error.to_string()),
            },
        };
        eprintln!(
            "[{}/{}] {} {:?}{} {:.2}s{}",
            index + 1,
            jobs.len(),
            record.id,
            record.mode,
            target
                .map(|t| format!(" →{}", t.code()))
                .unwrap_or_default(),
            latency_secs,
            record
                .error
                .as_ref()
                .map(|e| format!(" ERROR: {e}"))
                .unwrap_or_default()
        );
        records.push(record);
    }
    Ok(records)
}

fn mode(target: Option<Language>) -> Mode {
    if target.is_some() {
        Mode::Translate
    } else {
        Mode::Transcribe
    }
}

fn build_task(
    item: &Item,
    target: Option<Language>,
    options: &RunOptions,
) -> anyhow::Result<SpeechTask> {
    Ok(SpeechTask {
        audio: esperanto_audio::load_wav(&item.audio)?,
        source_language: if options.fixed_source {
            SourceLanguage::Fixed(item.language)
        } else {
            SourceLanguage::Auto
        },
        target_language: target,
    })
}
