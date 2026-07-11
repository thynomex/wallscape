use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

static THUMBNAIL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportProbe {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub fps: Option<i32>,
    pub duration_ms: Option<i64>,
    pub thumbnail_data_url: Option<String>,
    pub warnings: Option<Vec<String>>,
}

pub struct NormalizedImportProbe {
    pub width: i32,
    pub height: i32,
    pub fps: i32,
    pub duration_ms: i64,
    pub thumbnail_path: Option<String>,
    pub warnings: Vec<String>,
}

pub fn normalize_import_path(video_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(video_path.trim());
    if path.as_os_str().is_empty() {
        return Err("No video file was selected".to_string());
    }

    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|e| format!("Failed to resolve selected video path: {}", e))
    }
}

pub fn validate_video_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "Selected video file does not exist: {}",
            path.display()
        ));
    }

    if !path.is_file() {
        return Err(format!("Selected path is not a file: {}", path.display()));
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    if !supported_video_extensions().contains(&extension.as_str()) {
        return Err(format!(
            "Unsupported video format '{}'. Choose MP4, MOV, WebM, MKV, AVI, or WMV.",
            extension
        ));
    }

    Ok(())
}

pub fn is_supported_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            supported_video_extensions()
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

pub fn supported_video_extensions() -> &'static [&'static str] {
    &["mp4", "mov", "m4v", "webm", "mkv", "avi", "wmv"]
}

pub fn normalize_import_probe(
    app_handle: &tauri::AppHandle,
    probe: Option<ImportProbe>,
) -> NormalizedImportProbe {
    let mut warnings = probe
        .as_ref()
        .and_then(|probe| probe.warnings.clone())
        .unwrap_or_default();

    let width = probe
        .as_ref()
        .and_then(|probe| probe.width)
        .filter(|width| (1..=16_384).contains(width))
        .unwrap_or_else(|| {
            warnings.push("Video width could not be probed; using 1920.".to_string());
            1920
        });

    let height = probe
        .as_ref()
        .and_then(|probe| probe.height)
        .filter(|height| (1..=16_384).contains(height))
        .unwrap_or_else(|| {
            warnings.push("Video height could not be probed; using 1080.".to_string());
            1080
        });

    let fps = probe
        .as_ref()
        .and_then(|probe| probe.fps)
        .filter(|fps| (1..=240).contains(fps))
        .unwrap_or_else(|| {
            warnings.push("Video frame rate could not be estimated; using 60fps.".to_string());
            60
        });

    let duration_ms = probe
        .as_ref()
        .and_then(|probe| probe.duration_ms)
        .filter(|duration_ms| *duration_ms >= 0 && *duration_ms <= 24 * 60 * 60 * 1000)
        .unwrap_or_else(|| {
            warnings.push("Video duration could not be probed.".to_string());
            0
        });

    let thumbnail_path = probe
        .and_then(|probe| probe.thumbnail_data_url)
        .and_then(|thumbnail| store_thumbnail_data_url(app_handle, thumbnail, &mut warnings));

    NormalizedImportProbe {
        width,
        height,
        fps,
        duration_ms,
        thumbnail_path,
        warnings,
    }
}

pub fn title_from_path(path: &Path) -> String {
    let raw_title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Imported Video");

    raw_title
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn tags_from_path(path: &Path) -> Vec<String> {
    let mut tags = vec!["local".to_string(), "imported".to_string()];

    if let Some(extension) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        tags.push(extension);
    }

    tags
}

