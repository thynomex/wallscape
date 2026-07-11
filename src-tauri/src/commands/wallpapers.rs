use super::wallpaper_health;
use super::wallpaper_history;
use super::wallpaper_import;
use super::wallpaper_import_scan;
use super::wallpaper_library;
use super::wallpaper_restore;
use super::wallpaper_rotation;
use crate::importing::ImportProbe;
use crate::wallpaper::WallpaperRuntime;
use crate::wallpaper_lifecycle;
use crate::AppDatabase;
use tauri::Manager;

pub use super::wallpaper_health::BrokenWallpaper;
pub use super::wallpaper_import::{
    BatchImportItem, BatchImportResult, ImportWallpaperResult, ThumbnailRegenerationResult,
};
pub use super::wallpaper_import_scan::ImportScanResult;
pub use super::wallpaper_library::RemoveWallpaperResult;
pub use super::wallpaper_restore::{WallpaperBackupStatus, WallpaperRestoreStatus};

#[tauri::command]
pub async fn list_wallpapers(
    db: tauri::State<'_, AppDatabase>,
) -> Result<Vec<crate::db::Wallpaper>, String> {
    db.0.lock()
        .list_wallpapers()
        .map_err(|e| format!("Failed to list wallpapers: {}", e))
}

#[tauri::command]
pub async fn search_wallpapers(
    db: tauri::State<'_, AppDatabase>,
    query: String,
) -> Result<Vec<crate::db::Wallpaper>, String> {
    db.0.lock()
        .search_wallpapers(&query)
        .map_err(|e| format!("Failed to search wallpapers: {}", e))
}

#[tauri::command]
pub async fn scan_import_paths(paths: Vec<String>) -> Result<ImportScanResult, String> {
    run_blocking(move || wallpaper_import_scan::scan_import_paths(paths)).await
}

#[tauri::command]
pub async fn import_wallpaper(
    app_handle: tauri::AppHandle,
    video_path: String,
    probe: Option<ImportProbe>,
) -> Result<ImportWallpaperResult, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        wallpaper_import::import_wallpaper(&app_handle, db.inner(), video_path, probe)
    })
    .await
}

#[tauri::command]
pub async fn import_wallpapers(
    app_handle: tauri::AppHandle,
    items: Vec<BatchImportItem>,
) -> Result<BatchImportResult, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        wallpaper_import::import_wallpapers(&app_handle, db.inner(), items)
    })
    .await
}

#[tauri::command]
pub async fn detect_broken_wallpapers(
    app_handle: tauri::AppHandle,
) -> Result<Vec<BrokenWallpaper>, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        let wallpapers =
            db.0.lock()
                .list_wallpapers()
                .map_err(|e| format!("Failed to list wallpapers: {}", e))?;

        Ok(wallpaper_health::detect_broken_wallpapers(wallpapers))
    })
    .await
}

#[tauri::command]
pub async fn reveal_wallpaper_in_explorer(
    db: tauri::State<'_, AppDatabase>,
    id: i64,
) -> Result<(), String> {
    wallpaper_library::reveal_wallpaper_in_explorer(db.inner(), id)
}

#[tauri::command]
pub async fn regenerate_wallpaper_thumbnail(
    app_handle: tauri::AppHandle,
    id: i64,
    probe: ImportProbe,
) -> Result<ThumbnailRegenerationResult, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        wallpaper_import::regenerate_wallpaper_thumbnail(&app_handle, db.inner(), id, probe)
    })
    .await
}

#[tauri::command]
pub async fn remove_wallpaper(
    db: tauri::State<'_, AppDatabase>,
    id: i64,
) -> Result<RemoveWallpaperResult, String> {
    wallpaper_library::remove_wallpaper(db.inner(), id)
}

#[tauri::command]
pub async fn set_wallpaper_favorite(
    db: tauri::State<'_, AppDatabase>,
    id: i64,
    is_favorite: bool,
) -> Result<crate::db::Wallpaper, String> {
    wallpaper_library::set_wallpaper_favorite(db.inner(), id, is_favorite)
}

#[tauri::command]
pub async fn set_wallpaper(
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
    video_path: String,
    monitor_id: Option<String>,
) -> Result<String, String> {
    wallpaper_lifecycle::apply_wallpaper_path(db.inner(), runtime.inner(), video_path, monitor_id)
        .await
}

#[tauri::command]
pub async fn rotate_random_favorite_wallpaper(
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
) -> Result<crate::db::Wallpaper, String> {
    rotate_random_favorite_wallpaper_inner(db.inner(), runtime.inner()).await
}

pub(crate) async fn rotate_random_favorite_wallpaper_inner(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<crate::db::Wallpaper, String> {
    wallpaper_rotation::rotate_random_favorite_wallpaper(db, runtime).await
}

#[tauri::command]
pub async fn list_wallpaper_history(
    db: tauri::State<'_, AppDatabase>,
    limit: Option<i64>,
) -> Result<Vec<crate::db::WallpaperHistoryEntry>, String> {
    wallpaper_history::list_wallpaper_history(db.inner(), limit)
}

#[tauri::command]
pub async fn undo_wallpaper_history(
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
) -> Result<crate::db::WallpaperHistoryEntry, String> {
    wallpaper_history::undo_wallpaper_history(db.inner(), runtime.inner()).await
}

#[tauri::command]
pub async fn get_original_wallpaper_backup(
    db: tauri::State<'_, AppDatabase>,
) -> Result<WallpaperBackupStatus, String> {
    wallpaper_restore::get_original_wallpaper_backup(db.inner())
}

#[tauri::command]
pub async fn get_previous_wallpaper(
    db: tauri::State<'_, AppDatabase>,
) -> Result<WallpaperRestoreStatus, String> {
    wallpaper_restore::get_previous_wallpaper(db.inner())
}

#[tauri::command]
pub async fn restore_previous_wallpaper(
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
) -> Result<String, String> {
    wallpaper_restore::restore_previous_wallpaper(db.inner(), runtime.inner()).await
}

#[tauri::command]
pub async fn restore_original_wallpaper(
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
) -> Result<String, String> {
    wallpaper_restore::restore_original_wallpaper(db.inner(), runtime.inner()).await
}

pub(crate) fn restore_original_wallpaper_blocking(
    app_handle: &tauri::AppHandle,
    runtime: &WallpaperRuntime,
) -> Result<String, String> {
    wallpaper_restore::restore_original_wallpaper_blocking(app_handle, runtime)
}

async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
}
