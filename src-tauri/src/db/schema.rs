use rusqlite::{Connection, Result};

pub(super) fn initialize(conn: &Connection) -> Result<()> {
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS wallpapers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                thumbnail_path TEXT,
                tags TEXT,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                fps INTEGER NOT NULL DEFAULT 60,
                duration_ms INTEGER NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                media_type TEXT NOT NULL DEFAULT 'video',
                source TEXT,
                source_id TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0
            )",
        [],
    )?;
    ensure_column(
        conn,
        "wallpapers",
        "media_type",
        "TEXT NOT NULL DEFAULT 'video'",
    )?;
    ensure_column(conn, "wallpapers", "source", "TEXT")?;
    ensure_column(conn, "wallpapers", "source_id", "TEXT")?;
    ensure_column(
        conn,
        "wallpapers",
        "is_favorite",
        "INTEGER NOT NULL DEFAULT 0",
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wallpapers_title ON wallpapers(title)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wallpapers_created_at
             ON wallpapers(created_at DESC, id DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wallpapers_source ON wallpapers(source, source_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                created_at INTEGER NOT NULL
            )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS collection_wallpapers (
                collection_id INTEGER NOT NULL,
                wallpaper_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (collection_id, wallpaper_id),
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
                FOREIGN KEY (wallpaper_id) REFERENCES wallpapers(id) ON DELETE CASCADE
            )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_collection_wallpapers_wallpaper
             ON collection_wallpapers(wallpaper_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS saved_filters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL COLLATE NOCASE,
                filter_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(filter_type, name)
            )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_saved_filters_type
             ON saved_filters(filter_type, name)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS wallpaper_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallpaper_id INTEGER,
                title TEXT NOT NULL,
                file_path TEXT NOT NULL,
                thumbnail_path TEXT,
                media_type TEXT NOT NULL,
                target_monitor_id TEXT,
                apply_source TEXT NOT NULL,
                applied_at INTEGER NOT NULL,
                FOREIGN KEY (wallpaper_id) REFERENCES wallpapers(id) ON DELETE SET NULL
            )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wallpaper_history_applied_at
             ON wallpaper_history(applied_at DESC, id DESC)",
        [],
    )?;

    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>>>()?
        .iter()
        .any(|name| name == column);

    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }

    Ok(())
}