fn store_thumbnail_data_url(
    app_handle: &tauri::AppHandle,
    thumbnail: String,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let (mime, bytes) = match decode_thumbnail_data_url(&thumbnail) {
        Ok(result) => result,
        Err(error) => {
            warnings.push(error);
            return None;
        }
    };

    let cache_dir = match app_handle.path().app_data_dir() {
        Ok(dir) => dir.join("thumbnails"),
        Err(error) => {
            warnings.push(format!(
                "Failed to resolve thumbnail cache directory: {}",
                error
            ));
            return None;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&cache_dir) {
        warnings.push(format!(
            "Failed to create thumbnail cache directory: {}",
            error
        ));
        return None;
    }

    let ext = match mime.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => {
            warnings.push("Generated thumbnail used an unsupported image format.".to_string());
            return None;
        }
    };

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let seq = THUMBNAIL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = format!("thumbnail-{stamp}-{seq}.{ext}");
    let path = cache_dir.join(file_name);

    if let Err(error) = std::fs::write(&path, bytes) {
        warnings.push(format!("Failed to save thumbnail cache file: {}", error));
        return None;
    }

    Some(path.to_string_lossy().to_string())
}

fn decode_thumbnail_data_url(thumbnail: &str) -> Result<(String, Vec<u8>), String> {
    const JPEG_PREFIX: &str = "data:image/jpeg;base64,";
    const PNG_PREFIX: &str = "data:image/png;base64,";
    const WEBP_PREFIX: &str = "data:image/webp;base64,";

    let (mime, payload) = if let Some(rest) = thumbnail.strip_prefix(JPEG_PREFIX) {
        ("image/jpeg", rest)
    } else if let Some(rest) = thumbnail.strip_prefix(PNG_PREFIX) {
        ("image/png", rest)
    } else if let Some(rest) = thumbnail.strip_prefix(WEBP_PREFIX) {
        ("image/webp", rest)
    } else {
        return Err("Generated thumbnail used an unsupported image format.".to_string());
    };

    if thumbnail.len() > 2_000_000 {
        return Err("Generated thumbnail was too large to save.".to_string());
    }

    let bytes = BASE64_STANDARD
        .decode(payload)
        .map_err(|error| format!("Failed to decode generated thumbnail: {}", error))?;

    Ok((mime.to_string(), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "wallscape-importing-test-{}-{}",
            std::process::id(),
            name
        ))
    }

    #[test]
    fn normalize_import_path_rejects_blank_selection() {
        let error = normalize_import_path("   ").expect_err("blank path should fail");
        assert_eq!(error, "No video file was selected");
    }

    #[test]
    fn title_from_path_removes_separator_noise() {
        let title = title_from_path(Path::new("C:/videos/forest_mist-loop.mp4"));
        assert_eq!(title, "forest mist loop");
    }

    #[test]
    fn tags_from_path_includes_lowercase_extension() {
        let tags = tags_from_path(Path::new("C:/videos/demo.MP4"));
        assert_eq!(tags, vec!["local", "imported", "mp4"]);
    }

    #[test]
    fn validate_video_file_accepts_supported_file() {
        let path = temp_path("supported.mp4");
        std::fs::write(&path, b"video").expect("test file should be writable");

        let result = validate_video_file(&path);

        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_video_file_rejects_unsupported_extension() {
        let path = temp_path("unsupported.txt");
        std::fs::write(&path, b"not-video").expect("test file should be writable");

        let error = validate_video_file(&path).expect_err("txt should be rejected");

        let _ = std::fs::remove_file(&path);
        assert!(error.contains("Unsupported video format 'txt'"));
    }

    #[test]
    fn decode_thumbnail_data_url_accepts_supported_mime() {
        let (mime, bytes) = decode_thumbnail_data_url("data:image/png;base64,aGVsbG8=")
            .expect("png data URL should decode");

        assert_eq!(mime, "image/png");
        assert_eq!(bytes, b"hello");
    }

    #[test]
    fn decode_thumbnail_data_url_rejects_unsupported_mime() {
        let error = decode_thumbnail_data_url("data:text/plain;base64,aGVsbG8=")
            .expect_err("text data URL should be rejected");

        assert_eq!(
            error,
            "Generated thumbnail used an unsupported image format."
        );
    }
}
