use crate::db::WallpaperHistoryEntry;
use crate::wallpaper::WallpaperRuntime;
use crate::wallpaper_lifecycle;
use crate::wallpaper_media::is_supported_wallpaper_path;
use crate::AppDatabase;
use std::path::Path;

const DEFAULT_HISTORY_LIMIT: i64 = 12;
const MAX_HISTORY_LIMIT: i64 = 50;

pub(crate) fn list_wallpaper_history(
    db: &AppDatabase,
    limit: Option<i64>,
) -> Result<Vec<WallpaperHistoryEntry>, String> {
    let limit = limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .clamp(1, MAX_HISTORY_LIMIT);

    db.0.lock()
        .list_wallpaper_history(limit)
        .map_err(|e| format!("Failed to list wallpaper history: {}", e))
}

pub(crate) async fn undo_wallpaper_history(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<WallpaperHistoryEntry, String> {
    let target = {
        let db_guard = db.0.lock();
        let entries = db_guard
            .list_wallpaper_history(MAX_HISTORY_LIMIT)
            .map_err(|e| format!("Failed to list wallpaper history: {}", e))?;

        select_undo_target(&entries, is_existing_supported_wallpaper)
            .ok_or_else(|| "No previous wallpaper history entry is available".to_string())?
    };

    wallpaper_lifecycle::apply_wallpaper_path_with_source(
        db,
        runtime,
        target.entry.file_path.clone(),
        target.monitor_id.clone(),
        "history_undo",
    )
    .await?;

    Ok(target.entry)
}

#[derive(Debug, Clone)]
struct WallpaperHistoryUndoTarget {
    entry: WallpaperHistoryEntry,
    monitor_id: Option<String>,
}

fn select_undo_target(
    entries: &[WallpaperHistoryEntry],
    is_valid: impl Fn(&WallpaperHistoryEntry) -> bool,
) -> Option<WallpaperHistoryUndoTarget> {
    let latest = entries.first()?;
    let monitor_id = latest.target_monitor_id.clone();

    entries
        .iter()
        .skip(1)
        .find(|entry| {
            affects_undo_target(
                entry.target_monitor_id.as_deref(),
                latest.target_monitor_id.as_deref(),
            ) && !same_wallpaper_path(Some(&latest.file_path), &entry.file_path)
                && is_valid(entry)
        })
        .cloned()
        .map(|entry| WallpaperHistoryUndoTarget { entry, monitor_id })
}

fn affects_undo_target(entry_target: Option<&str>, undo_target: Option<&str>) -> bool {
    match undo_target {
        Some(monitor_id) => entry_target.is_none() || entry_target == Some(monitor_id),
        None => entry_target.is_none(),
    }
}

fn is_existing_supported_wallpaper(entry: &WallpaperHistoryEntry) -> bool {
    let path = Path::new(&entry.file_path);
    path.exists() && is_supported_wallpaper_path(path)
}

fn same_wallpaper_path(left: Option<&str>, right: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{same_wallpaper_path, select_undo_target};
    use crate::db::WallpaperHistoryEntry;

    fn history_entry(
        id: i64,
        file_path: &str,
        target_monitor_id: Option<&str>,
    ) -> WallpaperHistoryEntry {
        WallpaperHistoryEntry {
            id,
            wallpaper_id: Some(id),
            title: format!("Wallpaper {id}"),
            file_path: file_path.to_string(),
            thumbnail_path: None,
            media_type: "image".to_string(),
            target_monitor_id: target_monitor_id.map(str::to_string),
            apply_source: "manual".to_string(),
            applied_at: id,
        }
    }

    #[test]
    fn same_wallpaper_path_ignores_windows_separator_differences() {
        assert!(same_wallpaper_path(
            Some("C:/Wallpapers/one.jpg"),
            "C:\\Wallpapers\\one.jpg",
        ));
    }

    #[test]
    fn undo_monitor_apply_uses_prior_entry_that_affected_same_monitor() {
        let entries = vec![
            history_entry(3, "C:/wallpapers/city.jpg", Some("monitor-2")),
            history_entry(2, "C:/wallpapers/forest.jpg", Some("monitor-1")),
            history_entry(1, "C:/wallpapers/original.jpg", None),
        ];

        let target =
            select_undo_target(&entries, |_| true).expect("undo target should be selected");

        assert_eq!(target.entry.file_path, "C:/wallpapers/original.jpg");
        assert_eq!(target.monitor_id.as_deref(), Some("monitor-2"));
    }

    #[test]
    fn undo_monitor_apply_prefers_prior_same_monitor_entry() {
        let entries = vec![
            history_entry(3, "C:/wallpapers/city.jpg", Some("monitor-2")),
            history_entry(2, "C:/wallpapers/forest.jpg", Some("monitor-2")),
            history_entry(1, "C:/wallpapers/original.jpg", None),
        ];

        let target =
            select_undo_target(&entries, |_| true).expect("undo target should be selected");

        assert_eq!(target.entry.file_path, "C:/wallpapers/forest.jpg");
        assert_eq!(target.monitor_id.as_deref(), Some("monitor-2"));
    }

    #[test]
    fn undo_all_monitor_apply_ignores_monitor_specific_entries() {
        let entries = vec![
            history_entry(4, "C:/wallpapers/global-new.jpg", None),
            history_entry(3, "C:/wallpapers/monitor-two.jpg", Some("monitor-2")),
            history_entry(2, "C:/wallpapers/monitor-one.jpg", Some("monitor-1")),
            history_entry(1, "C:/wallpapers/global-old.jpg", None),
        ];

        let target =
            select_undo_target(&entries, |_| true).expect("undo target should be selected");

        assert_eq!(target.entry.file_path, "C:/wallpapers/global-old.jpg");
        assert_eq!(target.monitor_id, None);
    }
}
