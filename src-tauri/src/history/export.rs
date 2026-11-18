//! Turns a saved session into a text file: plain
//! text, Markdown, or subtitles (SRT, WebVTT).

use std::fmt::Write;

use serde::Deserialize;
use specta::Type;

use super::model::{HistorySegment, HistorySession};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Md,
    Srt,
    Vtt,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }
}

/// Which text of each phrase to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExportContent {
    Original,
    /// The translation; the original where there is none.
    Translation,
    /// The original, then its translation.
    Both,
}

/// The lines to write for a phrase.
fn lines(segment: &HistorySegment, content: ExportContent) -> Vec<&str> {
    let translation = segment.translation.as_deref().filter(|t| !t.is_empty());
    match (content, translation) {
        (ExportContent::Original, _) | (_, None) => vec![&segment.text],
        (ExportContent::Translation, Some(t)) => vec![t],
        (ExportContent::Both, Some(t)) => vec![&segment.text, t],
    }
}

/// `HH:MM:SS` + `separator` + milliseconds, as subtitles need.
fn timestamp(ms: f64, separator: char) -> String {
    let total = ms.max(0.0).round() as u64;
    let (h, m, s, millis) = (
        total / 3_600_000,
        total / 60_000 % 60,
        total / 1000 % 60,
        total % 1000,
    );
    format!("{h:02}:{m:02}:{s:02}{separator}{millis:03}")
}

/// `MM:SS` (or `H:MM:SS` past an hour), for reading.
fn clock(ms: f64) -> String {
    let total = (ms.max(0.0) / 1000.0) as u64;
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Subtitle cues can't contain an empty line (it ends the cue).
fn cue_text(lines: &[&str]) -> String {
    lines
        .iter()
        .flat_map(|line| line.lines())
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The file's contents. `title` heads Markdown files.
pub fn export(
    session: &HistorySession,
    format: ExportFormat,
    content: ExportContent,
    title: &str,
) -> String {
    let mut out = String::new();
    match format {
        ExportFormat::Txt => {
            for segment in &session.segments {
                for line in lines(segment, content) {
                    out.push_str(line);
                    out.push('\n');
                }
                if content == ExportContent::Both {
                    out.push('\n');
                }
            }
        }
        ExportFormat::Md => {
            let _ = writeln!(out, "# {title}\n");
            for segment in &session.segments {
                let lines = lines(segment, content);
                let _ = writeln!(out, "`{}` {}", clock(segment.start_ms), lines[0]);
                if let Some(translation) = lines.get(1) {
                    let _ = writeln!(out, "\n_{translation}_");
                }
                out.push('\n');
            }
        }
        ExportFormat::Srt => {
            for (index, segment) in session.segments.iter().enumerate() {
                let _ = writeln!(
                    out,
                    "{}\n{} --> {}\n{}\n",
                    index + 1,
                    timestamp(segment.start_ms, ','),
                    timestamp(segment.end_ms, ','),
                    cue_text(&lines(segment, content)),
                );
            }
        }
        ExportFormat::Vtt => {
            out.push_str("WEBVTT\n\n");
            for segment in &session.segments {
                let _ = writeln!(
                    out,
                    "{} --> {}\n{}\n",
                    timestamp(segment.start_ms, '.'),
                    timestamp(segment.end_ms, '.'),
                    cue_text(&lines(segment, content)),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::model::tests::{segment, session};
    use super::*;

    fn sample() -> HistorySession {
        let mut untranslated = segment(65_250.0, 3_725_500.0, "Bye.");
        untranslated.translation = None;
        session(vec![segment(500.0, 2_040.0, "Hello."), untranslated])
    }

    #[test]
    fn formats_timestamps() {
        assert_eq!(timestamp(0.0, ','), "00:00:00,000");
        assert_eq!(timestamp(3_725_500.4, '.'), "01:02:05.500");
        assert_eq!(timestamp(-5.0, ','), "00:00:00,000");
        assert_eq!(clock(65_250.0), "01:05");
        assert_eq!(clock(3_725_500.0), "1:02:05");
    }

    #[test]
    fn exports_plain_text() {
        let s = sample();
        assert_eq!(
            export(&s, ExportFormat::Txt, ExportContent::Original, "T"),
            "Hello.\nBye.\n"
        );
        assert_eq!(
            export(&s, ExportFormat::Txt, ExportContent::Translation, "T"),
            "[es] Hello.\nBye.\n"
        );
        assert_eq!(
            export(&s, ExportFormat::Txt, ExportContent::Both, "T"),
            "Hello.\n[es] Hello.\n\nBye.\n\n"
        );
    }

    #[test]
    fn exports_markdown() {
        assert_eq!(
            export(&sample(), ExportFormat::Md, ExportContent::Both, "Session"),
            "# Session\n\n`00:00` Hello.\n\n_[es] Hello._\n\n`01:05` Bye.\n\n"
        );
    }

    #[test]
    fn exports_srt() {
        assert_eq!(
            export(&sample(), ExportFormat::Srt, ExportContent::Both, "T"),
            "1\n00:00:00,500 --> 00:00:02,040\nHello.\n[es] Hello.\n\n\
             2\n00:01:05,250 --> 01:02:05,500\nBye.\n\n"
        );
    }

    #[test]
    fn exports_webvtt() {
        assert_eq!(
            export(
                &sample(),
                ExportFormat::Vtt,
                ExportContent::Translation,
                "T"
            ),
            "WEBVTT\n\n00:00:00.500 --> 00:00:02.040\n[es] Hello.\n\n\
             00:01:05.250 --> 01:02:05.500\nBye.\n\n"
        );
    }

    #[test]
    fn cues_never_contain_blank_lines() {
        let s = session(vec![segment(0.0, 1.0, "One.\n\nTwo.")]);
        let srt = export(&s, ExportFormat::Srt, ExportContent::Original, "T");
        assert_eq!(srt, "1\n00:00:00,000 --> 00:00:00,001\nOne.\nTwo.\n\n");
    }
}
