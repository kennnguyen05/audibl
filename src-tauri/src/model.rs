//! The one local speech model: where it lives, whether it is ready, and the
//! resumable, sha256-verified download onboarding requires.
//!
//! The resumable HTTP logic is ported from Handy
//! (`managers/model/download.rs`, MIT), minus hf-hub and the catalog.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio_util::sync::CancellationToken;

pub struct ModelSpec {
    pub name: &'static str,
    pub repo: &'static str,
    pub revision: &'static str,
    pub file: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

impl ModelSpec {
    pub fn url(&self) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repo, self.revision, self.file
        )
    }
}

/// Chosen by the Milestone 3 bake-off. Swapping models only changes this.
pub const MODEL: ModelSpec = ModelSpec {
    name: "Qwen3-ASR 1.7B",
    repo: "handy-computer/Qwen3-ASR-1.7B-gguf",
    revision: "92282af1610a2db19d66f2bef1e260f5deca782d",
    file: "Qwen3-ASR-1.7B-Q5_K_M.gguf",
    size: 1_517_290_464,
    sha256: "034c557fe92ff8fcd9a9c041cbdaad347be0a86a58d3a348f63cf3f0180879d0",
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const STALL_TIMEOUT: Duration = Duration::from_secs(60);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
/// Free space kept beyond the bytes still to download.
const DISK_MARGIN_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModelStatus {
    Missing,
    Partial { downloaded: u64, total: u64 },
    Downloading { downloaded: u64, total: u64 },
    Verifying,
    Ready,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type, Event)]
pub struct ModelDownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub verifying: bool,
}

/// `reason` is a code the frontend translates: `network`, `stalled`,
/// `checksum`, `disk_space`, `server`, `io`.
#[derive(Serialize, Deserialize, Debug, Clone, Type, Event)]
pub struct ModelDownloadFailed {
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type, Event)]
pub struct ModelDownloadComplete;

#[derive(Debug, PartialEq, Eq)]
pub enum DownloadError {
    Network(String),
    Stalled,
    Checksum,
    DiskSpace,
    Server(String),
    Io(String),
}

impl DownloadError {
    pub fn reason(&self) -> &'static str {
        match self {
            DownloadError::Network(_) => "network",
            DownloadError::Stalled => "stalled",
            DownloadError::Checksum => "checksum",
            DownloadError::DiskSpace => "disk_space",
            DownloadError::Server(_) => "server",
            DownloadError::Io(_) => "io",
        }
    }
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Network(e) => write!(f, "network error: {e}"),
            DownloadError::Stalled => write!(f, "no data for {}s", STALL_TIMEOUT.as_secs()),
            DownloadError::Checksum => write!(f, "sha256 mismatch"),
            DownloadError::DiskSpace => write!(f, "not enough disk space"),
            DownloadError::Server(e) => write!(f, "server error: {e}"),
            DownloadError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        DownloadError::Io(e.to_string())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DownloadOutcome {
    Completed,
    Cancelled,
}

pub enum DownloadEvent {
    Progress { downloaded: u64, total: u64 },
    Verifying,
}

pub fn models_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("models")
}

pub fn model_path(app: &AppHandle) -> PathBuf {
    models_dir(app).join(MODEL.file)
}

fn partial_path_for(final_path: &Path) -> PathBuf {
    let mut name = final_path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    final_path.with_file_name(name)
}

/// On-disk status. Startup only checks the size; the full sha256 runs after
/// a download.
pub fn status_on_disk(final_path: &Path, expected_size: u64) -> ModelStatus {
    if fs::metadata(final_path).is_ok_and(|m| m.len() == expected_size) {
        return ModelStatus::Ready;
    }
    match fs::metadata(partial_path_for(final_path)) {
        Ok(m) if m.len() > 0 && m.len() <= expected_size => ModelStatus::Partial {
            downloaded: m.len(),
            total: expected_size,
        },
        _ => ModelStatus::Missing,
    }
}

pub fn compute_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Deletes the file on mismatch so a retry starts from zero.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<(), DownloadError> {
    match compute_sha256(path) {
        Ok(actual) if actual == expected => Ok(()),
        Ok(actual) => {
            log::warn!("sha256 mismatch: expected {expected}, got {actual}");
            let _ = fs::remove_file(path);
            Err(DownloadError::Checksum)
        }
        Err(e) => {
            let _ = fs::remove_file(path);
            Err(DownloadError::Io(e.to_string()))
        }
    }
}

