//! Quality metrics: word error rate for transcripts, chrF for translations.

/// Lowercases, replaces punctuation and symbols with spaces and collapses
/// whitespace. Accents are kept (they matter in Spanish).
pub fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Word-level edit distance after [`normalize`]: (errors, reference words).
/// Sum both over a corpus and divide to get the corpus WER.
pub fn word_errors(reference: &str, hypothesis: &str) -> (usize, usize) {
    let reference = normalize(reference);
    let hypothesis = normalize(hypothesis);
    let r: Vec<&str> = reference.split_whitespace().collect();
    let h: Vec<&str> = hypothesis.split_whitespace().collect();

    let mut prev: Vec<usize> = (0..=h.len()).collect();
    for (i, rw) in r.iter().enumerate() {
        let mut row = vec![i + 1; h.len() + 1];
        for (j, hw) in h.iter().enumerate() {
            let substitution = prev[j] + usize::from(rw != hw);
            row[j + 1] = substitution.min(prev[j + 1] + 1).min(row[j] + 1);
        }
        prev = row;
    }
    (prev[h.len()], r.len())
}

/// Character n-gram statistics for chrF, accumulated over a corpus.
#[derive(Debug, Clone, Default)]
pub struct ChrfStats {
    /// Per n-gram order (1..=6): (matches, hypothesis n-grams, reference n-grams).
    orders: [(usize, usize, usize); ChrfStats::MAX_ORDER],
}

impl ChrfStats {
    const MAX_ORDER: usize = 6;
    const BETA: f64 = 2.0;

    /// Adds one sentence pair. Whitespace is ignored, as in sacreBLEU's chrF.
    pub fn add(&mut self, reference: &str, hypothesis: &str) {
        let r: Vec<char> = reference.chars().filter(|c| !c.is_whitespace()).collect();
        let h: Vec<char> = hypothesis.chars().filter(|c| !c.is_whitespace()).collect();
        for n in 1..=Self::MAX_ORDER {
            let ref_counts = ngram_counts(&r, n);
            let hyp_counts = ngram_counts(&h, n);
            let matches: usize = hyp_counts
                .iter()
                .map(|(gram, count)| (*count).min(ref_counts.get(gram).copied().unwrap_or(0)))
                .sum();
            let entry = &mut self.orders[n - 1];
            entry.0 += matches;
            entry.1 += hyp_counts.values().sum::<usize>();
            entry.2 += ref_counts.values().sum::<usize>();
        }
    }

    /// chrF2 score in 0–100: precision and recall averaged over n-gram
    /// orders, then combined with recall weighted twice as much.
    pub fn score(&self) -> f64 {
        let used: Vec<_> = self
            .orders
            .iter()
            .filter(|(_, h, r)| *h > 0 && *r > 0)
            .collect();
        if used.is_empty() {
            return 0.0;
        }
        let precision = used
            .iter()
            .map(|(m, h, _)| *m as f64 / *h as f64)
            .sum::<f64>()
            / used.len() as f64;
        let recall = used
            .iter()
            .map(|(m, _, r)| *m as f64 / *r as f64)
            .sum::<f64>()
            / used.len() as f64;
        if precision + recall == 0.0 {
            return 0.0;
        }
        let beta2 = Self::BETA * Self::BETA;
        100.0 * (1.0 + beta2) * precision * recall / (beta2 * precision + recall)
    }
}

fn ngram_counts(chars: &[char], n: usize) -> std::collections::HashMap<&[char], usize> {
    let mut counts = std::collections::HashMap::new();
    for gram in chars.windows(n) {
        *counts.entry(gram).or_insert(0) += 1;
    }
    counts
}

/// Value at quantile `q` (0–1) of a sorted slice, nearest-rank.
pub fn quantile(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_punctuation_but_keeps_accents() {
        assert_eq!(normalize("¡Hola, Señor!  ¿Qué tal?"), "hola señor qué tal");
    }

    #[test]
    fn counts_word_errors() {
        assert_eq!(word_errors("the cat sat", "the cat sat"), (0, 3));
        assert_eq!(word_errors("the cat sat", "the bat sat down"), (2, 3));
        assert_eq!(word_errors("The cat.", "the, cat"), (0, 2));
        assert_eq!(word_errors("hello world", ""), (2, 2));
    }

    #[test]
    fn chrf_is_100_for_identical_and_lower_for_different_text() {
        let mut same = ChrfStats::default();
        same.add("the cat sat on the mat", "the cat sat on the mat");
        assert!((same.score() - 100.0).abs() < 1e-9);

        let mut close = ChrfStats::default();
        close.add("the cat sat on the mat", "the cat sat on a mat");
        let mut far = ChrfStats::default();
        far.add("the cat sat on the mat", "dogs run fast");
        assert!(close.score() > far.score());
        assert!(close.score() < 100.0);
    }

    #[test]
    fn quantiles_use_nearest_rank() {
        let sorted = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(quantile(&sorted, 0.5), Some(5.0));
        assert_eq!(quantile(&sorted, 0.9), Some(9.0));
        assert_eq!(quantile(&[], 0.5), None);
    }
}
