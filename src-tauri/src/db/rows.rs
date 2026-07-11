use super::{Collection, SavedFilter, Wallpaper, WallpaperHistoryEntry};
use rusqlite::{Result, Row};

pub(super) fn wallpaper_from_row(row: &Row<'_>) -> Result<Wallpaper> {
    let tags_json: String = row.get(4)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

    Ok(Wallpaper {
        id: row.get(0)?,
        title: row.get(1)?,
        file_path: row.get(2)?,
        thumbnail_path: row.get(3)?,
        tags,
        width: row.get(5)?,
        height: row.get(6)?,
        fps: row.get(7)?,
        duration_ms: row.get(8)?,
        file_size_bytes: row.get(9)?,
        created_at: row.get(10)?,
        media_type: row.get(11)?,
        source: row.get(12)?,
        source_id: row.get(13)?,
        is_favorite: row.get(14)?,
    })
}

pub(super) fn collection_from_row(row: &Row<'_>) -> Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        wallpaper_count: row.get(3)?,
    })
}

pub(super) fn saved_filter_from_row(row: &Row<'_>) -> Result<SavedFilter> {
    Ok(SavedFilter {
        id: row.get(0)?,
        name: row.get(1)?,
        filter_type: row.get(2)?,
        payload: row.get(3)?,
        created_at: row.get(4)?,
    })
}

pub(super) fn wallpaper_history_from_row(row: &Row<'_>) -> Result<WallpaperHistoryEntry> {
    Ok(WallpaperHistoryEntry {
        id: row.get(0)?,
        wallpaper_id: row.get(1)?,
        title: row.get(2)?,
        file_path: row.get(3)?,
        thumbnail_path: row.get(4)?,
        media_type: row.get(5)?,
        target_monitor_id: row.get(6)?,
        apply_source: row.get(7)?,
        applied_at: row.get(8)?,
    })
}
