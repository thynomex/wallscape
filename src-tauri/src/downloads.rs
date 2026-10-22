use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

static ACTIVE_DOWNLOADS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

pub async fn acquire_download_slot() -> tokio::sync::SemaphorePermit<'static> {
    ACTIVE_DOWNLOADS
        .acquire()
        .await
        .expect("download semaphore stays open")
}

#[derive(Debug)]
pub struct StagedDownload {
    file: tempfile::NamedTempFile,
    pub bytes: u64,
}

impl StagedDownload {
    pub fn path(&self) -> &Path {
        self.file.path()
    }

    /// Publish only a completely downloaded file. The registration callback is
    /// synchronous so cancellation cannot interrupt publication and registration.
    pub fn commit<T>(
        self,
        source_id: &str,
        extension: &str,
        register: impl FnOnce(&Path) -> Result<T, String>,
    ) -> Result<T, String> {
        let suffix = self
            .file
            .path()
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "Invalid download staging path".to_string())?;
        let destination = self
            .file
            .path()
            .with_file_name(format!("{source_id}-{suffix}.{extension}"));
        let file = self
            .file
            .persist_noclobber(&destination)
            .map_err(|error| format!("Failed to finalize download: {}", error))?;
        drop(file);
        let mut pending = PendingCacheFile {
            path: destination,
            committed: false,
        };
        let result = register(&pending.path)?;
        pending.committed = true;
        Ok(result)
    }
}

struct PendingCacheFile {
    path: PathBuf,
    committed: bool,
}

impl Drop for PendingCacheFile {
    fn drop(&mut self) {
        if !self.committed {
            if let Err(error) = std::fs::remove_file(&self.path) {
                tracing::warn!(
                    "Failed to remove unregistered download '{}': {}",
                    self.path.display(),
                    error
                );
            }
        }
    }
}

pub async fn stream_download<F>(
    mut response: reqwest::Response,
    cache_dir: &Path,
    max_bytes: u64,
    source: &str,
    on_progress: F,
) -> Result<StagedDownload, String>
where
    F: FnMut(ByteDownloadProgress),
{
    let total_bytes = response.content_length();
    if total_bytes.is_some_and(|length| length > max_bytes) {
        return Err(format!("{source} file is too large to download"));
    }
    let directory = cache_dir.to_path_buf();
    let file = tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&directory)?;
        tempfile::Builder::new()
            .prefix("download-")
            .suffix(".part")
            .tempfile_in(directory)
    })
    .await
    .map_err(|error| format!("Failed to stage download: {error}"))?
    .map_err(|error| format!("Failed to stage download: {error}"))?;
    let writer = file
        .as_file()
        .try_clone()
        .map_err(|error| format!("Failed to open download: {error}"))?;
    let mut writer = tokio::fs::File::from_std(writer);
    let mut received_bytes = 0_u64;
    let mut progress = ByteProgressReporter::new(
        on_progress,
        ByteDownloadProgress {
            received_bytes,
            total_bytes,
        },
        Instant::now(),
    );
    let result = async {
        loop {
            let next_chunk = tokio::select! {
                chunk = response.chunk() => chunk,
                _ = tokio::time::sleep_until(
                    (progress.last_emitted_at + DOWNLOAD_PROGRESS_INTERVAL).into()
                ), if progress.pending.is_some() => {
                    progress.flush_at(Instant::now());
                    continue;
                }
            };
            let Some(chunk) =
                next_chunk.map_err(|error| format!("Failed to read {source} file: {error}"))?
            else {
                break;
            };
            received_bytes += chunk.len() as u64;
            if received_bytes > max_bytes {
                return Err(format!("{source} file is too large to download"));
            }
            writer
                .write_all(&chunk)
                .await
                .map_err(|error| format!("Failed to save {source} file: {error}"))?;
            progress.update(
                ByteDownloadProgress {
                    received_bytes,
                    total_bytes,
                },
                Instant::now(),
            );
        }
        if total_bytes.is_some_and(|total| total != received_bytes) {
            return Err(format!(
                "{source} download ended early: received {received_bytes} of {} bytes",
                total_bytes.unwrap()
            ));
        }
        if received_bytes == 0 {
            return Err(format!("{source} download ended early: empty response"));
        }
        writer
            .flush()
            .await
            .map_err(|error| format!("Failed to save {source} file: {error}"))?;
        writer
            .sync_all()
            .await
            .map_err(|error| format!("Failed to save {source} file: {error}"))?;
        Ok(())
    }
    .await;
    // Flush before the caller emits a terminal or retry status, even on a read/write error.
    progress.flush();
    result?;
    drop(writer);
    Ok(StagedDownload {
        file,
        bytes: received_bytes,
    })
}

pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";
pub const REMOTE_DOWNLOAD_MAX_ATTEMPTS: u32 = 3;
pub const REMOTE_DOWNLOAD_RETRY_DELAY_MS: u64 = 750;

#[derive(Debug, Clone, Copy)]
pub struct ByteDownloadProgress {
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
}

const DOWNLOAD_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

struct ByteProgressReporter<F: FnMut(ByteDownloadProgress)> {
    on_progress: F,
    last_emitted_at: Instant,
    pending: Option<ByteDownloadProgress>,
}

impl<F: FnMut(ByteDownloadProgress)> ByteProgressReporter<F> {
    fn new(mut on_progress: F, initial: ByteDownloadProgress, now: Instant) -> Self {
        on_progress(initial);
        Self {
            on_progress,
            last_emitted_at: now,
            pending: None,
        }
    }

    fn update(&mut self, progress: ByteDownloadProgress, now: Instant) {
        self.pending = Some(progress);
        if now.duration_since(self.last_emitted_at) >= DOWNLOAD_PROGRESS_INTERVAL {
            self.flush_at(now);
        }
    }

    fn flush_at(&mut self, now: Instant) {
        self.flush();
        self.last_emitted_at = now;
    }

    fn flush(&mut self) {
        if let Some(progress) = self.pending.take() {
            (self.on_progress)(progress);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub source: String,
    pub source_id: String,
    pub download_key: String,
    pub status: String,
    pub attempt: u32,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress: Option<f32>,
    pub message: Option<String>,
}

impl DownloadProgressEvent {
    pub fn new(
        source: &str,
        source_id: &str,
        status: &str,
        attempt: u32,
        received_bytes: u64,
        total_bytes: Option<u64>,
        message: Option<String>,
    ) -> Self {
        let progress = total_bytes
            .filter(|total| *total > 0)
            .map(|total| (received_bytes as f32 / total as f32).clamp(0.0, 1.0));

        Self {
            source: source.to_string(),
            source_id: source_id.to_string(),
            download_key: remote_download_key(source, source_id),
            status: status.to_string(),
            attempt,
            received_bytes,
            total_bytes,
            progress,
            message,
        }
    }
}

pub fn remote_download_key(source: &str, source_id: &str) -> String {
    format!("{source}:{source_id}")
}

pub fn emit_download_progress(app_handle: &tauri::AppHandle, event: DownloadProgressEvent) {
    if let Err(error) = app_handle.emit(DOWNLOAD_PROGRESS_EVENT, event) {
        tracing::debug!("Failed to emit download progress event: {}", error);
    }
}

pub fn should_retry_download_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();

    error.contains("download failed")
        || error.contains("failed to read")
        || error.contains("ended early")
        || error.contains("timed out")
        || error.contains("timeout")
        || error.contains("http 408")
        || error.contains("http 429")
        || error.contains("http 500")
        || error.contains("http 502")
        || error.contains("http 503")
        || error.contains("http 504")
}

#[cfg(test)]
pub(crate) mod progress_tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
    use tokio::net::TcpListener;

