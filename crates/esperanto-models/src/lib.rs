//! AI models for the local provider: the catalog of models the
//! app knows about, where they are stored, and resumable, checksum-verified
//! downloads.

mod catalog;
mod download;
mod store;

pub use catalog::{Catalog, FileRole, License, ModelEntry, ModelFile, Runtime};
pub use download::{DownloadError, Progress, download};
pub use store::{ModelStatus, ModelStore};
