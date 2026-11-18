//! Test sets: a JSON Lines file with one clip per line.
//!
//! ```json
//! {"id":"1001","audio":"en/1001.wav","language":"en","transcript":"…","translations":{"es":"…"}}
//! ```
//!
//! `audio` is relative to the manifest's folder. `translations` holds the
//! reference translation for each target language.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use esperanto_core::Language;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    pub id: String,
    pub audio: PathBuf,
    pub language: Language,
    pub transcript: String,
    #[serde(default)]
    pub translations: HashMap<Language, String>,
}

/// Loads a manifest, resolving audio paths against its folder.
pub fn load(path: &Path) -> Result<Vec<Item>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read manifest {}", path.display()))?;
    let base = path.parent().unwrap_or(Path::new("."));
    let mut items = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut item: Item = serde_json::from_str(line)
            .with_context(|| format!("{}:{}: invalid item", path.display(), index + 1))?;
        if item.audio.is_relative() {
            item.audio = base.join(&item.audio);
        }
        items.push(item);
    }
    if items.is_empty() {
        bail!("manifest {} has no items", path.display());
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_items_and_resolves_audio_paths() {
        let dir = std::env::temp_dir().join(format!("esperanto-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.jsonl");
        std::fs::write(
            &path,
            concat!(
                r#"{"id":"1","audio":"en/1.wav","language":"en","transcript":"Hi","translations":{"es":"Hola"}}"#,
                "\n\n",
                r#"{"id":"2","audio":"/abs/2.wav","language":"es","transcript":"Hola"}"#,
                "\n"
            ),
        )
        .unwrap();

        let items = load(&path).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].audio, dir.join("en/1.wav"));
        assert_eq!(items[0].translations[&Language::Es], "Hola");
        assert_eq!(items[1].audio, PathBuf::from("/abs/2.wav"));
        assert!(items[1].translations.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
