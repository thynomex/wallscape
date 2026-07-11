use crate::importing::is_supported_video_path;
use std::path::Path;

const STATIC_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp"];

pub(crate) fn is_supported_image_path(path: &Path) -> bool {
    is_supported_static_image_path(path)
}

pub(crate) fn is_supported_static_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            STATIC_IMAGE_EXTENSIONS
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(ext))
        })
        .unwrap_or(false)
}

pub(crate) fn is_supported_wallpaper_path(path: &Path) -> bool {
    is_supported_static_image_path(path) || is_supported_video_path(path)
}

pub(crate) fn should_apply_as_static_wallpaper(media_type: &str, wallpaper_path: &str) -> bool {
    media_type == "image" && is_supported_static_image_path(Path::new(wallpaper_path))
}