fn free_disk_bytes(dir: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c_path = CString::new(dir.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    Some(stat.f_bavail as u64 * stat.f_frsize as u64)
}

fn content_range_start(value: &str) -> Option<u64> {
    let range = value.trim().strip_prefix("bytes")?.trim_start();
    range.split('-').next()?.trim().parse().ok()
}

/// Downloads `url` into `partial`, resuming from its current length, and
/// verifies size and sha256. Verified bytes stay in `partial`; renaming is the
/// caller's job. Robustness rules (from Handy):
/// - a full-size partial is verified without a request;
/// - a 200 reply to a Range request restarts from zero instead of appending;
/// - a 206 must start exactly at our offset;
/// - more bytes than expected aborts the transfer;
/// - no bytes for 60 s is a stall that keeps the partial for resume.
pub async fn download_resumable(
    url: &str,
    partial: &Path,
    expected_size: u64,
    expected_sha256: &str,
    cancel: &CancellationToken,
    emit: &(dyn Fn(DownloadEvent) + Send + Sync),
) -> Result<DownloadOutcome, DownloadError> {
    let mut resume_from = fs::metadata(partial).map(|m| m.len()).unwrap_or(0);
    if resume_from > expected_size {
        let _ = fs::remove_file(partial);
        resume_from = 0;
    }
    if resume_from == expected_size {
        emit(DownloadEvent::Verifying);
        verify_blocking(partial, expected_sha256).await?;
        return Ok(DownloadOutcome::Completed);
    }

    if let Some(dir) = partial.parent() {
        fs::create_dir_all(dir)?;
        if let Some(free) = free_disk_bytes(dir) {
            if free < expected_size - resume_from + DISK_MARGIN_BYTES {
                return Err(DownloadError::DiskSpace);
            }
        }
    }

    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    let mut request = client.get(url);
    if resume_from > 0 {
        log::info!("Resuming model download from byte {resume_from}");
        request = request.header("Range", format!("bytes={resume_from}-"));
    } else {
        log::info!("Starting model download from {url}");
    }

    let response = tokio::select! {
        r = tokio::time::timeout(STALL_TIMEOUT, request.send()) => match r {
            Err(_) => return Err(DownloadError::Stalled),
            Ok(Err(e)) => return Err(DownloadError::Network(e.to_string())),
            Ok(Ok(response)) => response,
        },
        _ = cancel.cancelled() => return Ok(DownloadOutcome::Cancelled),
    };

    let status = response.status();
    if resume_from > 0 && status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let _ = fs::remove_file(partial);
        return Err(DownloadError::Server("HTTP 416".into()));
    }
    if resume_from > 0 && status == reqwest::StatusCode::OK {
        // Server ignored the Range header; appending would corrupt the file.
        let _ = fs::remove_file(partial);
        resume_from = 0;
    }
    if !status.is_success() {
        return Err(DownloadError::Server(format!("HTTP {status}")));
    }
    if resume_from > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
        let starts_at = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(content_range_start);
        if starts_at != Some(resume_from) {
            let _ = fs::remove_file(partial);
            return Err(DownloadError::Server(format!(
                "Content-Range starts at {starts_at:?}, expected {resume_from}"
            )));
        }
    }
    if let Some(len) = response.content_length() {
        if resume_from + len != expected_size {
            return Err(DownloadError::Server(format!(
                "server advertises {} bytes, expected {expected_size}",
                resume_from + len
            )));
        }
    }

    let mut file = if resume_from > 0 {
        fs::OpenOptions::new().append(true).open(partial)?
    } else {
        File::create(partial)?
    };
    let mut downloaded = resume_from;
    emit(DownloadEvent::Progress {
        downloaded,
        total: expected_size,
    });

    let mut last_emit = Instant::now();
    let mut stream = response.bytes_stream();
    loop {
        let chunk = tokio::select! {
            c = tokio::time::timeout(STALL_TIMEOUT, stream.next()) => match c {
                Err(_) => return Err(DownloadError::Stalled),
                Ok(None) => break,
                Ok(Some(Err(e))) => return Err(DownloadError::Network(e.to_string())),
                Ok(Some(Ok(chunk))) => chunk,
            },
            _ = cancel.cancelled() => return Ok(DownloadOutcome::Cancelled),
        };
        if downloaded + chunk.len() as u64 > expected_size {
            drop(file);
            let _ = fs::remove_file(partial);
            return Err(DownloadError::Server("sent more bytes than expected".into()));
        }
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if last_emit.elapsed() >= PROGRESS_INTERVAL {
            emit(DownloadEvent::Progress {
                downloaded,
                total: expected_size,
            });
            last_emit = Instant::now();
        }
    }
    file.flush()?;
    drop(file);
    emit(DownloadEvent::Progress {
        downloaded,
        total: expected_size,
    });

    if downloaded != expected_size {
        // Connection closed early: keep the partial so Retry resumes.
        return Err(DownloadError::Network(format!(
            "connection closed at {downloaded} of {expected_size} bytes"
        )));
    }

    emit(DownloadEvent::Verifying);
    verify_blocking(partial, expected_sha256).await?;
    Ok(DownloadOutcome::Completed)
}