    pub(crate) async fn assert_client_connection_reuse(
        new_client: impl Fn() -> reqwest::Client,
        user_agent: &'static str,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = BufReader::new(socket);
            for _ in 0..2 {
                let read_headers = async {
                    let mut headers = String::new();
                    loop {
                        let mut line = String::new();
                        assert!(socket.read_line(&mut line).await.unwrap() > 0);
                        if line == "\r\n" {
                            return headers;
                        }
                        headers.push_str(&line);
                    }
                };
                let headers = tokio::select! {
                    headers = read_headers => headers,
                    _ = listener.accept() => panic!("A new client opened another connection"),
                };
                assert!(headers.lines().any(|line| {
                    line.split_once(':').is_some_and(|(name, value)| {
                        name.eq_ignore_ascii_case("user-agent") && value.trim() == user_agent
                    })
                }));
                socket
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                    .await
                    .unwrap();
            }
        });

        tokio::time::timeout(Duration::from_secs(5), async {
            for _ in 0..2 {
                let body = new_client()
                    .get(&url)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                assert_eq!(body, "ok");
            }
            server.await.unwrap();
        })
        .await
        .expect("Local HTTP requests should reuse the connection promptly");
    }

    #[test]
    fn byte_progress_coalesces_chunks_and_flushes_terminal_bytes() {
        let start = Instant::now();
        let mut events = Vec::new();
        {
            let mut reporter = ByteProgressReporter::new(
                |progress| events.push(progress.received_bytes),
                ByteDownloadProgress {
                    received_bytes: 0,
                    total_bytes: Some(201),
                },
                start,
            );
            for received_bytes in 1..=201 {
                reporter.update(
                    ByteDownloadProgress {
                        received_bytes,
                        total_bytes: Some(201),
                    },
                    start + Duration::from_millis(received_bytes),
                );
            }
            reporter.flush();
            reporter.flush();
        }
        assert_eq!(events, [0, 100, 200, 201]);
    }

    #[test]
    fn byte_progress_retry_starts_immediately_and_preserves_unknown_totals() {
        let start = Instant::now();
        let mut events = Vec::new();
        for attempt in 1..=2 {
            let mut reporter = ByteProgressReporter::new(
                |progress| events.push((attempt, progress.received_bytes, progress.total_bytes)),
                ByteDownloadProgress {
                    received_bytes: 0,
                    total_bytes: None,
                },
                start,
            );
            reporter.update(
                ByteDownloadProgress {
                    received_bytes: 42,
                    total_bytes: None,
                },
                start + Duration::from_millis(1),
            );
            reporter.flush();
        }
        assert_eq!(
            events,
            [(1, 0, None), (1, 42, None), (2, 0, None), (2, 42, None)]
        );
    }

    #[tokio::test]
    async fn stream_progress_flushes_failed_attempt_and_successful_retry() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for body in [b"part".as_slice(), b"complete".as_slice()] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                assert!(socket.read(&mut request).await.unwrap() > 0);
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
                socket.write_all(body).await.unwrap();
                socket.shutdown().await.unwrap();
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let mut events = Vec::new();
        for attempt in 1..=2 {
            let response = client.get(&url).send().await.unwrap();
            let result = stream_download(response, directory.path(), 100, "Test", |progress| {
                events.push((attempt, progress.received_bytes, progress.total_bytes));
            })
            .await;
            if attempt == 1 {
                assert!(should_retry_download_error(&result.unwrap_err()));
            } else {
                let download = result.unwrap();
                assert_eq!(download.bytes, 8);
                assert_eq!(std::fs::read(download.path()).unwrap(), b"complete");
                drop(download);
            }
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        }
        server.await.unwrap();
        assert_eq!(
            events,
            [
                (1, 0, Some(8)),
                (1, 4, Some(8)),
                (2, 0, Some(8)),
                (2, 8, Some(8))
            ]
        );
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    async fn response(headers: &str, body: &[u8]) -> reqwest::Response {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wire = [
            format!("HTTP/1.1 200 OK\r\nConnection: close\r\n{headers}\r\n").into_bytes(),
            body.to_vec(),
        ]
        .concat();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            stream.write_all(&wire).await.unwrap();
        });
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap()
    }

    fn assert_empty(directory: &Path) {
        assert_eq!(std::fs::read_dir(directory).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn streams_bytes_and_removes_uncommitted_file_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        let response = response("Content-Length: 5\r\n", b"hello").await;
        let mut progress = Vec::new();
        let download = stream_download(response, directory.path(), 10, "Test", |event| {
            progress.push(event)
        })
        .await
        .unwrap();
        assert_eq!(download.bytes, 5);
        assert_eq!(std::fs::read(download.path()).unwrap(), b"hello");
        assert_eq!(progress.last().unwrap().received_bytes, 5);
        drop(download);
        assert_empty(directory.path());
    }

    #[tokio::test]
    async fn rejects_oversized_declared_and_unknown_length_responses() {
        let directory = tempfile::tempdir().unwrap();
        for headers in ["Content-Length: 5\r\n", ""] {
            let response = response(headers, b"hello").await;
            let error = stream_download(response, directory.path(), 3, "Test", |_| {})
                .await
                .unwrap_err();
            assert!(error.contains("too large"));
            assert_empty(directory.path());
        }
    }

    #[tokio::test]
    async fn rejects_truncated_and_empty_responses_and_removes_partial_files() {
        let directory = tempfile::tempdir().unwrap();
        for (headers, body) in [
            ("Content-Length: 8\r\n", b"short".as_slice()),
            ("Content-Length: 0\r\n", b"".as_slice()),
        ] {
            let response = response(headers, body).await;
            assert!(
                stream_download(response, directory.path(), 10, "Test", |_| {})
                    .await
                    .is_err()
            );
            assert_empty(directory.path());
        }
    }

    #[tokio::test]
    async fn cancellation_removes_partial_file() {
        let directory = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\npartial")
                .await
                .unwrap();
            std::future::pending::<()>().await;
        });
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap();
        let path = directory.path().to_path_buf();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let download = tokio::spawn(async move {
            let mut tx = Some(tx);
            stream_download(response, &path, 200, "Test", |progress| {
                if progress.received_bytes > 0 {
                    if let Some(tx) = tx.take() {
                        let _ = tx.send(());
                    }
                }
            })
            .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), rx)
            .await
            .unwrap()
            .unwrap();
        download.abort();
        assert!(download.await.unwrap_err().is_cancelled());
        server.abort();
        assert_empty(directory.path());
    }

    #[tokio::test]
    async fn failed_registration_removes_final_file_and_success_keeps_it() {
        let directory = tempfile::tempdir().unwrap();
        let response = response("Content-Length: 5\r\n", b"hello").await;
        let download = stream_download(response, directory.path(), 10, "Test", |_| {})
            .await
            .unwrap();
        let result: Result<(), _> = download.commit("abc", "jpg", |path| {
            assert_eq!(std::fs::read(path).unwrap(), b"hello");
            assert_eq!(path.extension().unwrap(), "jpg");
            Err("database failed".to_string())
        });
        assert!(result.is_err());
        assert_empty(directory.path());
        let response = self::response("Content-Length: 5\r\n", b"hello").await;
        let download = stream_download(response, directory.path(), 10, "Test", |_| {})
            .await
            .unwrap();
        let path = download
            .commit("abc", "jpg", |path| Ok(path.to_path_buf()))
            .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"hello");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn limits_concurrent_downloads_and_releases_slots() {
        let first = acquire_download_slot().await;
        let second = acquire_download_slot().await;
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(20),
            acquire_download_slot()
        )
        .await
        .is_err());
        drop(first);
        let third =
            tokio::time::timeout(std::time::Duration::from_secs(1), acquire_download_slot())
                .await
                .unwrap();
        drop((second, third));
    }
}
