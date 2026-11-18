//! Aggregates records into a summary table per task and language pair.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use esperanto_core::Language;
use serde::Serialize;

use crate::metrics::{ChrfStats, quantile};
use crate::run::{Mode, Record};

#[derive(Debug, Clone, Serialize)]
pub struct SummaryRow {
    pub task: String,
    pub requests: usize,
    pub failures: usize,
    /// Corpus word error rate of the transcript, in percent.
    pub wer: Option<f64>,
    /// Corpus chrF2 of the translation (translate rows only).
    pub chrf: Option<f64>,
    /// Share of answers whose detected language matches the spoken one, in percent.
    pub language_id: Option<f64>,
    pub latency_p50: Option<f64>,
    pub latency_p90: Option<f64>,
    /// Mean real-time factor: processing time / audio duration (< 1 is faster than real time).
    pub rtf_mean: Option<f64>,
}

fn task_name(mode: Mode, language: Language, target: Option<Language>) -> String {
    match (mode, target) {
        (Mode::Translate, Some(target)) => {
            format!("translate {}→{}", language.code(), target.code())
        }
        _ => format!("transcribe {}", language.code()),
    }
}

pub fn summarize(records: &[Record]) -> Vec<SummaryRow> {
    let mut groups: BTreeMap<(Mode, Language, Option<Language>), Vec<&Record>> = BTreeMap::new();
    for record in records {
        groups
            .entry((record.mode, record.language, record.target))
            .or_default()
            .push(record);
    }

    groups
        .into_iter()
        .map(|((mode, language, target), group)| {
            let ok: Vec<&&Record> = group.iter().filter(|r| r.error.is_none()).collect();
            let (errors, words) = ok.iter().fold((0, 0), |(e, w), r| {
                (e + r.word_errors, w + r.reference_words)
            });

            let chrf = (mode == Mode::Translate && !ok.is_empty()).then(|| {
                let mut stats = ChrfStats::default();
                for r in &ok {
                    stats.add(
                        r.reference_translation.as_deref().unwrap_or_default(),
                        r.translation.as_deref().unwrap_or_default(),
                    );
                }
                stats.score()
            });

            let detected: Vec<_> = ok.iter().filter_map(|r| r.detected_language).collect();
            let language_id = (!detected.is_empty()).then(|| {
                100.0 * detected.iter().filter(|l| **l == language).count() as f64
                    / detected.len() as f64
            });

            let mut latencies: Vec<f64> = ok.iter().map(|r| r.latency_secs).collect();
            latencies.sort_by(f64::total_cmp);
            let rtfs: Vec<f64> = ok
                .iter()
                .filter(|r| r.audio_secs > 0.0)
                .map(|r| r.latency_secs / r.audio_secs)
                .collect();

            SummaryRow {
                task: task_name(mode, language, target),
                requests: group.len(),
                failures: group.len() - ok.len(),
                wer: (words > 0).then(|| 100.0 * errors as f64 / words as f64),
                chrf,
                language_id,
                latency_p50: quantile(&latencies, 0.5),
                latency_p90: quantile(&latencies, 0.9),
                rtf_mean: (!rtfs.is_empty()).then(|| rtfs.iter().sum::<f64>() / rtfs.len() as f64),
            }
        })
        .collect()
}

fn cell(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "–".to_owned(), |v| format!("{v:.decimals$}"))
}

pub fn to_markdown(title: &str, rows: &[SummaryRow]) -> String {
    let mut out = format!("## {title}\n\n");
    out.push_str(
        "| Task | Requests | Failures | WER % ↓ | chrF ↑ | Lang ID % | p50 s | p90 s | RTF ↓ |\n",
    );
    out.push_str(
        "|------|---------:|---------:|--------:|-------:|----------:|------:|------:|------:|\n",
    );
    for row in rows {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.task,
            row.requests,
            row.failures,
            cell(row.wer, 1),
            cell(row.chrf, 1),
            cell(row.language_id, 0),
            cell(row.latency_p50, 2),
            cell(row.latency_p90, 2),
            cell(row.rtf_mean, 2),
        );
    }
    out.push_str(
        "\nWER: word error rate of the transcript. chrF: chrF2 of the translation against the \
         reference. Translate rows time both requests (transcription, then translation of the \
         text). RTF: processing time / audio duration.\n",
    );
    out
}
