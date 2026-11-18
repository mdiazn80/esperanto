//! Splits partial transcripts into a stable prefix and a part that may
//! still change (LocalAgreement): words that two consecutive
//! hypotheses agree on are shown as stable.

#[derive(Debug, Default)]
pub struct Stabilizer {
    previous: Vec<String>,
}

/// A partial transcript split for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partial {
    pub stable: String,
    pub unstable: String,
}

/// Compares words ignoring case and surrounding punctuation, which models
/// often change between passes ("hola," → "hola").
fn same_word(a: &str, b: &str) -> bool {
    let clean = |w: &str| {
        w.trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase()
    };
    clean(a) == clean(b)
}

impl Stabilizer {
    pub fn update(&mut self, hypothesis: &str) -> Partial {
        let words: Vec<String> = hypothesis.split_whitespace().map(str::to_owned).collect();
        let agreed = words
            .iter()
            .zip(&self.previous)
            .take_while(|(a, b)| same_word(a, b))
            .count();
        let partial = Partial {
            stable: words[..agreed].join(" "),
            unstable: words[agreed..].join(" "),
        };
        self.previous = words;
        partial
    }

    pub fn reset(&mut self) {
        self.previous.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_hypothesis_is_all_unstable() {
        let mut s = Stabilizer::default();
        let p = s.update("hello there");
        assert_eq!(
            (p.stable.as_str(), p.unstable.as_str()),
            ("", "hello there")
        );
    }

    #[test]
    fn agreed_prefix_becomes_stable() {
        let mut s = Stabilizer::default();
        s.update("the meeting starts at");
        let p = s.update("The meeting starts at nine in");
        assert_eq!(p.stable, "The meeting starts at");
        assert_eq!(p.unstable, "nine in");
    }

    #[test]
    fn a_changed_word_ends_the_stable_part() {
        let mut s = Stabilizer::default();
        s.update("la reunión empieza, a las");
        let p = s.update("la reunión comienza a las nueve");
        assert_eq!(p.stable, "la reunión");
        assert_eq!(p.unstable, "comienza a las nueve");
    }

    #[test]
    fn reset_forgets_the_previous_hypothesis() {
        let mut s = Stabilizer::default();
        s.update("one two");
        s.reset();
        assert_eq!(s.update("one two").stable, "");
    }
}
