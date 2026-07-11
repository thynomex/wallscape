use super::rows::saved_filter_from_row;
use super::SavedFilter;
use rusqlite::{Connection, OptionalExtension, Result};

const SAVED_FILTER_SELECT: &str = "SELECT id, name, filter_type, payload, created_at
             FROM saved_filters";

pub(super) fn list(conn: &Connection, filter_type: Option<&str>) -> Result<Vec<SavedFilter>> {
    if let Some(filter_type) = filter_type {
        let mut stmt = conn.prepare(&format!(
            "{SAVED_FILTER_SELECT}
                 WHERE filter_type = ?1
                 ORDER BY name COLLATE NOCASE"
        ))?;

        let filters = stmt
            .query_map([filter_type], saved_filter_from_row)?
            .collect::<Result<Vec<_>>>()?;
        return Ok(filters);
    }

    let mut stmt = conn.prepare(&format!(
        "{SAVED_FILTER_SELECT} ORDER BY filter_type, name COLLATE NOCASE"
    ))?;

    let filters = stmt
        .query_map([], saved_filter_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(filters)
}

pub(super) fn save(
    conn: &Connection,
    name: &str,
    filter_type: &str,
    payload: &str,
    created_at: i64,
) -> Result<SavedFilter> {
    conn.execute(
        "INSERT INTO saved_filters (name, filter_type, payload, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(filter_type, name) DO UPDATE SET
                name = excluded.name,
                payload = excluded.payload",
        (name, filter_type, payload, created_at),
    )?;

    get_by_name(conn, filter_type, name)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub(super) fn delete(conn: &Connection, id: i64) -> Result<bool> {
    let changed = conn.execute("DELETE FROM saved_filters WHERE id = ?1", [id])?;
    Ok(changed > 0)
}

fn get_by_name(conn: &Connection, filter_type: &str, name: &str) -> Result<Option<SavedFilter>> {
    let mut stmt = conn.prepare(&format!(
        "{SAVED_FILTER_SELECT}
             WHERE filter_type = ?1 AND name = ?2 COLLATE NOCASE",
    ))?;

    stmt.query_row((filter_type, name), saved_filter_from_row)
        .optional()
}
