use super::shell::reveal_path_in_explorer;
use crate::db::Wallpaper;
use crate::motionbgs::MOTIONBGS_SOURCE_NAME;
use crate::wallhaven::WALLHAVEN_SOURCE_NAME;
use crate::AppDatabase;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RemoveWallpaperResult {
    pub wallpaper: Wallpaper,
}

pub(crate) fn reveal_wallpaper_in_explorer(db: &AppDatabase, id: i64) -> Result<(), String> {
    if id <= 0 {
        return Err("Only library wallpapers can be revealed in Explorer".to_string());
    }

    let wallpaper =
        db.0.lock()
            .get_wallpaper(id)
            .map_err(|e| format!("Failed to read wallpaper: {}", e))?
            .ok_or_else(|| format!("Wallpaper {} was not found", id))?;

    let path = PathBuf::from(&wallpaper.file_path);
    if !path.exists() {
        return Err(format!("Wallpaper file is missing: {}", path.display()));
    }

    reveal_path_in_explorer(&path)
}

pub(crate) fn remove_wallpaper(db: &AppDatabase, id: i64) -> Result<RemoveWallpaperResult, String> {
    if id <= 0 {
        return Err("Only imported wallpapers can be removed".to_string());
    }

    let wallpaper =
        db.0.lock()
            .get_wallpaper(id)
            .map_err(|e| format!("Failed to read wallpaper before removal: {}", e))?
            .ok_or_else(|| format!("Wallpaper {} was not found", id))?;

    remove_cached_source_file(&wallpaper);

    db.0.lock()
        .delete_wallpaper(id)
        .map_err(|e| format!("Failed to remove wallpaper: {}", e))?;

    Ok(RemoveWallpaperResult { wallpaper })
}

pub(crate) fn set_wallpaper_favorite(
    db: &AppDatabase,
    id: i64,
    is_favorite: bool,
) -> Result<Wallpaper, String> {
    if id <= 0 {
        return Err("Only imported wallpapers can be favorited".to_string());
    }

    db.0.lock()
        .set_wallpaper_favorite(id, is_favorite)
        .map_err(|e| format!("Failed to update favorite: {}", e))?
        .ok_or_else(|| format!("Wallpaper {} was not found", id))
}

fn remove_cached_source_file(wallpaper: &Wallpaper) {
    let Some(source) = wallpaper.source.as_deref() else {
        return;
    };

    if !matches!(source, WALLHAVEN_SOURCE_NAME | MOTIONBGS_SOURCE_NAME) {
        return;
    }

    if let Err(error) = std::fs::remove_file(&wallpaper.file_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(
                "Failed to remove cached {} file '{}': {}",
                source,
                wallpaper.file_path,
                error
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn wallpaper(file_path: &Path, source: Option<&str>) -> Wallpaper {
        Wallpaper {
            id: 1,
            title: "Test Wallpaper".to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            thumbnail_path: None,
            tags: Vec::new(),
            width: 1920,
            height: 1080,
            fps: 0,
            duration_ms: 0,
            file_size_bytes: 0,
            created_at: 0,
            media_type: "image".to_string(),
            source: source.map(str::to_string),
            source_id: None,
            is_favorite: false,
        }
    }

    fn unique_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-library-{}-{}-{}.jpg",
            name,
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn cached_source_file_cleanup_removes_wallhaven_source_file() {
        let path = unique_path("wallhaven");
        fs::write(&path, b"image").expect("test image should be writable");

        remove_cached_source_file(&wallpaper(&path, Some(WALLHAVEN_SOURCE_NAME)));

        assert!(!path.exists());
    }

    #[test]
    fn cached_source_file_cleanup_removes_motionbgs_source_file() {
        let path = unique_path("motionbgs");
        fs::write(&path, b"video").expect("test video should be writable");

        remove_cached_source_file(&wallpaper(&path, Some(MOTIONBGS_SOURCE_NAME)));

        assert!(!path.exists());
    }

    #[test]
    fn cached_source_file_cleanup_keeps_non_cached_source_file() {
        let path = unique_path("local");
        fs::write(&path, b"image").expect("test image should be writable");

        remove_cached_source_file(&wallpaper(&path, None));

        assert!(path.exists());
        let _ = fs::remove_file(path);
    }
}
