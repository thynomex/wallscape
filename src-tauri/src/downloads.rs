use serde::Serialize;
use tauri::Emitter;

pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";
pub const REMOTE_DOWNLOAD_MAX_ATTEMPTS: u32 = 3;
pub const REMOTE_DOWNLOAD_RETRY_DELAY_MS: u64 = 750;

#[derive(Debug, Clone, Copy)]
pub struct ByteDownloadProgress {
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
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
