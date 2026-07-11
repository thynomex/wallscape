use super::rows::collection_from_row;
use super::{Collection, CollectionMembership};
use rusqlite::{Connection, OptionalExtension, Result};

const COLLECTION_WITH_COUNT_SELECT: &str =
    "SELECT c.id, c.name, c.created_at, COUNT(cw.wallpaper_id) AS wallpaper_count
             FROM collections c
             LEFT JOIN collection_wallpapers cw ON cw.collection_id = c.id";

pub(super) fn list(conn: &Connection) -> Result<Vec<Collection>> {
    let mut stmt = conn.prepare(&format!(
        "{COLLECTION_WITH_COUNT_SELECT}
             GROUP BY c.id, c.name, c.created_at
             ORDER BY c.name COLLATE NOCASE"
    ))?;

    let collections = stmt
        .query_map([], collection_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(collections)
}

pub(super) fn get(conn: &Connection, id: i64) -> Result<Option<Collection>> {
    let mut stmt = conn.prepare(&format!(
        "{COLLECTION_WITH_COUNT_SELECT}
             WHERE c.id = ?1
             GROUP BY c.id, c.name, c.created_at"
    ))?;

    stmt.query_row([id], collection_from_row).optional()
}

pub(super) fn create(conn: &Connection, name: &str, created_at: i64) -> Result<Collection> {
    conn.execute(
        "INSERT INTO collections (name, created_at) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET name = excluded.name",
        (name, created_at),
    )?;

    get_by_name(conn, name)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub(super) fn delete(conn: &Connection, id: i64) -> Result<bool> {
    let changed = conn.execute("DELETE FROM collections WHERE id = ?1", [id])?;
    Ok(changed > 0)
}

pub(super) fn set_membership(
    conn: &Connection,
    collection_id: i64,
    wallpaper_id: i64,
    in_collection: bool,
    created_at: i64,
) -> Result<Option<Collection>> {
    if get(conn, collection_id)?.is_none() || !wallpaper_exists(conn, wallpaper_id)? {
        return Ok(None);
    }

    if in_collection {
        conn.execute(
            "INSERT OR IGNORE INTO collection_wallpapers
                 (collection_id, wallpaper_id, created_at)
                 VALUES (?1, ?2, ?3)",
            (collection_id, wallpaper_id, created_at),
        )?;
    } else {
        conn.execute(
            "DELETE FROM collection_wallpapers
                 WHERE collection_id = ?1 AND wallpaper_id = ?2",
            (collection_id, wallpaper_id),
        )?;
    }

    get(conn, collection_id)
}

pub(super) fn list_memberships(conn: &Connection) -> Result<Vec<CollectionMembership>> {
    let mut stmt = conn.prepare(
        "SELECT collection_id, wallpaper_id
             FROM collection_wallpapers
             ORDER BY collection_id, created_at DESC",
    )?;

    let memberships = stmt
        .query_map([], |row| {
            Ok(CollectionMembership {
                collection_id: row.get(0)?,
                wallpaper_id: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(memberships)
}

fn get_by_name(conn: &Connection, name: &str) -> Result<Option<Collection>> {
    let mut stmt = conn.prepare(&format!(
        "{COLLECTION_WITH_COUNT_SELECT}
             WHERE c.name = ?1 COLLATE NOCASE
             GROUP BY c.id, c.name, c.created_at"
    ))?;

    stmt.query_row([name], collection_from_row).optional()
}

fn wallpaper_exists(conn: &Connection, wallpaper_id: i64) -> Result<bool> {
    conn.query_row(
        "SELECT 1 FROM wallpapers WHERE id = ?1",
        [wallpaper_id],
        |_| Ok(()),
    )
    .optional()
    .map(|result| result.is_some())
}
