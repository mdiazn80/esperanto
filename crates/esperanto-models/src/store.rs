//! Where models live on disk: `<root>/<model id>/<file name>`, with
//! `<file name>.part` while downloading.

use std::path::{Path, PathBuf};

use crate::{ModelEntry, ModelFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    Installed,
    /// Some bytes are on disk (a paused or interrupted download).
    Partial {
        downloaded: u64,
    },
    NotInstalled,
}

#[derive(Debug, Clone)]
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model_dir(&self, model: &ModelEntry) -> PathBuf {
        self.root.join(&model.id)
    }

    pub fn file_path(&self, model: &ModelEntry, file: &ModelFile) -> PathBuf {
        self.model_dir(model).join(&file.name)
    }

    pub(crate) fn part_path(&self, model: &ModelEntry, file: &ModelFile) -> PathBuf {
        self.model_dir(model).join(format!("{}.part", file.name))
    }

    /// Bytes of `file` on disk, complete or partial.
    pub fn downloaded_bytes(&self, model: &ModelEntry, file: &ModelFile) -> u64 {
        let complete = file_len(&self.file_path(model, file));
        if complete == Some(file.size) {
            return file.size;
        }
        // A part file can't count for more than the file (e.g. after a bad
        // response); the download restarts it.
        file_len(&self.part_path(model, file)).map_or(0, |len| len.min(file.size))
    }

    /// Installed means every file is present with the expected size (files
    /// are only renamed into place after their checksum was verified).
    pub fn status(&self, model: &ModelEntry) -> ModelStatus {
        let complete = model
            .files
            .iter()
            .all(|f| file_len(&self.file_path(model, f)) == Some(f.size));
        if complete {
            return ModelStatus::Installed;
        }
        let downloaded: u64 = model
            .files
            .iter()
            .map(|f| self.downloaded_bytes(model, f))
            .sum();
        if downloaded > 0 {
            ModelStatus::Partial { downloaded }
        } else {
            ModelStatus::NotInstalled
        }
    }

    /// Removes the model's files, including partial downloads.
    pub fn delete(&self, model: &ModelEntry) -> std::io::Result<()> {
        match std::fs::remove_dir_all(self.model_dir(model)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    /// Bytes used by all models (complete and partial).
    pub fn disk_usage(&self) -> u64 {
        fn walk(path: &Path) -> u64 {
            let Ok(entries) = std::fs::read_dir(path) else {
                return 0;
            };
            entries
                .flatten()
                .map(|entry| match entry.file_type() {
                    Ok(t) if t.is_dir() => walk(&entry.path()),
                    Ok(_) => entry.metadata().map(|m| m.len()).unwrap_or(0),
                    Err(_) => 0,
                })
                .sum()
        }
        walk(&self.root)
    }
}

fn file_len(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}
