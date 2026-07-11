use rusqlite::{Connection, OptionalExtension, Result};

pub(super) fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO preferences (key, value) VALUES (?1, ?2)",
        (key, value),
    )?;
    Ok(())
}

pub(super) fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1")?;
    stmt.query_row([key], |row| row.get(0)).optional()
}

pub(super) fn delete(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM preferences WHERE key = ?1", [key])?;
    Ok(())
}
