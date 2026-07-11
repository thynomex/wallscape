use super::shell::open_url;
use crate::db::WallpaperMetadata;
use crate::downloads::{
    emit_download_progress, should_retry_download_error, DownloadProgressEvent,
    REMOTE_DOWNLOAD_MAX_ATTEMPTS, REMOTE_DOWNLOAD_RETRY_DELAY_MS,
};
use crate::wallhaven::{
    DownloadedImage, WallhavenClient, WallhavenSearchRequest, WallhavenSearchResponse,
    WallhavenWallpaper, WALLHAVEN_SOURCE_NAME,
};
use crate::wallhaven_cache::store_wallhaven_image;
use crate::AppDatabase;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct WallhavenSearchArgs {
    pub query: Option<String>,
    pub categories: Option<String>,
    pub purity: Option<String>,
    pub sorting: Option<String>,
    pub order: Option<String>,
    pub atleast: Option<String>,
    pub ratios: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DownloadWallhavenResult {
    pub wallpaper: crate::db::Wallpaper,
    pub source_id: String,
}

#[tauri::command]
pub async fn search_wallhaven(
    args: WallhavenSearchArgs,
) -> Result<WallhavenSearchResponse, String> {
    let client = WallhavenClient::new()?;
    client
        .search(WallhavenSearchRequest {
            query: args.query,
            categories: args.categories,
            purity: args.purity,
            sorting: args.sorting,
            order: args.order,
            atleast: args.atleast,
            ratios: args.ratios,
            page: args.page,
        })
        .await
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| "Invalid external URL".to_string())?;
    let host = parsed
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "External URL is missing a host".to_string())?;

    let allowed_host =
        host == "wallhaven.cc" || host.ends_with(".wallhaven.cc") || host == "motionbgs.com";
    if parsed.scheme() != "https" || !allowed_host {
        return Err("Only Wallhaven or MotionBGS HTTPS URLs can be opened".to_string());
    }

    open_url(parsed.as_str())
}

#[tauri::command]
pub async fn download_wallhaven_wallpaper(
    app_handle: tauri::AppHandle,
    db: tauri::State<'_, AppDatabase>,
    wallpaper: WallhavenWallpaper,
) -> Result<DownloadWallhavenResult, String> {
    if let Some(existing) =
        db.0.lock()
            .get_wallpaper_by_source(WALLHAVEN_SOURCE_NAME, &wallpaper.id)
            .map_err(|e| format!("Failed to check Wallhaven library entry: {}", e))?
    {
        emit_wallhaven_download_status(
            &app_handle,
            &wallpaper.id,
            "complete",
            1,
            1,
            Some(1),
            Some("Already in library".to_string()),
        );
        return Ok(DownloadWallhavenResult {
            source_id: wallpaper.id,
            wallpaper: existing,
        });
    }

    let client = WallhavenClient::new()?;
    let (image, final_attempt) =
        download_wallhaven_image_with_retries(&app_handle, &client, &wallpaper).await?;

    emit_wallhaven_download_status(
        &app_handle,
        &wallpaper.id,
        "saving",
        final_attempt,
        image.bytes.len() as u64,
        Some(image.bytes.len() as u64),
        Some("Saving to library".to_string()),
    );

    let image_path =
        store_wallhaven_image(&app_handle, &wallpaper.id, image.extension, &image.bytes)?;

    let metadata = WallpaperMetadata {
        title: format!("Wallhaven {} {}", wallpaper.category, wallpaper.id),
        thumbnail_path: Some(wallpaper.thumbs.large.clone()),
        tags: vec![
            WALLHAVEN_SOURCE_NAME.to_string(),
            wallpaper.category.clone(),
            wallpaper.purity.clone(),
        ],
        width: wallpaper.dimension_x,
        height: wallpaper.dimension_y,
        fps: 0,
        duration_ms: 0,
        media_type: "image".to_string(),
        source: Some(WALLHAVEN_SOURCE_NAME.to_string()),
        source_id: Some(wallpaper.id.clone()),
    };

    let file_path = image_path.to_string_lossy().to_string();
    let id =
        db.0.lock()
            .add_wallpaper(&metadata, &file_path)
            .map_err(|e| format!("Failed to save Wallhaven wallpaper: {}", e))?;

    let imported = db
        .0
        .lock()
        .get_wallpaper(id)
        .map_err(|e| format!("Failed to read Wallhaven wallpaper: {}", e))?
        .ok_or_else(|| "Downloaded Wallhaven wallpaper was not found after saving".to_string())?;

    emit_wallhaven_download_status(
        &app_handle,
        &wallpaper.id,
        "complete",
        final_attempt,
        image.bytes.len() as u64,
        Some(image.bytes.len() as u64),
        Some("Download complete".to_string()),
    );

    Ok(DownloadWallhavenResult {
        source_id: wallpaper.id,
        wallpaper: imported,
    })
}

async fn download_wallhaven_image_with_retries(
    app_handle: &tauri::AppHandle,
    client: &WallhavenClient,
    wallpaper: &WallhavenWallpaper,
) -> Result<(DownloadedImage, u32), String> {
    let mut attempt = 1;

    loop {
        emit_wallhaven_download_status(
            app_handle,
            &wallpaper.id,
            "downloading",
            attempt,
            0,
            Some(wallpaper.file_size.max(0) as u64),
            Some(if attempt == 1 {
                "Starting download".to_string()
            } else {
                format!("Retrying download, attempt {attempt}")
            }),
        );

        let progress_app_handle = app_handle.clone();
        let progress_source_id = wallpaper.id.clone();
        let result = client
            .download_image_with_progress(&wallpaper.path, move |progress| {
                emit_wallhaven_download_status(
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
            Ok(image) => return Ok((image, attempt)),
            Err(error)
                if attempt < REMOTE_DOWNLOAD_MAX_ATTEMPTS
                    && should_retry_download_error(&error) =>
            {
                let next_attempt = attempt + 1;
                emit_wallhaven_download_status(
                    app_handle,
                    &wallpaper.id,
                    "retrying",
                    next_attempt,
                    0,
                    Some(wallpaper.file_size.max(0) as u64),
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
                emit_wallhaven_download_status(
                    app_handle,
                    &wallpaper.id,
                    "failed",
                    attempt,
                    0,
                    Some(wallpaper.file_size.max(0) as u64),
                    Some(error.clone()),
                );
                return Err(error);
            }
        }
    }
}

fn emit_wallhaven_download_status(
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
            WALLHAVEN_SOURCE_NAME,
            source_id,
            status,
            attempt,
            received_bytes,
            total_bytes,
            message,
        ),
    );
}
