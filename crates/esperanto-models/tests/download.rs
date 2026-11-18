use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use esperanto_core::Language;
use esperanto_models::{
    DownloadError, FileRole, License, ModelEntry, ModelFile, ModelStatus, ModelStore, Progress,
    Runtime, download,
};
use sha2::{Digest, Sha256};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn temp_store(name: &str) -> ModelStore {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("esperanto-models-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    ModelStore::new(dir)
}

fn content(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

fn entry(server: &MockServer, files: &[(&str, &[u8])]) -> ModelEntry {
    ModelEntry {
        id: "test-model".into(),
        name: "Test".into(),
        description: String::new(),
        runtime: Runtime::LlamaServer,
        chat_template: None,
        languages: vec![Language::En],
        min_ram_gb: 1,
        recommended: false,
        license: License {
            name: "MIT".into(),
            url: "https://example.com".into(),
        },
        files: files
            .iter()
            .enumerate()
            .map(|(i, (name, bytes))| ModelFile {
                role: if i == 0 {
                    FileRole::Model
                } else {
                    FileRole::AudioProjector
                },
                name: (*name).into(),
                url: format!("{}/{name}", server.uri()),
                size: bytes.len() as u64,
                sha256: hex::encode(Sha256::digest(bytes)),
            })
            .collect(),
    }
}

/// Serves a file, honouring `Range: bytes=N-` like a real CDN.
struct RangeServer(Vec<u8>);

impl Respond for RangeServer {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let start = request
            .headers
            .get("range")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("bytes="))
            .and_then(|v| v.trim_end_matches('-').parse::<usize>().ok());
        match start {
            Some(start) => ResponseTemplate::new(206).set_body_bytes(self.0[start..].to_vec()),
            None => ResponseTemplate::new(200).set_body_bytes(self.0.clone()),
        }
    }
}

async fn run(
    store: &ModelStore,
    model: &ModelEntry,
    cancel: &AtomicBool,
) -> (Result<(), DownloadError>, Vec<Progress>) {
    let mut progress = Vec::new();
    let result = download(&reqwest::Client::new(), store, model, cancel, |p| {
        progress.push(p)
    })
    .await;
    (result, progress)
}

#[tokio::test]
async fn downloads_and_verifies_every_file() {
    let server = MockServer::start().await;
    let a = content(300_000, 1);
    let b = content(50_000, 2);
    Mock::given(path("/a.gguf"))
        .respond_with(RangeServer(a.clone()))
        .mount(&server)
        .await;
    Mock::given(path("/b.gguf"))
        .respond_with(RangeServer(b.clone()))
        .mount(&server)
        .await;
    let store = temp_store("full");
    let model = entry(&server, &[("a.gguf", &a), ("b.gguf", &b)]);

    assert_eq!(store.status(&model), ModelStatus::NotInstalled);
    let (result, progress) = run(&store, &model, &AtomicBool::new(false)).await;
    result.unwrap();

    assert_eq!(store.status(&model), ModelStatus::Installed);
    assert_eq!(
        std::fs::read(store.file_path(&model, &model.files[0])).unwrap(),
        a
    );
    let last = progress.last().unwrap();
    assert_eq!((last.downloaded, last.total), (350_000, 350_000));
    assert!(
        progress
            .windows(2)
            .all(|w| w[0].downloaded <= w[1].downloaded)
    );
    assert_eq!(store.disk_usage(), 350_000);

    // Nothing left to download: no more requests.
    let requests = server.received_requests().await.unwrap().len();
    run(&store, &model, &AtomicBool::new(false))
        .await
        .0
        .unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), requests);

    store.delete(&model).unwrap();
    assert_eq!(store.status(&model), ModelStatus::NotInstalled);
}

#[tokio::test]
async fn resumes_a_partial_download_with_a_range_request() {
    let server = MockServer::start().await;
    let a = content(200_000, 3);
    Mock::given(method("GET"))
        .and(path("/a.gguf"))
        .and(header("range", "bytes=120000-"))
        .respond_with(RangeServer(a.clone()))
        .expect(1)
        .mount(&server)
        .await;
    let store = temp_store("resume");
    let model = entry(&server, &[("a.gguf", &a)]);
    let part = store.model_dir(&model).join("a.gguf.part");
    std::fs::create_dir_all(store.model_dir(&model)).unwrap();
    std::fs::write(&part, &a[..120_000]).unwrap();
    assert_eq!(
        store.status(&model),
        ModelStatus::Partial {
            downloaded: 120_000
        }
    );

    run(&store, &model, &AtomicBool::new(false))
        .await
        .0
        .unwrap();
    assert_eq!(
        std::fs::read(store.file_path(&model, &model.files[0])).unwrap(),
        a
    );
    assert!(!part.exists());
}

#[tokio::test]
async fn starts_over_if_the_server_ignores_the_range() {
    let server = MockServer::start().await;
    let a = content(100_000, 4);
    Mock::given(path("/a.gguf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(a.clone()))
        .mount(&server)
        .await;
    let store = temp_store("norange");
    let model = entry(&server, &[("a.gguf", &a)]);
    std::fs::create_dir_all(store.model_dir(&model)).unwrap();
    std::fs::write(store.model_dir(&model).join("a.gguf.part"), &a[..40_000]).unwrap();

    run(&store, &model, &AtomicBool::new(false))
        .await
        .0
        .unwrap();
    assert_eq!(
        std::fs::read(store.file_path(&model, &model.files[0])).unwrap(),
        a
    );
}

#[tokio::test]
async fn rejects_corrupted_files() {
    let server = MockServer::start().await;
    let a = content(10_000, 5);
    let mut corrupted = a.clone();
    corrupted[5_000] ^= 0xff;
    Mock::given(path("/a.gguf"))
        .respond_with(RangeServer(corrupted))
        .mount(&server)
        .await;
    let store = temp_store("corrupt");
    let model = entry(&server, &[("a.gguf", &a)]);

    let (result, _) = run(&store, &model, &AtomicBool::new(false)).await;
    assert!(matches!(
        result,
        Err(DownloadError::ChecksumMismatch { .. })
    ));
    assert_eq!(store.status(&model), ModelStatus::NotInstalled);
}

#[tokio::test]
async fn cancelling_keeps_the_partial_file() {
    let server = MockServer::start().await;
    let a = content(2_000_000, 6);
    Mock::given(path("/a.gguf"))
        .respond_with(RangeServer(a.clone()))
        .mount(&server)
        .await;
    let store = temp_store("cancel");
    let model = entry(&server, &[("a.gguf", &a)]);
    let cancel = AtomicBool::new(false);

    let mut progress_calls = 0;
    let result = download(&reqwest::Client::new(), &store, &model, &cancel, |_| {
        progress_calls += 1;
        // Cancel right after the initial progress report.
        cancel.store(true, Ordering::Relaxed);
    })
    .await;
    assert!(matches!(result, Err(DownloadError::Cancelled)));
    assert!(progress_calls >= 1);
    assert!(matches!(
        store.status(&model),
        ModelStatus::Partial { .. } | ModelStatus::NotInstalled
    ));
    assert!(!store.file_path(&model, &model.files[0]).exists());
}

#[tokio::test]
async fn reports_http_errors() {
    let server = MockServer::start().await;
    Mock::given(path("/a.gguf"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let store = temp_store("404");
    let model = entry(&server, &[("a.gguf", b"data")]);
    let (result, _) = run(&store, &model, &AtomicBool::new(false)).await;
    assert!(matches!(
        result,
        Err(DownloadError::Status { status: 404, .. })
    ));
}
