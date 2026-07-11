use crate::db::Wallpaper;
use crate::wallhaven::WALLHAVEN_SOURCE_NAME;
use crate::AppDatabase;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct StorageStats {
    pub total_cache_bytes: i64,
    pub wallhaven_cache_bytes: i64,
    pub wallhaven_cache_files: i64,
    pub unused_wallhaven_cache_bytes: i64,
    pub unused_wallhaven_cache_files: i64,
    pub wallhaven_library_entries: i64,
    pub thumbnail_cache_bytes: i64,
    pub thumbnail_cache_files: i64,
    pub unused_thumbnail_bytes: i64,
    pub unused_thumbnail_files: i64,
    pub missing_library_entries: i64,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct StorageCleanupResult {
    pub removed_files: i64,
    pub removed_bytes: i64,
    pub removed_entries: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct CacheFile {
    path: PathBuf,
    bytes: i64,
}

pub(crate) fn get_storage_stats(
    app_handle: &tauri::AppHandle,
    db: &AppDatabase,
) -> Result<StorageStats, String> {
    let app_data_dir = app_data_dir(app_handle)?;
    let wallpapers =
        db.0.lock()
            .list_wallpapers()
            .map_err(|e| format!("Failed to list wallpapers: {}", e))?;

    Ok(storage_stats_for_paths(&app_data_dir, &wallpapers))
}

pub(crate) fn clear_wallhaven_cache(
    app_handle: &tauri::AppHandle,
    db: &AppDatabase,
) -> Result<StorageCleanupResult, String> {
    let app_data_dir = app_data_dir(app_handle)?;
    let wallpapers =
        db.0.lock()
            .list_wallpapers()
            .map_err(|e| format!("Failed to list wallpapers: {}", e))?;
    Ok(remove_unused_wallhaven_cache_for_paths(
        &app_data_dir,
        &wallpapers,
    ))
}

pub(crate) fn cleanup_unused_thumbnails(
    app_handle: &tauri::AppHandle,
    db: &AppDatabase,
) -> Result<StorageCleanupResult, String> {
    let app_data_dir = app_data_dir(app_handle)?;
    let wallpapers =
        db.0.lock()
            .list_wallpapers()
            .map_err(|e| format!("Failed to list wallpapers: {}", e))?;

    Ok(remove_unused_thumbnails_for_paths(
        &app_data_dir,
        &wallpapers,
    ))
}

pub(crate) fn cleanup_missing_library_entries(
    db: &AppDatabase,
) -> Result<StorageCleanupResult, String> {
    let wallpapers =
        db.0.lock()
            .list_wallpapers()
            .map_err(|e| format!("Failed to list wallpapers: {}", e))?;
    let missing = wallpapers
        .iter()
        .filter(|wallpaper| is_missing_library_entry(wallpaper))
        .cloned()
        .collect::<Vec<_>>();

    let removed_entries = delete_wallpapers(db, &missing)?;
    Ok(StorageCleanupResult {
        removed_files: 0,
        removed_bytes: 0,
        removed_entries,
        warnings: Vec::new(),
    })
}

fn app_data_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))
}

fn storage_stats_for_paths(app_data_dir: &Path, wallpapers: &[Wallpaper]) -> StorageStats {
    let wallhaven_files = collect_cache_files(&wallhaven_cache_dir(app_data_dir));
    let thumbnail_files = collect_cache_files(&thumbnail_cache_dir(app_data_dir));
    let unused_wallhaven = unused_wallhaven_cache_files(app_data_dir, wallpapers, &wallhaven_files);
    let unused_thumbnails = unused_thumbnail_files(app_data_dir, wallpapers, &thumbnail_files);

    let wallhaven_cache_bytes = sum_bytes(&wallhaven_files);
    let thumbnail_cache_bytes = sum_bytes(&thumbnail_files);

    StorageStats {
        total_cache_bytes: wallhaven_cache_bytes + thumbnail_cache_bytes,
        wallhaven_cache_bytes,
        wallhaven_cache_files: wallhaven_files.len() as i64,
        unused_wallhaven_cache_bytes: sum_bytes(&unused_wallhaven),
        unused_wallhaven_cache_files: unused_wallhaven.len() as i64,
        wallhaven_library_entries: wallpapers
            .iter()
            .filter(|wallpaper| wallpaper.source.as_deref() == Some(WALLHAVEN_SOURCE_NAME))
            .count() as i64,
        thumbnail_cache_bytes,
        thumbnail_cache_files: thumbnail_files.len() as i64,
        unused_thumbnail_bytes: sum_bytes(&unused_thumbnails),
        unused_thumbnail_files: unused_thumbnails.len() as i64,
        missing_library_entries: wallpapers
            .iter()
            .filter(|wallpaper| is_missing_library_entry(wallpaper))
            .count() as i64,
    }
}

