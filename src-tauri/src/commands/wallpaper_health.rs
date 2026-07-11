use crate::db::Wallpaper;
use crate::importing::is_supported_video_path;
use crate::wallpaper_media::{is_supported_image_path, is_supported_wallpaper_path};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct BrokenWallpaper {
    pub wallpaper: Wallpaper,
    pub reason: String,
}

pub(crate) fn detect_broken_wallpapers(wallpapers: Vec<Wallpaper>) -> Vec<BrokenWallpaper> {
    let mut broken = Vec::new();
    for wallpaper in wallpapers {
        if let Some(reason) = broken_wallpaper_reason(&wallpaper) {
            broken.push(BrokenWallpaper { wallpaper, reason });
        }
    }

    broken
}

fn broken_wallpaper_reason(wallpaper: &Wallpaper) -> Option<String> {
    let path = Path::new(&wallpaper.file_path);

    if !path.exists() {
        return Some("File is missing".to_string());
    }

    if !path.is_file() {
        return Some("Path is not a file".to_string());
    }

    if let Err(error) = std::fs::File::open(path) {
        return Some(format!("File cannot be opened: {}", error));
    }

    if !is_supported_wallpaper_path(path) {
        return Some("Wallpaper format is no longer supported".to_string());
    }

    if wallpaper.media_type == "video" && !is_supported_video_path(path) {
        return Some("Video format is no longer supported".to_string());
    }

    if wallpaper.media_type == "image" && !is_supported_image_path(path) {
        return Some("Image format is no longer supported".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn wallpaper(file_path: &str, media_type: &str) -> Wallpaper {
        Wallpaper {
            id: 1,
            title: "Test Wallpaper".to_string(),
            file_path: file_path.to_string(),
            thumbnail_path: None,
            tags: Vec::new(),
            width: 1920,
            height: 1080,
            fps: 0,
            duration_ms: 0,
            file_size_bytes: 0,
            created_at: 0,
            media_type: media_type.to_string(),
            source: None,
            source_id: None,
            is_favorite: false,
        }
    }

    fn unique_path(name: &str, extension: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-health-{}-{}-{}.{}",
            name,
            std::process::id(),
            stamp,
            extension
        ))
    }

    #[test]
    fn reports_missing_wallpaper_file() {
        let missing_path = unique_path("missing", "jpg");
        let broken =
            detect_broken_wallpapers(vec![wallpaper(missing_path.to_str().unwrap(), "image")]);

        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].reason, "File is missing");
    }

    #[test]
    fn accepts_existing_supported_image_file() {
        let image_path = unique_path("existing", "jpg");
        fs::write(&image_path, b"not a real image but enough for file checks")
            .expect("test image file should be writable");

        let broken =
            detect_broken_wallpapers(vec![wallpaper(image_path.to_str().unwrap(), "image")]);

        assert!(broken.is_empty());

        let _ = fs::remove_file(image_path);
    }
}
