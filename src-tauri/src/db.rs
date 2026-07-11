mod collections;
mod preferences;
mod rows;
mod saved_filters;
mod schema;
mod wallpaper_history;
mod wallpapers;

use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Wallpaper {
    pub id: i64,
    pub title: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub tags: Vec<String>,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
    pub duration_ms: i64,
    pub file_size_bytes: i64,
    pub created_at: i64,
    pub media_type: String,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperMetadata {
    pub title: String,
    pub thumbnail_path: Option<String>,
    pub tags: Vec<String>,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
    pub duration_ms: i64,
    pub media_type: String,
    pub source: Option<String>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub wallpaper_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CollectionMembership {
    pub collection_id: i64,
    pub wallpaper_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SavedFilter {
    pub id: i64,
    pub name: String,
    pub filter_type: String,
    pub payload: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct WallpaperHistoryEntry {
    pub id: i64,
    pub wallpaper_id: Option<i64>,
    pub title: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub media_type: String,
    pub target_monitor_id: Option<String>,
    pub apply_source: String,
    pub applied_at: i64,
}

#[derive(Debug, Clone)]
pub struct WallpaperHistoryRecord {
    pub wallpaper_id: Option<i64>,
    pub title: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub media_type: String,
    pub target_monitor_id: Option<String>,
    pub apply_source: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: impl Into<PathBuf>) -> Result<Self> {
        let path = db_path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| rusqlite::Error::InvalidPath(parent.to_path_buf()))?;
        }

        let conn = Connection::open(path)?;

        let db = Self { conn };
        db.initialize()?;

        Ok(db)
    }

    fn initialize(&self) -> Result<()> {
        schema::initialize(&self.conn)
    }

    pub fn add_wallpaper(&self, wallpaper: &WallpaperMetadata, file_path: &str) -> Result<i64> {
        wallpapers::add(&self.conn, wallpaper, file_path, current_unix_timestamp())
    }

    pub fn get_wallpaper(&self, id: i64) -> Result<Option<Wallpaper>> {
        wallpapers::get(&self.conn, id)
    }

    pub fn list_wallpapers(&self) -> Result<Vec<Wallpaper>> {
        wallpapers::list(&self.conn)
    }

    pub fn list_favorite_wallpapers(&self) -> Result<Vec<Wallpaper>> {
        wallpapers::list_favorites(&self.conn)
    }

    pub fn get_wallpaper_by_file_path(&self, file_path: &str) -> Result<Option<Wallpaper>> {
        wallpapers::get_by_file_path(&self.conn, file_path)
    }

    pub fn search_wallpapers(&self, query: &str) -> Result<Vec<Wallpaper>> {
        wallpapers::search(&self.conn, query)
    }

    pub fn delete_wallpaper(&self, id: i64) -> Result<()> {
        wallpapers::delete(&self.conn, id)
    }

    pub fn set_wallpaper_favorite(&self, id: i64, is_favorite: bool) -> Result<Option<Wallpaper>> {
        wallpapers::set_favorite(&self.conn, id, is_favorite)
    }

    pub fn set_wallpaper_thumbnail(
        &self,
        id: i64,
        thumbnail_path: Option<&str>,
    ) -> Result<Option<Wallpaper>> {
        wallpapers::set_thumbnail(&self.conn, id, thumbnail_path)
    }

    pub fn list_collections(&self) -> Result<Vec<Collection>> {
        collections::list(&self.conn)
    }

    pub fn get_collection(&self, id: i64) -> Result<Option<Collection>> {
        collections::get(&self.conn, id)
    }

    pub fn create_collection(&self, name: &str) -> Result<Collection> {
        collections::create(&self.conn, name, current_unix_timestamp())
    }

    pub fn delete_collection(&self, id: i64) -> Result<bool> {
        collections::delete(&self.conn, id)
    }

    pub fn set_collection_membership(
        &self,
        collection_id: i64,
        wallpaper_id: i64,
        in_collection: bool,
    ) -> Result<Option<Collection>> {
        collections::set_membership(
            &self.conn,
            collection_id,
            wallpaper_id,
            in_collection,
            current_unix_timestamp(),
        )
    }

    pub fn list_collection_memberships(&self) -> Result<Vec<CollectionMembership>> {
        collections::list_memberships(&self.conn)
    }

    pub fn list_saved_filters(&self, filter_type: Option<&str>) -> Result<Vec<SavedFilter>> {
        saved_filters::list(&self.conn, filter_type)
    }

    pub fn save_filter(&self, name: &str, filter_type: &str, payload: &str) -> Result<SavedFilter> {
        saved_filters::save(
            &self.conn,
            name,
            filter_type,
            payload,
            current_unix_timestamp(),
        )
    }

    pub fn delete_saved_filter(&self, id: i64) -> Result<bool> {
        saved_filters::delete(&self.conn, id)
    }

    pub fn add_wallpaper_history(&self, record: &WallpaperHistoryRecord) -> Result<i64> {
        wallpaper_history::add(&self.conn, record, current_unix_timestamp())
    }

    pub fn list_wallpaper_history(&self, limit: i64) -> Result<Vec<WallpaperHistoryEntry>> {
        wallpaper_history::list(&self.conn, limit)
    }

    pub fn latest_distinct_wallpaper_history(
        &self,
        current_path: Option<&str>,
    ) -> Result<Option<WallpaperHistoryEntry>> {
        wallpaper_history::latest_distinct_before(&self.conn, current_path)
    }

    pub fn wallpaper_history_record(
        &self,
        file_path: &str,
        target_monitor_id: Option<&str>,
        apply_source: &str,
    ) -> Result<WallpaperHistoryRecord> {
        let wallpaper = self.get_wallpaper_by_file_path(file_path)?;
        Ok(wallpaper_history::record_from_wallpaper(
            wallpaper,
            file_path,
            target_monitor_id,
            apply_source,
        ))
    }

    pub fn get_wallpaper_by_source(
        &self,
        source: &str,
        source_id: &str,
    ) -> Result<Option<Wallpaper>> {
        wallpapers::get_by_source(&self.conn, source, source_id)
    }

    pub fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        preferences::set(&self.conn, key, value)
    }

    pub fn get_preference(&self, key: &str) -> Result<Option<String>> {
        preferences::get(&self.conn, key)
    }

    pub fn delete_preference(&self, key: &str) -> Result<()> {
        preferences::delete(&self.conn, key)
    }
}

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_db_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-{}-{}-{}.db",
            name,
            std::process::id(),
            stamp
        ))
    }

    fn sample_metadata(
        title: &str,
        thumbnail_path: Option<&str>,
        tags: Vec<&str>,
    ) -> WallpaperMetadata {
        WallpaperMetadata {
            title: title.to_string(),
            thumbnail_path: thumbnail_path.map(str::to_string),
            tags: tags.into_iter().map(str::to_string).collect(),
            width: 1920,
            height: 1080,
            fps: 60,
            duration_ms: 12_000,
            media_type: "video".to_string(),
            source: None,
            source_id: None,
        }
    }

    #[test]
    fn add_wallpaper_upserts_by_file_path_and_keeps_existing_thumbnail() -> Result<()> {
        let db_path = unique_db_path("upsert");
        let media_path = db_path.with_extension("mp4");
        fs::write(&media_path, b"test-video").expect("test media file should be writable");

        let db = Database::new(&db_path)?;

        let first = sample_metadata("First Title", Some("thumb-1.jpg"), vec!["calm", "night"]);
        let first_id = db.add_wallpaper(&first, media_path.to_str().unwrap())?;
        assert!(first_id > 0);

        let second = sample_metadata("Second Title", None, vec!["updated"]);
        let second_id = db.add_wallpaper(&second, media_path.to_str().unwrap())?;
        assert_eq!(first_id, second_id);

        let saved = db.get_wallpaper(first_id)?.expect("wallpaper should exist");
        assert_eq!(saved.title, "First Title");
        assert_eq!(saved.thumbnail_path.as_deref(), Some("thumb-1.jpg"));
        assert_eq!(saved.tags, vec!["updated"]);
        assert_eq!(saved.width, 1920);
        assert_eq!(saved.height, 1080);
        assert_eq!(saved.fps, 60);
        assert_eq!(saved.media_type, "video");
        assert_eq!(saved.source, None);
        assert_eq!(saved.source_id, None);
        assert!(!saved.is_favorite);

        drop(db);
        let _ = fs::remove_file(&media_path);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }

    #[test]
    fn search_and_preferences_round_trip() -> Result<()> {
        let db_path = unique_db_path("search");
        let db = Database::new(&db_path)?;

        db.set_preference("launchAtStartup", "true")?;
        assert_eq!(db.get_preference("launchAtStartup")?, Some("true".into()));

        let nature = sample_metadata("Forest Mist", None, vec!["nature", "green"]);
        let city = sample_metadata("City Lights", None, vec!["urban", "night"]);
        db.add_wallpaper(&nature, "/tmp/forest.mp4")?;
        db.add_wallpaper(&city, "/tmp/city.mp4")?;

        let title_matches = db.search_wallpapers("City")?;
        assert_eq!(title_matches.len(), 1);
        assert_eq!(title_matches[0].title, "City Lights");

        let tag_matches = db.search_wallpapers("nature")?;
        assert_eq!(tag_matches.len(), 1);
        assert_eq!(tag_matches[0].title, "Forest Mist");

        drop(db);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }

    #[test]
    fn favorite_status_round_trips() -> Result<()> {
        let db_path = unique_db_path("favorites");
        let db = Database::new(&db_path)?;

        let metadata = sample_metadata("Favorite Candidate", None, vec!["ambient"]);
        let wallpaper_id = db.add_wallpaper(&metadata, "/tmp/favorite-candidate.mp4")?;

        let updated = db
            .set_wallpaper_favorite(wallpaper_id, true)?
            .expect("wallpaper should be updated");
        assert!(updated.is_favorite);

        let listed = db.list_wallpapers()?;
        assert_eq!(listed.len(), 1);
        assert!(listed[0].is_favorite);

        let favorites = db.list_favorite_wallpapers()?;
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].id, wallpaper_id);

        let updated = db
            .set_wallpaper_favorite(wallpaper_id, false)?
            .expect("wallpaper should be updated");
        assert!(!updated.is_favorite);
        assert!(db.list_favorite_wallpapers()?.is_empty());

        assert!(db.set_wallpaper_favorite(-999, true)?.is_none());

        drop(db);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }

    #[test]
    fn collections_and_memberships_round_trip() -> Result<()> {
        let db_path = unique_db_path("collections");
        let db = Database::new(&db_path)?;

        let forest = sample_metadata("Forest Loop", None, vec!["nature"]);
        let city = sample_metadata("City Loop", None, vec!["urban"]);
        let forest_id = db.add_wallpaper(&forest, "/tmp/forest-loop.mp4")?;
        let city_id = db.add_wallpaper(&city, "/tmp/city-loop.mp4")?;

        let collection = db.create_collection("Nature")?;
        assert_eq!(collection.name, "Nature");
        assert_eq!(collection.wallpaper_count, 0);

        let updated = db
            .set_collection_membership(collection.id, forest_id, true)?
            .expect("collection should be updated");
        assert_eq!(updated.wallpaper_count, 1);

        let updated = db
            .set_collection_membership(collection.id, city_id, true)?
            .expect("collection should be updated");
        assert_eq!(updated.wallpaper_count, 2);

        let memberships = db.list_collection_memberships()?;
        assert_eq!(memberships.len(), 2);
        assert!(memberships
            .iter()
            .any(|item| item.collection_id == collection.id && item.wallpaper_id == forest_id));
        assert!(memberships
            .iter()
            .any(|item| item.collection_id == collection.id && item.wallpaper_id == city_id));

        let updated = db
            .set_collection_membership(collection.id, forest_id, false)?
            .expect("collection should be updated");
        assert_eq!(updated.wallpaper_count, 1);

        assert!(db.delete_collection(collection.id)?);
        assert!(db.list_collections()?.is_empty());
        assert!(db.list_collection_memberships()?.is_empty());

        drop(db);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }

    #[test]
    fn saved_filters_upsert_and_delete() -> Result<()> {
        let db_path = unique_db_path("saved-filters");
        let db = Database::new(&db_path)?;

        let first = db.save_filter("Ultrawide", "wallhaven", r#"{"ratios":"21x9"}"#)?;
        assert_eq!(first.name, "Ultrawide");
        assert_eq!(first.filter_type, "wallhaven");
        assert_eq!(first.payload, r#"{"ratios":"21x9"}"#);

        let updated = db.save_filter(
            "ultrawide",
            "wallhaven",
            r#"{"ratios":"21x9","sorting":"favorites"}"#,
        )?;
        assert_eq!(updated.id, first.id);
        assert_eq!(updated.name, "ultrawide");
        assert_eq!(
            updated.payload,
            r#"{"ratios":"21x9","sorting":"favorites"}"#
        );

        let local = db.save_filter("Calm", "local", r#"{"query":"calm"}"#)?;
        let wallhaven_filters = db.list_saved_filters(Some("wallhaven"))?;
        assert_eq!(wallhaven_filters.len(), 1);
        assert_eq!(wallhaven_filters[0].id, first.id);

        let all_filters = db.list_saved_filters(None)?;
        assert_eq!(all_filters.len(), 2);

        assert!(db.delete_saved_filter(local.id)?);
        assert_eq!(db.list_saved_filters(None)?.len(), 1);

        drop(db);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }

    #[test]
    fn wallpaper_history_records_and_finds_previous_distinct_entry() -> Result<()> {
        let db_path = unique_db_path("history");
        let db = Database::new(&db_path)?;

        let first = sample_metadata("Forest Loop", Some("forest-thumb.jpg"), vec!["nature"]);
        let second = sample_metadata("City Loop", None, vec!["urban"]);
        db.add_wallpaper(&first, "/tmp/forest-loop.mp4")?;
        db.add_wallpaper(&second, "/tmp/city-loop.mp4")?;

        let first_record = db.wallpaper_history_record("/tmp/forest-loop.mp4", None, "manual")?;
        db.add_wallpaper_history(&first_record)?;
        let second_record =
            db.wallpaper_history_record("/tmp/city-loop.mp4", Some("monitor-1"), "rotation")?;
        db.add_wallpaper_history(&second_record)?;

        let entries = db.list_wallpaper_history(10)?;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "City Loop");
        assert_eq!(entries[0].target_monitor_id.as_deref(), Some("monitor-1"));
        assert_eq!(entries[0].apply_source, "rotation");

        let previous = db
            .latest_distinct_wallpaper_history(Some("/tmp/city-loop.mp4"))?
            .expect("previous history entry should exist");
        assert_eq!(previous.file_path, "/tmp/forest-loop.mp4");
        assert_eq!(previous.thumbnail_path.as_deref(), Some("forest-thumb.jpg"));

        drop(db);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }

    #[test]
    fn wallpaper_history_skips_consecutive_duplicates_for_same_path_and_target() -> Result<()> {
        let db_path = unique_db_path("history-dedup");
        let db = Database::new(&db_path)?;

        let first_record = db.wallpaper_history_record("/tmp/forest-loop.mp4", None, "manual")?;
        db.add_wallpaper_history(&first_record)?;

        for _ in 0..60 {
            let duplicate_record =
                db.wallpaper_history_record("/tmp/city-loop.mp4", Some("monitor-1"), "rotation")?;
            db.add_wallpaper_history(&duplicate_record)?;
        }

        let entries = db.list_wallpaper_history(100)?;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file_path, "/tmp/city-loop.mp4");
        assert_eq!(entries[0].target_monitor_id.as_deref(), Some("monitor-1"));

        let previous = db
            .latest_distinct_wallpaper_history(Some("/tmp/city-loop.mp4"))?
            .expect("previous history entry should remain visible");
        assert_eq!(previous.file_path, "/tmp/forest-loop.mp4");

        let other_target_record =
            db.wallpaper_history_record("/tmp/city-loop.mp4", Some("monitor-2"), "manual")?;
        db.add_wallpaper_history(&other_target_record)?;
        assert_eq!(db.list_wallpaper_history(100)?.len(), 3);

        drop(db);
        let _ = fs::remove_file(&db_path);
        Ok(())
    }
}