fn remove_unused_wallhaven_cache_for_paths(
    app_data_dir: &Path,
    wallpapers: &[Wallpaper],
) -> StorageCleanupResult {
    let wallhaven_files = collect_cache_files(&wallhaven_cache_dir(app_data_dir));
    let unused = unused_wallhaven_cache_files(app_data_dir, wallpapers, &wallhaven_files);
    let result = remove_files(unused);
    remove_empty_dir(&wallhaven_cache_dir(app_data_dir));
    result
}

fn remove_unused_thumbnails_for_paths(
    app_data_dir: &Path,
    wallpapers: &[Wallpaper],
) -> StorageCleanupResult {
    let thumbnail_files = collect_cache_files(&thumbnail_cache_dir(app_data_dir));
    let unused = unused_thumbnail_files(app_data_dir, wallpapers, &thumbnail_files);
    remove_files(unused)
}

fn wallhaven_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("wallhaven")
}

fn thumbnail_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("thumbnails")
}

fn collect_cache_files(root: &Path) -> Vec<CacheFile> {
    let mut files = Vec::new();
    collect_cache_files_inner(root, &mut files);
    files
}

fn collect_cache_files_inner(path: &Path, files: &mut Vec<CacheFile>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };

    if metadata.is_file() {
        files.push(CacheFile {
            path: path.to_path_buf(),
            bytes: metadata.len() as i64,
        });
        return;
    }

    if !metadata.is_dir() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_cache_files_inner(&entry.path(), files);
    }
}

fn unused_wallhaven_cache_files(
    app_data_dir: &Path,
    wallpapers: &[Wallpaper],
    wallhaven_files: &[CacheFile],
) -> Vec<CacheFile> {
    unused_app_cache_files(
        &wallhaven_cache_dir(app_data_dir),
        wallpapers
            .iter()
            .filter(|wallpaper| wallpaper.source.as_deref() == Some(WALLHAVEN_SOURCE_NAME))
            .map(|wallpaper| wallpaper.file_path.as_str()),
        wallhaven_files,
    )
}

fn unused_thumbnail_files(
    app_data_dir: &Path,
    wallpapers: &[Wallpaper],
    thumbnail_files: &[CacheFile],
) -> Vec<CacheFile> {
    unused_app_cache_files(
        &thumbnail_cache_dir(app_data_dir),
        wallpapers
            .iter()
            .filter_map(|wallpaper| wallpaper.thumbnail_path.as_deref()),
        thumbnail_files,
    )
}

