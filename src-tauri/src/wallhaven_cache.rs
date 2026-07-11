use std::path::PathBuf;
use tauri::Manager;

pub(crate) fn store_wallhaven_image(
    app_handle: &tauri::AppHandle,
    source_id: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let cache_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve Wallhaven cache directory: {}", e))?
        .join("wallhaven");

    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create Wallhaven cache directory: {}", e))?;

    let source_id = wallhaven_source_id(source_id)?;
    let ext = wallhaven_image_extension(extension)?;
    let path = cache_dir.join(format!("{source_id}.{ext}"));
    std::fs::write(&path, bytes).map_err(|e| format!("Failed to save Wallhaven image: {}", e))?;
    Ok(path)
}

fn wallhaven_source_id(source_id: &str) -> Result<String, String> {
    let source_id = source_id.trim();
    if !source_id.is_empty() && source_id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        Ok(source_id.to_string())
    } else {
        Err("Invalid Wallhaven source id".to_string())
    }
}

fn wallhaven_image_extension(extension: &str) -> Result<&'static str, String> {
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Ok("jpg"),
        "png" => Ok("png"),
        _ => Err("Unsupported Wallhaven image format".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallhaven_image_extension_normalizes_jpeg_variants() {
        assert_eq!(wallhaven_image_extension("jpg"), Ok("jpg"));
        assert_eq!(wallhaven_image_extension("JPEG"), Ok("jpg"));
    }

    #[test]
    fn wallhaven_image_extension_accepts_png() {
        assert_eq!(wallhaven_image_extension("PNG"), Ok("png"));
    }

    #[test]
    fn wallhaven_image_extension_rejects_unknown_formats() {
        assert_eq!(
            wallhaven_image_extension("gif"),
            Err("Unsupported Wallhaven image format".to_string()),
        );
    }

    #[test]
    fn wallhaven_source_id_rejects_path_characters() {
        assert_eq!(
            wallhaven_source_id("../abc123"),
            Err("Invalid Wallhaven source id".to_string()),
        );
    }
}
