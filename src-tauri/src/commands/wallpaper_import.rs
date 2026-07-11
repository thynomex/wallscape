use crate::db::{Wallpaper, WallpaperMetadata};
use crate::importing::{
    is_supported_video_path, normalize_import_path, normalize_import_probe, tags_from_path,
    title_from_path, validate_video_file, ImportProbe,
};
use crate::wallpaper_media::is_supported_image_path;
use crate::AppDatabase;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ImportWallpaperResult {
    pub wallpaper: Wallpaper,
    pub warnings: Vec<String>,
    pub duplicate: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BatchImportItem {
    pub video_path: String,
    pub probe: Option<ImportProbe>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ImportFailure {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct BatchImportResult {
    pub imported: Vec<Wallpaper>,
    pub duplicates: Vec<Wallpaper>,
    pub failed: Vec<ImportFailure>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ThumbnailRegenerationResult {
    pub wallpaper: Wallpaper,
    pub warnings: Vec<String>,
}

pub(crate) fn import_wallpaper(
    app_handle: &tauri::AppHandle,
    db: &AppDatabase,
    video_path: String,
    probe: Option<ImportProbe>,
) -> Result<ImportWallpaperResult, String> {
    let path = normalize_import_path(&video_path)?;
    validate_wallpaper_file(&path)?;

    let file_path = path.to_string_lossy().to_string();
    if let Some(existing) =
        db.0.lock()
            .get_wallpaper_by_file_path(&file_path)
            .map_err(|e| format!("Failed to check existing wallpaper: {}", e))?
    {
        return Ok(ImportWallpaperResult {
            wallpaper: existing,
            warnings: vec!["Already in library; skipped duplicate.".to_string()],
            duplicate: true,
        });
    }

    let (metadata, warnings) = import_metadata(app_handle, &path, probe);

    let id =
        db.0.lock()
            .add_wallpaper(&metadata, &file_path)
            .map_err(|e| format!("Failed to import wallpaper: {}", e))?;

    let wallpaper =
        db.0.lock()
            .get_wallpaper(id)
            .map_err(|e| format!("Failed to read imported wallpaper: {}", e))?
            .ok_or_else(|| "Imported wallpaper was not found after saving".to_string())?;

    Ok(ImportWallpaperResult {
        wallpaper,
        warnings,
        duplicate: false,
    })
}

pub(crate) fn import_wallpapers(
    app_handle: &tauri::AppHandle,
    db: &AppDatabase,
    items: Vec<BatchImportItem>,
) -> Result<BatchImportResult, String> {
    let mut imported = Vec::new();
    let mut duplicates = Vec::new();
    let mut failed = Vec::new();
    let mut warnings = Vec::new();

    for item in items {
        let source_path = item.video_path.clone();
        match import_wallpaper(app_handle, db, item.video_path, item.probe) {
            Ok(result) => {
                warnings.extend(
                    result
                        .warnings
                        .into_iter()
                        .map(|warning| format!("{}: {}", result.wallpaper.title, warning)),
                );

                if result.duplicate {
                    duplicates.push(result.wallpaper);
                } else {
                    imported.push(result.wallpaper);
                }
            }
            Err(reason) => failed.push(ImportFailure {
                path: source_path,
                reason,
            }),
        }
    }

    Ok(BatchImportResult {
        imported,
        duplicates,
        failed,
        warnings,
    })
}

pub(crate) fn regenerate_wallpaper_thumbnail(
    app_handle: &tauri::AppHandle,
    db: &AppDatabase,
    id: i64,
    probe: ImportProbe,
) -> Result<ThumbnailRegenerationResult, String> {
    if id <= 0 {
        return Err("Only imported wallpapers can refresh thumbnails".to_string());
    }

    let wallpaper =
        db.0.lock()
            .get_wallpaper(id)
            .map_err(|e| format!("Failed to read wallpaper: {}", e))?
            .ok_or_else(|| format!("Wallpaper {} was not found", id))?;

    if wallpaper.media_type != "video" {
        return Err("Thumbnail regeneration is only available for video wallpapers".to_string());
    }

    validate_video_file(Path::new(&wallpaper.file_path))?;

    let normalized = normalize_import_probe(app_handle, Some(probe));
    let mut warnings = normalized.warnings;
    let Some(thumbnail_path) = normalized.thumbnail_path else {
        return Err("Thumbnail preview could not be generated for this file".to_string());
    };

    let wallpaper =
        db.0.lock()
            .set_wallpaper_thumbnail(id, Some(&thumbnail_path))
            .map_err(|e| format!("Failed to update thumbnail: {}", e))?
            .ok_or_else(|| format!("Wallpaper {} was not found", id))?;

    if warnings.is_empty() {
        warnings.push("Thumbnail regenerated.".to_string());
    }

    Ok(ThumbnailRegenerationResult {
        wallpaper,
        warnings,
    })
}

fn validate_wallpaper_file(path: &Path) -> Result<(), String> {
    if is_supported_video_path(path) {
        return validate_video_file(path);
    }

    if !path.exists() {
        return Err(format!("Selected file does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Selected path is not a file: {}", path.display()));
    }

    if is_supported_image_path(path) {
        return Ok(());
    }

    Err("Unsupported wallpaper format. Choose a supported image or video file.".to_string())
}

fn import_metadata(
    app_handle: &tauri::AppHandle,
    path: &Path,
    probe: Option<ImportProbe>,
) -> (WallpaperMetadata, Vec<String>) {
    if is_supported_image_path(path) {
        let warnings = probe
            .as_ref()
            .and_then(|probe| probe.warnings.clone())
            .unwrap_or_default();

        let metadata = WallpaperMetadata {
            title: title_from_path(path),
            thumbnail_path: Some(path.to_string_lossy().to_string()),
            tags: tags_from_path(path),
            width: probe.as_ref().and_then(|probe| probe.width).unwrap_or(0),
            height: probe.as_ref().and_then(|probe| probe.height).unwrap_or(0),
            fps: 0,
            duration_ms: 0,
            media_type: "image".to_string(),
            source: None,
            source_id: None,
        };

        return (metadata, warnings);
    }

    let probe = normalize_import_probe(app_handle, probe);
    let mut warnings = probe.warnings;

    if probe.thumbnail_path.is_none() {
        warnings.push("Thumbnail preview could not be generated for this file.".to_string());
    }

    let metadata = WallpaperMetadata {
        title: title_from_path(path),
        thumbnail_path: probe.thumbnail_path,
        tags: tags_from_path(path),
        width: probe.width,
        height: probe.height,
        fps: probe.fps,
        duration_ms: probe.duration_ms,
        media_type: "video".to_string(),
        source: None,
        source_id: None,
    };

    (metadata, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(name: &str, extension: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-import-{}-{}-{}.{}",
            name,
            std::process::id(),
            stamp,
            extension
        ))
    }

    #[test]
    fn validate_wallpaper_file_accepts_supported_image() {
        let path = unique_path("image", "jpg");
        fs::write(&path, b"image").expect("test image should be writable");

        let result = validate_wallpaper_file(&path);

        let _ = fs::remove_file(path);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_wallpaper_file_rejects_unsupported_file() {
        let path = unique_path("unsupported", "txt");
        fs::write(&path, b"text").expect("test file should be writable");

        let error = validate_wallpaper_file(&path).expect_err("txt should be rejected");

        let _ = fs::remove_file(path);
        assert_eq!(
            error,
            "Unsupported wallpaper format. Choose a supported image or video file."
        );
    }
}