fn unused_app_cache_files<'a>(
    cache_dir: &Path,
    used_paths: impl Iterator<Item = &'a str>,
    cache_files: &[CacheFile],
) -> Vec<CacheFile> {
    let Some(cache_root) = canonical_path(cache_dir) else {
        return Vec::new();
    };

    let used_cache_paths = used_paths
        .filter_map(|path| canonical_path(Path::new(path)))
        .filter(|path| path.starts_with(&cache_root))
        .collect::<HashSet<_>>();

    cache_files
        .iter()
        .filter(|file| {
            canonical_path(&file.path)
                .map(|path| !used_cache_paths.contains(&path))
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

fn canonical_path(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

fn remove_files(files: Vec<CacheFile>) -> StorageCleanupResult {
    let mut result = StorageCleanupResult {
        removed_files: 0,
        removed_bytes: 0,
        removed_entries: 0,
        warnings: Vec::new(),
    };

    for file in files {
        match std::fs::remove_file(&file.path) {
            Ok(()) => {
                result.removed_files += 1;
                result.removed_bytes += file.bytes;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => result.warnings.push(format!(
                "Failed to remove '{}': {}",
                file.path.display(),
                error
            )),
        }
    }

    result
}

fn remove_empty_dir(path: &Path) {
    let _ = std::fs::remove_dir(path);
}

fn delete_wallpapers(db: &AppDatabase, wallpapers: &[Wallpaper]) -> Result<i64, String> {
    let db_guard = db.0.lock();
    let mut removed = 0;

    for wallpaper in wallpapers {
        db_guard
            .delete_wallpaper(wallpaper.id)
            .map_err(|e| format!("Failed to remove library entry: {}", e))?;
        removed += 1;
    }

    Ok(removed)
}

fn is_missing_library_entry(wallpaper: &Wallpaper) -> bool {
    let path = Path::new(&wallpaper.file_path);
    if !path.exists() || !path.is_file() {
        return true;
    }

    std::fs::File::open(path).is_err()
}

fn sum_bytes(files: &[CacheFile]) -> i64 {
    files.iter().map(|file| file.bytes).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, WallpaperMetadata};
    use parking_lot::Mutex;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wallscape-storage-{}-{}-{}",
            name,
            std::process::id(),
            stamp
        ))
    }

    fn sample_metadata(
        title: &str,
        thumbnail_path: Option<&Path>,
        source: Option<&str>,
    ) -> WallpaperMetadata {
        WallpaperMetadata {
            title: title.to_string(),
            thumbnail_path: thumbnail_path.map(|path| path.to_string_lossy().to_string()),
            tags: Vec::new(),
            width: 1920,
            height: 1080,
            fps: 60,
            duration_ms: 10_000,
            media_type: "video".to_string(),
            source: source.map(str::to_string),
            source_id: source.map(|_| "abc123".to_string()),
        }
    }

    fn test_db(path: &Path) -> AppDatabase {
        AppDatabase(Mutex::new(
            Database::new(path.join("wallscape.db")).expect("database should open"),
        ))
    }

    #[test]
    fn stats_counts_wallhaven_thumbnails_and_missing_entries() {
        let root = unique_temp_dir("stats");
        let wallhaven_dir = wallhaven_cache_dir(&root);
        let thumbnails_dir = thumbnail_cache_dir(&root);
        fs::create_dir_all(&wallhaven_dir).expect("wallhaven dir");
        fs::create_dir_all(&thumbnails_dir).expect("thumbnail dir");

        let wallhaven_file = wallhaven_dir.join("abc123.jpg");
        let orphan_wallhaven_file = wallhaven_dir.join("orphan.jpg");
        let used_thumb = thumbnails_dir.join("used.jpg");
        let unused_thumb = thumbnails_dir.join("unused.jpg");
        let local_file = root.join("local.mp4");
        fs::write(&wallhaven_file, b"wallhaven").expect("wallhaven file");
        fs::write(&orphan_wallhaven_file, b"orphan").expect("orphan wallhaven file");
        fs::write(&used_thumb, b"used").expect("used thumb");
        fs::write(&unused_thumb, b"unused").expect("unused thumb");
        fs::write(&local_file, b"video").expect("local file");

        let db = test_db(&root);
        {
            let db_guard = db.0.lock();
            db_guard
                .add_wallpaper(
                    &sample_metadata("Wallhaven", Some(&used_thumb), Some(WALLHAVEN_SOURCE_NAME)),
                    wallhaven_file.to_str().unwrap(),
                )
                .expect("wallhaven row");
            db_guard
                .add_wallpaper(
                    &sample_metadata("Missing", None, None),
                    root.join("missing.mp4").to_str().unwrap(),
                )
                .expect("missing row");
            db_guard
                .add_wallpaper(
                    &sample_metadata("Local", None, None),
                    local_file.to_str().unwrap(),
                )
                .expect("local row");
        }

        let wallpapers = db.0.lock().list_wallpapers().expect("wallpapers");
        let stats = storage_stats_for_paths(&root, &wallpapers);

        assert_eq!(stats.wallhaven_cache_files, 2);
        assert_eq!(stats.wallhaven_cache_bytes, 15);
        assert_eq!(stats.unused_wallhaven_cache_files, 1);
        assert_eq!(stats.unused_wallhaven_cache_bytes, 6);
        assert_eq!(stats.thumbnail_cache_files, 2);
        assert_eq!(stats.unused_thumbnail_files, 1);
        assert_eq!(stats.unused_thumbnail_bytes, 6);
        assert_eq!(stats.wallhaven_library_entries, 1);
        assert_eq!(stats.missing_library_entries, 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clear_wallhaven_cache_keeps_saved_wallhaven_wallpapers() {
        let root = unique_temp_dir("wallhaven-cleanup");
        let wallhaven_dir = wallhaven_cache_dir(&root);
        fs::create_dir_all(&wallhaven_dir).expect("wallhaven dir");

        let saved_wallhaven_file = wallhaven_dir.join("saved.jpg");
        let orphan_wallhaven_file = wallhaven_dir.join("orphan.jpg");
        fs::write(&saved_wallhaven_file, b"saved").expect("saved wallhaven file");
        fs::write(&orphan_wallhaven_file, b"orphan").expect("orphan wallhaven file");

        let db = test_db(&root);
        db.0.lock()
            .add_wallpaper(
                &sample_metadata("Saved Wallhaven", None, Some(WALLHAVEN_SOURCE_NAME)),
                saved_wallhaven_file.to_str().unwrap(),
            )
            .expect("wallhaven row");

        let wallpapers = db.0.lock().list_wallpapers().expect("wallpapers");
        let result = remove_unused_wallhaven_cache_for_paths(&root, &wallpapers);
        let remaining = db.0.lock().list_wallpapers().expect("wallpapers");

        assert_eq!(result.removed_files, 1);
        assert_eq!(result.removed_bytes, 6);
        assert_eq!(result.removed_entries, 0);
        assert!(saved_wallhaven_file.exists());
        assert!(!orphan_wallhaven_file.exists());
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "Saved Wallhaven");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cleanup_unused_thumbnails_keeps_referenced_files() {
        let root = unique_temp_dir("thumb-cleanup");
        let thumbnails_dir = thumbnail_cache_dir(&root);
        fs::create_dir_all(&thumbnails_dir).expect("thumbnail dir");

        let used_thumb = thumbnails_dir.join("used.jpg");
        let unused_thumb = thumbnails_dir.join("unused.jpg");
        let local_file = root.join("local.mp4");
        fs::write(&used_thumb, b"used").expect("used thumb");
        fs::write(&unused_thumb, b"unused").expect("unused thumb");
        fs::write(&local_file, b"video").expect("local file");

        let db = test_db(&root);
        db.0.lock()
            .add_wallpaper(
                &sample_metadata("Local", Some(&used_thumb), None),
                local_file.to_str().unwrap(),
            )
            .expect("local row");

        let wallpapers = db.0.lock().list_wallpapers().expect("wallpapers");
        let result = remove_unused_thumbnails_for_paths(&root, &wallpapers);

        assert_eq!(result.removed_files, 1);
        assert_eq!(result.removed_bytes, 6);
        assert!(used_thumb.exists());
        assert!(!unused_thumb.exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cleanup_missing_library_entries_removes_only_missing_rows() {
        let root = unique_temp_dir("missing-cleanup");
        fs::create_dir_all(&root).expect("root dir");
        let local_file = root.join("local.mp4");
        fs::write(&local_file, b"video").expect("local file");

        let db = test_db(&root);
        {
            let db_guard = db.0.lock();
            db_guard
                .add_wallpaper(
                    &sample_metadata("Local", None, None),
                    local_file.to_str().unwrap(),
                )
                .expect("local row");
            db_guard
                .add_wallpaper(
                    &sample_metadata("Missing", None, None),
                    root.join("missing.mp4").to_str().unwrap(),
                )
                .expect("missing row");
        }

        let result = cleanup_missing_library_entries(&db).expect("cleanup");
        let remaining = db.0.lock().list_wallpapers().expect("wallpapers");

        assert_eq!(result.removed_entries, 1);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "Local");

        let _ = fs::remove_dir_all(&root);
    }
}
