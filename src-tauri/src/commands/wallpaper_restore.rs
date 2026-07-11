use crate::settings::PREVIOUS_WALLPAPER_KEY;
use crate::wallpaper::WallpaperRuntime;
use crate::wallpaper_lifecycle::{self, ORIGINAL_WALLPAPER_BACKUP_KEY};
use crate::AppDatabase;
use serde::Serialize;
use std::path::Path;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct WallpaperBackupStatus {
    pub path: Option<String>,
    pub can_restore: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct WallpaperRestoreStatus {
    pub path: Option<String>,
    pub can_restore: bool,
}

pub(crate) fn get_original_wallpaper_backup(
    db: &AppDatabase,
) -> Result<WallpaperBackupStatus, String> {
    let path =
        db.0.lock()
            .get_preference(ORIGINAL_WALLPAPER_BACKUP_KEY)
            .map_err(|e| format!("Failed to read original wallpaper backup: {}", e))?;

    let can_restore = can_restore_saved_path(path.as_deref());

    Ok(WallpaperBackupStatus { path, can_restore })
}

pub(crate) fn get_previous_wallpaper(db: &AppDatabase) -> Result<WallpaperRestoreStatus, String> {
    let path =
        db.0.lock()
            .get_preference(PREVIOUS_WALLPAPER_KEY)
            .map_err(|e| format!("Failed to read previous wallpaper: {}", e))?;

    let can_restore = can_restore_saved_path(path.as_deref());

    Ok(WallpaperRestoreStatus { path, can_restore })
}

pub(crate) async fn restore_previous_wallpaper(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<String, String> {
    let previous_path =
        db.0.lock()
            .get_preference(PREVIOUS_WALLPAPER_KEY)
            .map_err(|e| format!("Failed to read previous wallpaper: {}", e))?
            .ok_or_else(|| "No previous wallpaper has been saved yet".to_string())?;

    if !Path::new(&previous_path).exists() {
        return Err(format!(
            "Previous wallpaper file no longer exists: {}",
            previous_path
        ));
    }

    let result =
        wallpaper_lifecycle::restore_wallpaper_path(db, runtime, previous_path, "previous").await?;

    if let Err(error) = db.0.lock().delete_preference(PREVIOUS_WALLPAPER_KEY) {
        tracing::warn!("Failed to clear previous wallpaper path: {}", error);
    }

    Ok(result)
}

pub(crate) async fn restore_original_wallpaper(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<String, String> {
    wallpaper_lifecycle::restore_original_wallpaper(db, runtime).await
}

pub(crate) fn restore_original_wallpaper_blocking(
    app_handle: &tauri::AppHandle,
    runtime: &WallpaperRuntime,
) -> Result<String, String> {
    let db = app_handle
        .try_state::<AppDatabase>()
        .ok_or_else(|| "Database state is not initialized".to_string())?;

    wallpaper_lifecycle::restore_original_wallpaper_blocking(db.inner(), runtime)
}

fn can_restore_saved_path(path: Option<&str>) -> bool {
    path.map(|path| Path::new(path).exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-restore-{}-{}-{}.jpg",
            name,
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn saved_path_is_restorable_when_file_exists() {
        let path = unique_path("existing");
        fs::write(&path, b"wallpaper").expect("test wallpaper should be writable");

        assert!(can_restore_saved_path(Some(path.to_str().unwrap())));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn saved_path_is_not_restorable_when_missing_or_empty() {
        let path = unique_path("missing");

        assert!(!can_restore_saved_path(Some(path.to_str().unwrap())));
        assert!(!can_restore_saved_path(None));
    }
}
