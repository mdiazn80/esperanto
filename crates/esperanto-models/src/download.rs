//! Resumable, checksum-verified downloads.
//!
//! Each file is downloaded to `<name>.part`, resuming with an HTTP `Range`
//! request if a partial file exists. The SHA-256 covers the whole file
//! (resumed bytes are re-read), and only a verified file is renamed into
//! place. Cancelling keeps the `.part` file so the download can resume.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{ModelEntry, ModelFile, ModelStore};

/// Extra free space kept on top of the download size.
const SPACE_MARGIN: u64 = 200 * 1024 * 1024;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes on disk across all files of the model.
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("download failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server answered {status} for {file}")]
    Status { file: String, status: u16 },
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{file} is corrupted (checksum mismatch)")]
    ChecksumMismatch { file: String },
    #[error("not enough disk space: {needed} bytes needed, {available} available")]
    InsufficientSpace { needed: u64, available: u64 },
    #[error("download cancelled")]
    Cancelled,
}

/// Downloads every missing file of `model` into `store`.
pub async fn download(
    http: &reqwest::Client,
    store: &ModelStore,
    model: &ModelEntry,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(Progress),
) -> Result<(), DownloadError> {
    tokio::fs::create_dir_all(store.model_dir(model)).await?;

    let total = model.total_size();
    let already: u64 = model
        .files
        .iter()
        .map(|f| store.downloaded_bytes(model, f))
        .sum();
    let needed = total.saturating_sub(already);
    let available = fs4::available_space(store.root())?;
    if needed + SPACE_MARGIN > available {
        return Err(DownloadError::InsufficientSpace { needed, available });
    }

    let mut done_before = 0;
    on_progress(Progress {
        downloaded: already,
        total,
    });
    for file in &model.files {
        let report = |bytes: u64| Progress {
            downloaded: done_before + bytes,
            total,
        };
        download_file(http, store, model, file, cancel, |bytes| {
            on_progress(report(bytes))
        })
        .await?;
        done_before += file.size;
    }
    on_progress(Progress {
        downloaded: total,
        total,
    });
    Ok(())
}

async fn download_file(
    http: &reqwest::Client,
    store: &ModelStore,
    model: &ModelEntry,
    file: &ModelFile,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64),
) -> Result<(), DownloadError> {
    let target = store.file_path(model, file);
    if tokio::fs::metadata(&target).await.map(|m| m.len()).ok() == Some(file.size) {
        return Ok(());
    }
    let part = store.part_path(model, file);

    // Hash what is already on disk, then continue from there.
    let mut hasher = Sha256::new();
    let mut offset = hash_existing(&part, &mut hasher).await?;
    if offset > file.size {
        tokio::fs::remove_file(&part).await?;
        hasher = Sha256::new();
        offset = 0;
    }

    if offset < file.size {
        let mut request = http.get(&file.url);
        if offset > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let mut response = request.send().await?;
        let status = response.status();
        let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
        if !status.is_success() {
            return Err(DownloadError::Status {
                file: file.name.clone(),
                status: status.as_u16(),
            });
        }
        if offset > 0 && !resumed {
            // The server ignored the range: start over.
            hasher = Sha256::new();
            offset = 0;
        }

        let mut output = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(resumed)
            .truncate(!resumed)
            .open(&part)
            .await?;
        let mut last_report = Instant::now();
        while let Some(chunk) = response.chunk().await? {
            if cancel.load(Ordering::Relaxed) {
                output.flush().await?;
                return Err(DownloadError::Cancelled);
            }
            output.write_all(&chunk).await?;
            hasher.update(&chunk);
            offset += chunk.len() as u64;
            if last_report.elapsed() >= PROGRESS_INTERVAL {
                on_progress(offset);
                last_report = Instant::now();
            }
        }
        output.flush().await?;
        output.sync_all().await?;
    }

    let digest = hex::encode(hasher.finalize());
    if offset != file.size || !digest.eq_ignore_ascii_case(&file.sha256) {
        tokio::fs::remove_file(&part).await?;
        return Err(DownloadError::ChecksumMismatch {
            file: file.name.clone(),
        });
    }
    tokio::fs::rename(&part, &target).await?;
    on_progress(file.size);
    Ok(())
}

async fn hash_existing(path: &std::path::Path, hasher: &mut Sha256) -> std::io::Result<u64> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut buffer = vec![0; 1 << 20];
    let mut total = 0;
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            return Ok(total);
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
}
