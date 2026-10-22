use crate::downloads::StagedDownload;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub(crate) fn store_motionbgs_video<T>(
    source_id: &str,
    extension: &str,
    download: StagedDownload,
    register: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    let source_id = motionbgs_source_id(source_id)?;
    let ext = motionbgs_video_extension(extension)?;
    download.commit(&source_id, ext, register)
}

pub(crate) fn motionbgs_cache_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve MotionBGS cache directory: {}", e))?
        .join("motionbgs"))
}

fn motionbgs_source_id(source_id: &str) -> Result<String, String> {
    let source_id = source_id.trim();
    if !source_id.is_empty()
        && source_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        Ok(source_id.to_string())
    } else {
        Err("Invalid MotionBGS source id".to_string())
    }
}

fn motionbgs_video_extension(extension: &str) -> Result<&'static str, String> {
    match extension.to_ascii_lowercase().as_str() {
        "mp4" => Ok("mp4"),
        _ => Err("Unsupported MotionBGS video format".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motionbgs_video_extension_accepts_mp4() {
        assert_eq!(motionbgs_video_extension("MP4"), Ok("mp4"));
    }

    #[test]
    fn motionbgs_video_extension_rejects_unknown_formats() {
        assert_eq!(
            motionbgs_video_extension("webm"),
            Err("Unsupported MotionBGS video format".to_string()),
        );
    }

    #[test]
    fn motionbgs_source_id_rejects_path_characters() {
        assert_eq!(
            motionbgs_source_id("../1964"),
            Err("Invalid MotionBGS source id".to_string()),
        );
    }
}