async fn verify_blocking(path: &Path, expected: &str) -> Result<(), DownloadError> {
    let path = path.to_path_buf();
    let expected = expected.to_string();
    tokio::task::spawn_blocking(move || verify_sha256(&path, &expected))
        .await
        .map_err(|e| DownloadError::Io(e.to_string()))?
}

#[derive(Default)]
struct DownloadState {
    cancel: Option<CancellationToken>,
    downloaded: u64,
    verifying: bool,
}

/// Tauri-managed state tracking the in-flight download.
#[derive(Default)]
pub struct ModelManager {
    state: Mutex<DownloadState>,
}

impl ModelManager {
    pub fn status(&self, app: &AppHandle) -> ModelStatus {
        let state = self.state.lock().unwrap();
        if state.cancel.is_some() {
            return if state.verifying {
                ModelStatus::Verifying
            } else {
                ModelStatus::Downloading {
                    downloaded: state.downloaded,
                    total: MODEL.size,
                }
            };
        }
        drop(state);
        status_on_disk(&model_path(app), MODEL.size)
    }

    pub fn is_ready(&self, app: &AppHandle) -> bool {
        self.status(app) == ModelStatus::Ready
    }

    pub fn start_download(&self, app: &AppHandle) {
        let token = {
            let mut state = self.state.lock().unwrap();
            if state.cancel.is_some() {
                return;
            }
            let token = CancellationToken::new();
            state.cancel = Some(token.clone());
            state.verifying = false;
            state.downloaded = 0;
            token
        };

        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let final_path = model_path(&app);
            let partial = partial_path_for(&final_path);
            let emit_app = app.clone();
            let emit = move |event: DownloadEvent| {
                let manager = emit_app.state::<ModelManager>();
                let (downloaded, verifying) = {
                    let mut state = manager.state.lock().unwrap();
                    match event {
                        DownloadEvent::Progress { downloaded, .. } => state.downloaded = downloaded,
                        DownloadEvent::Verifying => state.verifying = true,
                    }
                    (state.downloaded, state.verifying)
                };
                let _ = ModelDownloadProgress {
                    downloaded,
                    total: MODEL.size,
                    verifying,
                }
                .emit(&emit_app);
            };

            let result = download_resumable(
                &MODEL.url(),
                &partial,
                MODEL.size,
                MODEL.sha256,
                &token,
                &emit,
            )
            .await
            .and_then(|outcome| {
                if outcome == DownloadOutcome::Completed {
                    fs::rename(&partial, &final_path)?;
                }
                Ok(outcome)
            });

            *app.state::<ModelManager>().state.lock().unwrap() = DownloadState::default();
            match result {
                Ok(DownloadOutcome::Completed) => {
                    log::info!("Model downloaded and verified");
                    let _ = ModelDownloadComplete.emit(&app);
                }
                Ok(DownloadOutcome::Cancelled) => log::info!("Model download cancelled"),
                Err(e) => {
                    log::error!("Model download failed: {e}");
                    let _ = ModelDownloadFailed {
                        reason: e.reason().to_string(),
                    }
                    .emit(&app);
                }
            }
        });
    }

    pub fn cancel_download(&self) {
        if let Some(token) = &self.state.lock().unwrap().cancel {
            token.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    fn sha_hex(data: &[u8]) -> String {
        format!("{:x}", Sha256::digest(data))
    }

    #[test]
    fn status_reports_missing_partial_wrong_size_and_ready() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("m.gguf");
        assert_eq!(status_on_disk(&path, 10), ModelStatus::Missing);

        fs::write(partial_path_for(&path), b"abcd").unwrap();
        assert_eq!(
            status_on_disk(&path, 10),
            ModelStatus::Partial {
                downloaded: 4,
                total: 10
            }
        );

        fs::remove_file(partial_path_for(&path)).unwrap();
        fs::write(&path, b"wrong").unwrap();
        assert_eq!(status_on_disk(&path, 10), ModelStatus::Missing);

        fs::write(&path, b"0123456789").unwrap();
        assert_eq!(status_on_disk(&path, 10), ModelStatus::Ready);
    }

    #[test]
    fn sha256_mismatch_deletes_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("m.partial");
        fs::write(&path, b"not the model").unwrap();
        assert_eq!(
            verify_sha256(&path, &"0".repeat(64)),
            Err(DownloadError::Checksum)
        );
        assert!(!path.exists());
    }

    #[test]
    fn sha256_match_keeps_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("m.partial");
        fs::write(&path, b"hello").unwrap();
        assert_eq!(verify_sha256(&path, &sha_hex(b"hello")), Ok(()));
        assert!(path.exists());
    }

    fn http_response(status_line: &str, headers: &[String], body: &[u8]) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {status_line}\r\nConnection: close\r\n");
        for h in headers {
            head.push_str(h);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    async fn read_head(sock: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            if sock.read_exact(&mut byte).await.is_err() {
                break;
            }
            buf.push(byte[0]);
        }
        String::from_utf8_lossy(&buf).to_lowercase()
    }

    /// Serves one connection with a canned response and returns the request head.
    async fn serve_once(response: Vec<u8>) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let head = read_head(&mut sock).await;
            let _ = sock.write_all(&response).await;
            let _ = sock.shutdown().await;
            head
        });
        (format!("http://{addr}/model"), handle)
    }

    async fn download(url: &str, partial: &Path, body: &[u8]) -> Result<DownloadOutcome, DownloadError> {
        download_resumable(
            url,
            partial,
            body.len() as u64,
            &sha_hex(body),
            &CancellationToken::new(),
            &|_| {},
        )
        .await
    }

    #[tokio::test]
    async fn fresh_download_completes_and_verifies() {
        let body = b"hello world";
        let (url, server) = serve_once(http_response(
            "200 OK",
            &[format!("Content-Length: {}", body.len())],
            body,
        ))
        .await;
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("m.partial");

        assert_eq!(download(&url, &partial, body).await, Ok(DownloadOutcome::Completed));
        assert_eq!(fs::read(&partial).unwrap(), body);
        assert!(!server.await.unwrap().contains("range:"));
    }

    #[tokio::test]
    async fn resume_sends_range_and_appends() {
        let body = b"helloworld";
        let (url, server) = serve_once(http_response(
            "206 Partial Content",
            &["Content-Range: bytes 5-9/10".into(), "Content-Length: 5".into()],
            &body[5..],
        ))
        .await;
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("m.partial");
        fs::write(&partial, &body[..5]).unwrap();

        assert_eq!(download(&url, &partial, body).await, Ok(DownloadOutcome::Completed));
        assert_eq!(fs::read(&partial).unwrap(), body);
        assert!(server.await.unwrap().contains("range: bytes=5-"));
    }

    #[tokio::test]
    async fn ignored_range_restarts_from_zero() {
        let body = b"helloworld";
        let (url, _server) = serve_once(http_response(
            "200 OK",
            &[format!("Content-Length: {}", body.len())],
            body,
        ))
        .await;
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("m.partial");
        fs::write(&partial, b"hello").unwrap();

        assert_eq!(download(&url, &partial, body).await, Ok(DownloadOutcome::Completed));
        assert_eq!(fs::read(&partial).unwrap(), body);
    }

    #[tokio::test]
    async fn corrupt_download_fails_checksum_and_deletes_partial() {
        let body = b"hellowxrld";
        let (url, _server) = serve_once(http_response(
            "200 OK",
            &[format!("Content-Length: {}", body.len())],
            body,
        ))
        .await;
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("m.partial");

        let result = download_resumable(
            &url,
            &partial,
            body.len() as u64,
            &sha_hex(b"helloworld"),
            &CancellationToken::new(),
            &|_| {},
        )
        .await;
        assert_eq!(result, Err(DownloadError::Checksum));
        assert!(!partial.exists());
    }

    #[tokio::test]
    async fn early_close_keeps_partial_for_resume() {
        let (url, _server) = serve_once(http_response(
            "200 OK",
            &["Content-Length: 10".into()],
            b"hello",
        ))
        .await;
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("m.partial");

        let result = download(&url, &partial, b"helloworld").await;
        assert!(matches!(result, Err(DownloadError::Network(_))));
        assert_eq!(fs::read(&partial).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn full_size_partial_is_verified_without_request() {
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("m.partial");
        fs::write(&partial, b"helloworld").unwrap();
        // Unroutable URL: any request would fail the test.
        let result = download("http://127.0.0.1:9/never", &partial, b"helloworld").await;
        assert_eq!(result, Ok(DownloadOutcome::Completed));
    }
}
