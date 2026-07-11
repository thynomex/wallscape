use crate::db::WallpaperMetadata;
use crate::downloads::{
    emit_download_progress, should_retry_download_error, DownloadProgressEvent,
    REMOTE_DOWNLOAD_MAX_ATTEMPTS, REMOTE_DOWNLOAD_RETRY_DELAY_MS,
};
use crate::motionbgs::{
    motionbgs_download_source_id, mp4_duration_ms, DownloadedMotionBgsVideo, MotionBgsClient,
    MotionBgsSearchRequest, MotionBgsSearchResponse, MotionBgsWallpaper, MOTIONBGS_SOURCE_NAME,
};
use crate::motionbgs_cache::store_motionbgs_video;
use crate::AppDatabase;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MotionBgsSearchArgs {
    pub query: Option<String>,
    pub category: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DownloadMotionBgsResult {
    pub wallpaper: crate::db::Wallpaper,
    pub source_id: String,
}

#[tauri::command]
pub async fn search_motionbgs(
    args: MotionBgsSearchArgs,
) -> Result<MotionBgsSearchResponse, String> {
    let client = MotionBgsClient::new()?;
    client
        .search(MotionBgsSearchRequest {
            query: args.query,
            category: args.category,
            page: args.page,
        })
        .await
}

#[tauri::command]
pub async fn download_motionbgs_wallpaper(
    app_handle: tauri::AppHandle,
    db: tauri::State<'_, AppDatabase>,
    wallpaper: MotionBgsWallpaper,
) -> Result<DownloadMotionBgsResult, String> {
    let download_source_id = motionbgs_download_source_id(&wallpaper);

    if let Some(existing) =
        db.0.lock()
            .get_wallpaper_by_source(MOTIONBGS_SOURCE_NAME, &download_source_id)
            .map_err(|e| format!("Failed to check MotionBGS library entry: {}", e))?
    {
        emit_motionbgs_download_status(
            &app_handle,
            &download_source_id,
            "complete",
            1,
            1,
            Some(1),
            Some("Already in library".to_string()),
        );
        return Ok(DownloadMotionBgsResult {
            source_id: download_source_id,
            wallpaper: existing,
        });
    }

    let client = MotionBgsClient::new()?;
    let (video, final_attempt) = download_motionbgs_video_with_retries(
        &app_handle,
        &client,
        &wallpaper,
        &download_source_id,
    )
    .await?;

    emit_motionbgs_download_status(
        &app_handle,
        &download_source_id,
        "saving",
        final_attempt,
        video.bytes.len() as u64,
        Some(video.bytes.len() as u64),
        Some("Saving to library".to_string()),
    );

    let video_path = store_motionbgs_video(
        &app_handle,
        &download_source_id,
        video.extension,
        &video.bytes,
    )?;
    let duration_ms = mp4_duration_ms(&video.bytes).unwrap_or(0);

    let mut tags = vec![
        MOTIONBGS_SOURCE_NAME.to_string(),
        video.download.quality.clone(),
    ];
    tags.extend(video.wallpaper.tags.clone());

    let metadata = WallpaperMetadata {
        title: video.wallpaper.title.clone(),
        thumbnail_path: Some(video.wallpaper.thumbnail_url.clone()),
        tags,
        width: video.download.width,
        height: video.download.height,
        fps: 60,
        duration_ms,
        media_type: "video".to_string(),
        source: Some(MOTIONBGS_SOURCE_NAME.to_string()),
        source_id: Some(download_source_id.clone()),
    };

    let file_path = video_path.to_string_lossy().to_string();
    let id =
        db.0.lock()
            .add_wallpaper(&metadata, &file_path)
            .map_err(|e| format!("Failed to save MotionBGS wallpaper: {}", e))?;

    let imported = db
        .0
        .lock()
        .get_wallpaper(id)
        .map_err(|e| format!("Failed to read MotionBGS wallpaper: {}", e))?
        .ok_or_else(|| "Downloaded MotionBGS wallpaper was not found after saving".to_string())?;

    emit_motionbgs_download_status(
        &app_handle,
        &download_source_id,
        "complete",
        final_attempt,
        video.bytes.len() as u64,
        Some(video.bytes.len() as u64),
        Some("Download complete".to_string()),
    );

    Ok(DownloadMotionBgsResult {
        source_id: download_source_id,
        wallpaper: imported,
    })
}

async fn download_motionbgs_video_with_retries(
    app_handle: &tauri::AppHandle,
    client: &MotionBgsClient,
    wallpaper: &MotionBgsWallpaper,
    download_source_id: &str,
) -> Result<(DownloadedMotionBgsVideo, u32), String> {
    let mut attempt = 1;

    loop {
        emit_motionbgs_download_status(
            app_handle,
            download_source_id,
            "downloading",
            attempt,
            0,
            positive_bytes(wallpaper.file_size),
            Some(if attempt == 1 {
                "Starting download".to_string()
            } else {
                format!("Retrying download, attempt {attempt}")
            }),
        );

        let progress_app_handle = app_handle.clone();
        let progress_source_id = download_source_id.to_string();
        let result = client
            .download_video_with_progress(wallpaper, move |progress| {
                emit_motionbgs_download_status(
                    &progress_app_handle,
                    &progress_source_id,
                    "downloading",
                    attempt,
                    progress.received_bytes,
                    progress.total_bytes,
                    None,
                );
            })
            .await;

        match result {
            Ok(video) => return Ok((video, attempt)),
            Err(error)
                if attempt < REMOTE_DOWNLOAD_MAX_ATTEMPTS
                    && should_retry_download_error(&error) =>
            {
                let next_attempt = attempt + 1;
                emit_motionbgs_download_status(
                    app_handle,
                    download_source_id,
                    "retrying",
                    next_attempt,
                    0,
                    positive_bytes(wallpaper.file_size),
                    Some(format!(
                        "Download failed; retrying attempt {next_attempt} of {REMOTE_DOWNLOAD_MAX_ATTEMPTS}"
                    )),
                );
                sleep(Duration::from_millis(
                    REMOTE_DOWNLOAD_RETRY_DELAY_MS * attempt as u64,
                ))
                .await;
                attempt = next_attempt;
            }
            Err(error) => {
                emit_motionbgs_download_status(
                    app_handle,
                    download_source_id,
                    "failed",
                    attempt,
                    0,
                    positive_bytes(wallpaper.file_size),
                    Some(error.clone()),
                );
                return Err(error);
            }
        }
    }
}

fn emit_motionbgs_download_status(
    app_handle: &tauri::AppHandle,
    source_id: &str,
    status: &str,
    attempt: u32,
    received_bytes: u64,
    total_bytes: Option<u64>,
    message: Option<String>,
) {
    emit_download_progress(
        app_handle,
        DownloadProgressEvent::new(
            MOTIONBGS_SOURCE_NAME,
            source_id,
            status,
            attempt,
            received_bytes,
            total_bytes,
            message,
        ),
    );
}

fn positive_bytes(value: i64) -> Option<u64> {
    (value > 0).then_some(value as u64)
}
