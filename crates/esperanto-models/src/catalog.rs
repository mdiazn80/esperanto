//! The model catalog (`models/catalog.json`), embedded at compile time.

use esperanto_core::Language;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    /// What runs the model, and so what it is for.
    pub runtime: Runtime,
    /// Chat template to force on `llama-server` (`--chat-template`), when the
    /// one embedded in the model file is wrong. The Voxtral GGUF ships
    /// Devstral's template, which turns it into a coding assistant.
    #[serde(default)]
    pub chat_template: Option<String>,
    pub languages: Vec<Language>,
    pub min_ram_gb: u32,
    #[serde(default)]
    pub recommended: bool,
    pub license: License,
    pub files: Vec<ModelFile>,
}

impl ModelEntry {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    pub fn file(&self, role: FileRole) -> Option<&ModelFile> {
        self.files.iter().find(|f| f.role == role)
    }
}

/// What runs a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Runtime {
    /// Speech model (transcription and translation) served by `llama-server`.
    #[serde(rename = "llama-server")]
    LlamaServer,
    /// Speaker-embedding model run in-process by `sherpa-onnx` (speaker
    /// labels).
    #[serde(rename = "sherpa-onnx")]
    SherpaOnnx,
}

impl Runtime {
    /// The file a model of this runtime can't work without.
    pub fn main_file(self) -> FileRole {
        match self {
            Self::LlamaServer => FileRole::Model,
            Self::SherpaOnnx => FileRole::SpeakerEmbedding,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct License {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileRole {
    /// The language model weights (`-m` for llama-server).
    Model,
    /// The audio encoder (`--mmproj` for llama-server).
    AudioProjector,
    /// A speaker-embedding model (ONNX) for sherpa-onnx.
    SpeakerEmbedding,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelFile {
    pub role: FileRole,
    /// File name on disk. Must be a plain name (no directories).
    pub name: String,
    /// Download URL, pinned to a revision so the checksum stays valid.
    pub url: String,
    pub size: u64,
    /// Lowercase hex SHA-256 of the file.
    pub sha256: String,
}

const BUNDLED: &str = include_str!("../../../models/catalog.json");

impl Catalog {
    /// The catalog shipped with the app.
    pub fn bundled() -> Self {
        let catalog: Catalog =
            serde_json::from_str(BUNDLED).expect("models/catalog.json is invalid");
        if let Err(problem) = catalog.validate() {
            panic!("models/catalog.json is invalid: {problem}");
        }
        catalog
    }

    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Catalog format this build understands.
    const VERSION: u32 = 1;

    fn validate(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!("unsupported catalog version {}", self.version));
        }
        let mut ids = std::collections::HashSet::new();
        for model in &self.models {
            if !ids.insert(&model.id) {
                return Err(format!("duplicate model id {}", model.id));
            }
            if model.id.is_empty() || model.id.contains(['/', '\\', '.']) {
                return Err(format!("invalid model id {:?}", model.id));
            }
            if model.file(model.runtime.main_file()).is_none() {
                return Err(format!("{} has no model file", model.id));
            }
            for file in &model.files {
                if file.name.is_empty() || file.name.contains(['/', '\\']) || file.name == ".." {
                    return Err(format!("{}: invalid file name {:?}", model.id, file.name));
                }
                if !file.url.starts_with("https://") {
                    return Err(format!("{}: {} must use https", model.id, file.name));
                }
                if file.sha256.len() != 64 || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(format!("{}: {} has an invalid sha256", model.id, file.name));
                }
                if file.size == 0 {
                    return Err(format!("{}: {} has no size", model.id, file.name));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_valid_and_has_a_recommended_model() {
        let catalog = Catalog::bundled();
        let recommended: Vec<_> = catalog
            .models
            .iter()
            .filter(|m| m.recommended && m.runtime == Runtime::LlamaServer)
            .collect();
        assert_eq!(recommended.len(), 1);
        let voxtral = catalog.get("voxtral-mini-3b-2507-q4km").unwrap();
        assert!(voxtral.file(FileRole::AudioProjector).is_some());
        assert_eq!(voxtral.total_size(), 2_473_001_920 + 715_714_080);
        assert!(voxtral.languages.contains(&Language::Es));

        let speakers: Vec<_> = catalog
            .models
            .iter()
            .filter(|m| m.runtime == Runtime::SherpaOnnx)
            .collect();
        assert_eq!(speakers.len(), 1);
        assert!(speakers[0].file(FileRole::SpeakerEmbedding).is_some());
    }

    #[test]
    fn rejects_models_without_their_main_file() {
        let mut catalog = Catalog::bundled();
        catalog.models[0].runtime = Runtime::SherpaOnnx;
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn rejects_unsafe_file_names_and_plain_http() {
        let mut catalog = Catalog::bundled();
        catalog.models[0].files[0].name = "../evil".into();
        assert!(catalog.validate().is_err());

        let mut catalog = Catalog::bundled();
        catalog.models[0].files[0].url = "http://example.com/x".into();
        assert!(catalog.validate().is_err());
    }
}
