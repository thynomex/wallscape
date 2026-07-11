use super::rows::wallpaper_history_from_row;
use super::{Wallpaper, WallpaperHistoryEntry, WallpaperHistoryRecord};
use rusqlite::{Connection, Result};

const HISTORY_SELECT: &str = "SELECT id, wallpaper_id, title, file_path, thumbnail_path, media_type, target_monitor_id, apply_source, applied_at
             FROM wallpaper_history";

pub(super) fn add(
    conn: &Connection,
    record: &WallpaperHistoryRecord,
    applied_at: i64,
) -> Result<i64> {
    if let Some(latest) = list(conn, 1)?.into_iter().next() {
        if path_matches(Some(&latest.file_path), &record.file_path)
            && latest.target_monitor_id.as_deref() == record.target_monitor_id.as_deref()
        {
            return Ok(latest.id);
        }
    }

    conn.execute(
        "INSERT INTO wallpaper_history
             (wallpaper_id, title, file_path, thumbnail_path, media_type, target_monitor_id, apply_source, applied_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            record.wallpaper_id,
            &record.title,
            &record.file_path,
            record.thumbnail_path.as_deref(),
            &record.media_type,
            record.target_monitor_id.as_deref(),
            &record.apply_source,
            applied_at,
        ),
    )?;

    Ok(conn.last_insert_rowid())
}

pub(super) fn list(conn: &Connection, limit: i64) -> Result<Vec<WallpaperHistoryEntry>> {
    let limit = limit.clamp(1, 100);
    let mut stmt = conn.prepare(&format!(
        "{HISTORY_SELECT}
             ORDER BY applied_at DESC, id DESC
             LIMIT ?1"
    ))?;

    let entries = stmt
        .query_map([limit], wallpaper_history_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(entries)
}

pub(super) fn latest_distinct_before(
    conn: &Connection,
    current_path: Option<&str>,
) -> Result<Option<WallpaperHistoryEntry>> {
    let entries = list(conn, 50)?;

    Ok(entries
        .into_iter()
        .find(|entry| !path_matches(current_path, &entry.file_path)))
}

pub(super) fn record_from_wallpaper(
    wallpaper: Option<Wallpaper>,
    file_path: &str,
    target_monitor_id: Option<&str>,
    apply_source: &str,
) -> WallpaperHistoryRecord {
    match wallpaper {
        Some(wallpaper) => WallpaperHistoryRecord {
            wallpaper_id: Some(wallpaper.id),
            title: wallpaper.title,
            file_path: wallpaper.file_path,
            thumbnail_path: wallpaper.thumbnail_path,
            media_type: wallpaper.media_type,
            target_monitor_id: target_monitor_id.map(str::to_string),
            apply_source: apply_source.to_string(),
        },
        None => WallpaperHistoryRecord {
            wallpaper_id: None,
            title: fallback_title(file_path),
            file_path: file_path.to_string(),
            thumbnail_path: None,
            media_type: fallback_media_type(file_path),
            target_monitor_id: target_monitor_id.map(str::to_string),
            apply_source: apply_source.to_string(),
        },
    }
}

fn path_matches(left: Option<&str>, right: &str) -> bool {
    let Some(left) = left else {
        return false;
    };

    if cfg!(windows) {
        left.replace('/', "\\")
            .eq_ignore_ascii_case(&right.replace('/', "\\"))
    } else {
        left == right
    }
}

fn fallback_title(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or(file_path)
        .to_string()
}

fn fallback_media_type(file_path: &str) -> String {
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("jpg" | "jpeg" | "png" | "bmp" | "webp") => "image".to_string(),
        _ => "video".to_string(),
    }
}
