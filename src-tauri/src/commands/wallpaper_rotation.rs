use crate::db::Wallpaper;
use crate::settings::LAST_WALLPAPER_KEY;
use crate::wallpaper::WallpaperRuntime;
use crate::wallpaper_lifecycle;
use crate::AppDatabase;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) async fn rotate_random_favorite_wallpaper(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<Wallpaper, String> {
    let wallpaper = next_favorite_wallpaper(db)?;
    wallpaper_lifecycle::apply_wallpaper_path_with_source(
        db,
        runtime,
        wallpaper.file_path.clone(),
        None,
        "rotation",
    )
    .await?;
    Ok(wallpaper)
}

fn next_favorite_wallpaper(db: &AppDatabase) -> Result<Wallpaper, String> {
    let db_guard = db.0.lock();
    let favorites = db_guard
        .list_favorite_wallpapers()
        .map_err(|e| format!("Failed to list favorite wallpapers: {}", e))?;

    if favorites.is_empty() {
        return Err("No favorite wallpapers are available for rotation".to_string());
    }

    let current_path = db_guard
        .get_preference(LAST_WALLPAPER_KEY)
        .map_err(|e| format!("Failed to read current wallpaper path: {}", e))?;

    Ok(pick_rotation_wallpaper(favorites, current_path.as_deref()))
}

fn pick_rotation_wallpaper(favorites: Vec<Wallpaper>, current_path: Option<&str>) -> Wallpaper {
    let mut candidates = favorites
        .iter()
        .filter(|wallpaper| Some(wallpaper.file_path.as_str()) != current_path)
        .cloned()
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        candidates = favorites;
    }

    let index = pseudo_random_index(candidates.len());
    candidates.swap_remove(index)
}

fn pseudo_random_index(len: usize) -> usize {
    if len <= 1 {
        return 0;
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    (nanos % len as u128) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wallpaper(id: i64, file_path: &str) -> Wallpaper {
        Wallpaper {
            id,
            title: format!("Wallpaper {id}"),
            file_path: file_path.to_string(),
            thumbnail_path: None,
            tags: Vec::new(),
            width: 1920,
            height: 1080,
            fps: 0,
            duration_ms: 0,
            file_size_bytes: 0,
            created_at: id,
            media_type: "image".to_string(),
            source: None,
            source_id: None,
            is_favorite: true,
        }
    }

    #[test]
    fn rotation_skips_current_wallpaper_when_another_favorite_exists() {
        let selected = pick_rotation_wallpaper(
            vec![wallpaper(1, "current.jpg"), wallpaper(2, "next.jpg")],
            Some("current.jpg"),
        );

        assert_eq!(selected.file_path, "next.jpg");
    }

    #[test]
    fn rotation_can_reuse_current_wallpaper_when_it_is_the_only_favorite() {
        let selected =
            pick_rotation_wallpaper(vec![wallpaper(1, "current.jpg")], Some("current.jpg"));

        assert_eq!(selected.file_path, "current.jpg");
    }
}
