use super::rows::wallpaper_from_row;
use super::{Wallpaper, WallpaperMetadata};
use rusqlite::{Connection, OptionalExtension, Result};

const WALLPAPER_SELECT: &str = "SELECT id, title, file_path, thumbnail_path, tags, width, height, fps, duration_ms, file_size_bytes, created_at, media_type, source, source_id, is_favorite
             FROM wallpapers";

pub(super) fn add(
    conn: &Connection,
    wallpaper: &WallpaperMetadata,
    file_path: &str,
    created_at: i64,
) -> Result<i64> {
    let tags_json = serde_json::to_string(&wallpaper.tags).unwrap_or_default();
    let file_size_bytes = std::fs::metadata(file_path)
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO wallpapers (title, file_path, thumbnail_path, tags, width, height, fps, duration_ms, file_size_bytes, created_at, media_type, source, source_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(file_path) DO UPDATE SET
                thumbnail_path = COALESCE(excluded.thumbnail_path, wallpapers.thumbnail_path),
                tags = excluded.tags,
                width = excluded.width,
                height = excluded.height,
                fps = excluded.fps,
                duration_ms = excluded.duration_ms,
                file_size_bytes = excluded.file_size_bytes,
                media_type = excluded.media_type,
                source = excluded.source,
                source_id = excluded.source_id",
        (
            &wallpaper.title,
            file_path,
            wallpaper.thumbnail_path.as_deref(),
            tags_json,
            wallpaper.width,
            wallpaper.height,
            wallpaper.fps,
            wallpaper.duration_ms,
            file_size_bytes,
            created_at,
            &wallpaper.media_type,
            wallpaper.source.as_deref(),
            wallpaper.source_id.as_deref(),
        ),
    )?;

    get_by_file_path(conn, file_path)?
        .map(|wallpaper| wallpaper.id)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub(super) fn get(conn: &Connection, id: i64) -> Result<Option<Wallpaper>> {
    let mut stmt = conn.prepare(&format!("{WALLPAPER_SELECT} WHERE id = ?1"))?;
    stmt.query_row([id], wallpaper_from_row).optional()
}

pub(super) fn list(conn: &Connection) -> Result<Vec<Wallpaper>> {
    let mut stmt = conn.prepare(&format!("{WALLPAPER_SELECT} ORDER BY created_at DESC"))?;

    let wallpapers = stmt
        .query_map([], wallpaper_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(wallpapers)
}

pub(super) fn list_favorites(conn: &Connection) -> Result<Vec<Wallpaper>> {
    let mut stmt = conn.prepare(&format!(
        "{WALLPAPER_SELECT}
             WHERE is_favorite = 1
             ORDER BY created_at DESC"
    ))?;

    let wallpapers = stmt
        .query_map([], wallpaper_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(wallpapers)
}

pub(super) fn get_by_file_path(conn: &Connection, file_path: &str) -> Result<Option<Wallpaper>> {
    let mut stmt = conn.prepare(&format!("{WALLPAPER_SELECT} WHERE file_path = ?1"))?;
    stmt.query_row([file_path], wallpaper_from_row).optional()
}

pub(super) fn search(conn: &Connection, query: &str) -> Result<Vec<Wallpaper>> {
    let search_pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(&format!(
        "{WALLPAPER_SELECT}
             WHERE title LIKE ?1 OR tags LIKE ?1
             ORDER BY created_at DESC"
    ))?;

    let wallpapers = stmt
        .query_map([&search_pattern], wallpaper_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(wallpapers)
}

pub(super) fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM collection_wallpapers WHERE wallpaper_id = ?1",
        [id],
    )?;
    conn.execute("DELETE FROM wallpapers WHERE id = ?1", [id])?;
    Ok(())
}

pub(super) fn set_favorite(
    conn: &Connection,
    id: i64,
    is_favorite: bool,
) -> Result<Option<Wallpaper>> {
    let changed = conn.execute(
        "UPDATE wallpapers SET is_favorite = ?1 WHERE id = ?2",
        (is_favorite, id),
    )?;

    if changed == 0 {
        return Ok(None);
    }

    get(conn, id)
}

pub(super) fn set_thumbnail(
    conn: &Connection,
    id: i64,
    thumbnail_path: Option<&str>,
) -> Result<Option<Wallpaper>> {
    let changed = conn.execute(
        "UPDATE wallpapers SET thumbnail_path = ?1 WHERE id = ?2",
        (thumbnail_path, id),
    )?;

    if changed == 0 {
        return Ok(None);
    }

    get(conn, id)
}

pub(super) fn get_by_source(
    conn: &Connection,
    source: &str,
    source_id: &str,
) -> Result<Option<Wallpaper>> {
    let mut stmt = conn.prepare(&format!(
        "{WALLPAPER_SELECT} WHERE source = ?1 AND source_id = ?2"
    ))?;

    stmt.query_row((source, source_id), wallpaper_from_row)
        .optional()
}
