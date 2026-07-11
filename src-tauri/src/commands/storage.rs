use crate::storage::{self, StorageCleanupResult, StorageStats};
use crate::AppDatabase;
use tauri::Manager;

#[tauri::command]
pub async fn get_storage_stats(app_handle: tauri::AppHandle) -> Result<StorageStats, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        storage::get_storage_stats(&app_handle, db.inner())
    })
    .await
}

#[tauri::command]
pub async fn clear_wallhaven_cache(
    app_handle: tauri::AppHandle,
) -> Result<StorageCleanupResult, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        storage::clear_wallhaven_cache(&app_handle, db.inner())
    })
    .await
}

#[tauri::command]
pub async fn cleanup_unused_thumbnails(
    app_handle: tauri::AppHandle,
) -> Result<StorageCleanupResult, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        storage::cleanup_unused_thumbnails(&app_handle, db.inner())
    })
    .await
}

#[tauri::command]
pub async fn cleanup_missing_library_entries(
    app_handle: tauri::AppHandle,
) -> Result<StorageCleanupResult, String> {
    run_blocking(move || {
        let db = app_handle.state::<AppDatabase>();
        storage::cleanup_missing_library_entries(db.inner())
    })
    .await
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
