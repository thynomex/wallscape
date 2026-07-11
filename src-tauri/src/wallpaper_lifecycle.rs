mod assignments;

use crate::db::Database;
use crate::settings::{Settings, LAST_WALLPAPER_KEY, PREVIOUS_WALLPAPER_KEY};
use crate::wallpaper::{
    get_desktop_wallpaper, set_desktop_wallpaper, set_desktop_wallpaper_for_monitor, FitMode,
    MonitorManager, PlaybackOptions, WallpaperRuntime, WallpaperTarget,
};
use crate::wallpaper_media::{
    is_supported_static_image_path, is_supported_wallpaper_path, should_apply_as_static_wallpaper,
};
use crate::AppDatabase;
use std::collections::BTreeMap;
use std::path::Path;

pub const ORIGINAL_WALLPAPER_BACKUP_KEY: &str = "original_wallpaper_path";

pub async fn apply_wallpaper_path(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
    wallpaper_path: String,
    monitor_id: Option<String>,
) -> Result<String, String> {
    apply_wallpaper_path_with_source(db, runtime, wallpaper_path, monitor_id, "manual").await
}

pub async fn apply_wallpaper_path_with_source(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
    wallpaper_path: String,
    monitor_id: Option<String>,
    apply_source: &str,
) -> Result<String, String> {
    let previous_path = {
        let db_guard = db.0.lock();
        backup_original_wallpaper_if_absent(&db_guard)?;
        previous_wallpaper_path(&db_guard, &wallpaper_path)?
    };

    let target_monitor_id = monitor_id.filter(|id| !id.trim().is_empty());
    let target = target_from_monitor_id(target_monitor_id.clone());
    let result =
        apply_without_previous_tracking(db, runtime, wallpaper_path.clone(), target.clone())
            .await?;

    if let Some(path) = previous_path {
        persist_previous_wallpaper(db, &path);
    }

    record_wallpaper_history(
        db,
        &wallpaper_path,
        target_monitor_id.as_deref(),
        apply_source,
    );

    Ok(result)
}

pub async fn restore_wallpaper_path(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
    wallpaper_path: String,
    restored_label: &str,
) -> Result<String, String> {
    apply_without_previous_tracking(db, runtime, wallpaper_path.clone(), WallpaperTarget::All)
        .await?;
    record_wallpaper_history(
        db,
        &wallpaper_path,
        None,
        &format!("restore_{restored_label}"),
    );
    Ok(format!(
        "Restored {restored_label} wallpaper: {}",
        wallpaper_path
    ))
}

pub async fn restore_original_wallpaper(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<String, String> {
    let backup_path = original_wallpaper_backup_path(db)?;
    restore_static_wallpaper(runtime, backup_path.clone(), "original").await?;
    assignments::clear(db);
    clear_last_wallpaper(db);
    record_wallpaper_history(db, &backup_path, None, "restore_original");
    Ok(format!("Restored original wallpaper: {}", backup_path))
}

pub fn restore_original_wallpaper_blocking(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<String, String> {
    let backup_path = original_wallpaper_backup_path(db)?;
    runtime
        .stop()
        .map_err(|e| format!("Failed to stop video wallpaper: {}", e))?;
    set_desktop_wallpaper(&backup_path)
        .map_err(|e| format!("Failed to restore original wallpaper: {}", e))?;
    assignments::clear(db);
    clear_last_wallpaper(db);
    record_wallpaper_history(db, &backup_path, None, "restore_original");

    Ok(format!("Restored original wallpaper: {}", backup_path))
}

pub fn restore_last_wallpaper_on_launch(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<(), String> {
    if restore_monitor_assignments_on_launch(db, runtime)? {
        return Ok(());
    }

    let Some(wallpaper_path) = last_wallpaper_path(db)? else {
        return Ok(());
    };

    if wallpaper_path.trim().is_empty() || !Path::new(&wallpaper_path).exists() {
        return Err(format!(
            "Last wallpaper is no longer available: {}",
            wallpaper_path
        ));
    }

    if !is_supported_wallpaper_path(Path::new(&wallpaper_path)) {
        return Err(format!(
            "Last wallpaper uses an unsupported format: {}",
            wallpaper_path
        ));
    }

    if is_static_wallpaper(db, &wallpaper_path)? {
        runtime
            .stop()
            .map_err(|e| format!("Failed to stop active video wallpaper: {}", e))?;
        set_desktop_wallpaper(&wallpaper_path)
            .map_err(|e| format!("Failed to restore last wallpaper: {}", e))?;
    } else {
        runtime
            .set_video_wallpaper_for_target_with_options(
                wallpaper_path,
                WallpaperTarget::All,
                playback_options(db),
            )
            .map_err(|e| format!("Failed to restore last wallpaper: {}", e))?;
    }

    Ok(())
}

pub fn previous_wallpaper_path(db: &Database, next_path: &str) -> Result<Option<String>, String> {
    let current_path = db
        .get_preference(LAST_WALLPAPER_KEY)
        .map_err(|e| format!("Failed to read current wallpaper path: {}", e))?;

    Ok(previous_path_candidate(current_path, next_path))
}

pub fn backup_original_wallpaper_if_absent(db: &Database) -> Result<(), String> {
    let existing = db
        .get_preference(ORIGINAL_WALLPAPER_BACKUP_KEY)
        .map_err(|e| format!("Failed to read original wallpaper backup: {}", e))?;

    if has_original_wallpaper_backup(existing.as_deref()) {
        return Ok(());
    }

    let Some(path) = get_desktop_wallpaper()
        .map_err(|e| format!("Failed to save original wallpaper backup: {}", e))?
    else {
        tracing::warn!("Windows did not report a static wallpaper path to back up");
        return Ok(());
    };

    if is_wallscape_tracked_wallpaper(db, &path)? {
        tracing::warn!(
            "Refusing to save Wallscape-tracked wallpaper as original: {}",
            path
        );
        return Ok(());
    }

    db.set_preference(ORIGINAL_WALLPAPER_BACKUP_KEY, &path)
        .map_err(|e| format!("Failed to save original wallpaper backup: {}", e))?;
    tracing::info!("Saved original Windows wallpaper backup: {}", path);

    Ok(())
}

fn last_wallpaper_path(db: &AppDatabase) -> Result<Option<String>, String> {
    db.0.lock()
        .get_preference(LAST_WALLPAPER_KEY)
        .map_err(|e| format!("Failed to read last wallpaper path: {}", e))
}

pub fn monitor_wallpaper_assignments(db: &AppDatabase) -> Result<BTreeMap<String, String>, String> {
    assignments::load(db)
}

pub fn persist_last_wallpaper(db: &AppDatabase, path: &str) {
    if let Err(error) = db.0.lock().set_preference(LAST_WALLPAPER_KEY, path) {
        tracing::warn!("Failed to persist last wallpaper path: {}", error);
    }
}

pub fn persist_previous_wallpaper(db: &AppDatabase, path: &str) {
    if let Err(error) = db.0.lock().set_preference(PREVIOUS_WALLPAPER_KEY, path) {
        tracing::warn!("Failed to persist previous wallpaper path: {}", error);
    }
}

fn clear_last_wallpaper(db: &AppDatabase) {
    if let Err(error) = db.0.lock().delete_preference(LAST_WALLPAPER_KEY) {
        tracing::warn!("Failed to clear last wallpaper path: {}", error);
    }
}

fn previous_path_candidate(current_path: Option<String>, next_path: &str) -> Option<String> {
    current_path.filter(|path| !path.trim().is_empty() && path != next_path)
}

async fn apply_without_previous_tracking(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
    wallpaper_path: String,
    target: WallpaperTarget,
) -> Result<String, String> {
    if !is_supported_wallpaper_path(Path::new(&wallpaper_path)) {
        return Err(format!("Unsupported wallpaper format: {}", wallpaper_path));
    }

    let is_image = is_static_wallpaper(db, &wallpaper_path)?;
    let retained_assignments = retained_all_display_video_assignments(db, runtime, &target);

    if is_image {
        match &target {
            WallpaperTarget::All => {
                restore_static_wallpaper(runtime, wallpaper_path.clone(), "static").await?;
                assignments::clear(db);
            }
            WallpaperTarget::Monitor(monitor_id) => {
                set_static_wallpaper_for_monitor(
                    runtime,
                    wallpaper_path.clone(),
                    monitor_id.clone(),
                )
                .await?;
                assignments::persist_target_with_retained_assignments(
                    db,
                    &target,
                    &wallpaper_path,
                    &retained_assignments,
                );
            }
        }
        persist_last_wallpaper(db, &wallpaper_path);
        Ok(format!("Wallpaper set to: {}", wallpaper_path))
    } else {
        let options = playback_options(db);
        let result =
            set_video_wallpaper(runtime, wallpaper_path.clone(), target.clone(), options).await?;
        persist_last_wallpaper(db, &wallpaper_path);
        assignments::persist_target_with_retained_assignments(
            db,
            &target,
            &wallpaper_path,
            &retained_assignments,
        );
        Ok(result)
    }
}

fn retained_all_display_video_assignments(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
    target: &WallpaperTarget,
) -> BTreeMap<String, String> {
    if !matches!(target, WallpaperTarget::Monitor(_)) || !runtime.is_active() {
        return BTreeMap::new();
    }

    match assignments::load(db) {
        Ok(assignments) if assignments.is_empty() => {}
        Ok(_) => return BTreeMap::new(),
        Err(error) => {
            tracing::warn!("{}", error);
            return BTreeMap::new();
        }
    }

    let Some(retained_path) = active_all_display_video_path(db) else {
        return BTreeMap::new();
    };

    let monitor_ids = match current_monitor_ids() {
        Ok(ids) => ids,
        Err(error) => {
            tracing::warn!("{}", error);
            return BTreeMap::new();
        }
    };

    retained_monitor_assignments(&retained_path, monitor_ids)
}

fn active_all_display_video_path(db: &AppDatabase) -> Option<String> {
    let path = match last_wallpaper_path(db) {
        Ok(Some(path)) => path,
        Ok(None) => return None,
        Err(error) => {
            tracing::warn!("{}", error);
            return None;
        }
    };

    if path.trim().is_empty() || !is_supported_wallpaper_path(Path::new(&path)) {
        return None;
    }

    match is_static_wallpaper(db, &path) {
        Ok(true) => None,
        Ok(false) => Some(path),
        Err(error) => {
            tracing::warn!("{}", error);
            None
        }
    }
}

fn current_monitor_ids() -> Result<Vec<String>, String> {
    let manager =
        MonitorManager::new().map_err(|e| format!("Failed to enumerate monitors: {}", e))?;
    Ok(manager
        .get_monitors()
        .iter()
        .map(|monitor| monitor.id.clone())
        .collect())
}

fn retained_monitor_assignments(
    wallpaper_path: &str,
    monitor_ids: Vec<String>,
) -> BTreeMap<String, String> {
    monitor_ids
        .into_iter()
        .map(|monitor_id| (monitor_id, wallpaper_path.to_string()))
        .collect()
}

fn is_static_wallpaper(db: &AppDatabase, wallpaper_path: &str) -> Result<bool, String> {
    Ok(db
        .0
        .lock()
        .get_wallpaper_by_file_path(wallpaper_path)
        .map_err(|e| format!("Failed to read wallpaper metadata: {}", e))?
        .map(|wallpaper| should_apply_as_static_wallpaper(&wallpaper.media_type, wallpaper_path))
        .unwrap_or_else(|| is_supported_static_image_path(Path::new(wallpaper_path))))
}

async fn restore_static_wallpaper(
    runtime: &WallpaperRuntime,
    image_path: String,
    action_label: &str,
) -> Result<(), String> {
    let runtime = runtime.clone();
    let action_label = action_label.to_string();
    tokio::task::spawn_blocking(move || {
        runtime
            .stop()
            .map_err(|e| format!("Failed to stop active video wallpaper: {}", e))?;
        set_desktop_wallpaper(&image_path)
            .map_err(|e| format!("Failed to {action_label} wallpaper: {}", e))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

async fn set_static_wallpaper_for_monitor(
    runtime: &WallpaperRuntime,
    image_path: String,
    monitor_id: String,
) -> Result<(), String> {
    let runtime = runtime.clone();
    tokio::task::spawn_blocking(move || {
        runtime
            .stop_target(WallpaperTarget::Monitor(monitor_id.clone()))
            .map_err(|e| format!("Failed to stop active video wallpaper for monitor: {}", e))?;
        set_desktop_wallpaper_for_monitor(&image_path, &monitor_id)
            .map_err(|e| format!("Failed to set monitor wallpaper: {}", e))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

async fn set_video_wallpaper(
    runtime: &WallpaperRuntime,
    video_path: String,
    target: WallpaperTarget,
    options: PlaybackOptions,
) -> Result<String, String> {
    let runtime = runtime.clone();
    tokio::task::spawn_blocking(move || {
        runtime.set_video_wallpaper_for_target_with_options(video_path, target, options)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Failed to set wallpaper: {}", e))
}

fn restore_monitor_assignments_on_launch(
    db: &AppDatabase,
    runtime: &WallpaperRuntime,
) -> Result<bool, String> {
    let assignments = monitor_wallpaper_assignments(db)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    let mut restored_any = false;

    for (monitor_id, wallpaper_path) in assignments {
        if wallpaper_path.trim().is_empty() || !Path::new(&wallpaper_path).exists() {
            tracing::warn!(
                "Skipping monitor wallpaper assignment for '{}'; file is missing: {}",
                monitor_id,
                wallpaper_path
            );
            continue;
        }

        if !is_supported_wallpaper_path(Path::new(&wallpaper_path)) {
            tracing::warn!(
                "Skipping monitor wallpaper assignment for '{}'; unsupported format: {}",
                monitor_id,
                wallpaper_path
            );
            continue;
        }

        if is_static_wallpaper(db, &wallpaper_path)? {
            runtime
                .stop_target(WallpaperTarget::Monitor(monitor_id.clone()))
                .map_err(|e| {
                    format!(
                        "Failed to stop active video wallpaper for monitor '{}': {}",
                        monitor_id, e
                    )
                })?;
            set_desktop_wallpaper_for_monitor(&wallpaper_path, &monitor_id).map_err(|e| {
                format!(
                    "Failed to restore static wallpaper assignment for monitor '{}': {}",
                    monitor_id, e
                )
            })?;
            restored_any = true;
            continue;
        }

        runtime
            .set_video_wallpaper_for_target_with_options(
                wallpaper_path.clone(),
                WallpaperTarget::Monitor(monitor_id.clone()),
                playback_options(db),
            )
            .map_err(|e| {
                format!(
                    "Failed to restore wallpaper assignment for monitor '{}': {}",
                    monitor_id, e
                )
            })?;
        restored_any = true;
    }

    Ok(restored_any)
}

fn playback_options(db: &AppDatabase) -> PlaybackOptions {
    let settings = Settings::load(&db.0.lock());
    PlaybackOptions {
        speed: settings.default_playback_speed,
        fit_mode: fit_mode_from_setting(&settings.default_fit_mode),
        fps_limit: settings.default_fps_cap,
    }
}

fn fit_mode_from_setting(mode: &str) -> FitMode {
    match mode {
        "fill" => FitMode::Fill,
        "stretch" => FitMode::Stretch,
        _ => FitMode::Fit,
    }
}

fn target_from_monitor_id(monitor_id: Option<String>) -> WallpaperTarget {
    monitor_id
        .map(WallpaperTarget::Monitor)
        .unwrap_or(WallpaperTarget::All)
}

pub fn record_wallpaper_history(
    db: &AppDatabase,
    wallpaper_path: &str,
    target_monitor_id: Option<&str>,
    apply_source: &str,
) {
    let db_guard = db.0.lock();
    match db_guard
        .wallpaper_history_record(wallpaper_path, target_monitor_id, apply_source)
        .and_then(|record| db_guard.add_wallpaper_history(&record).map(|_| ()))
    {
        Ok(()) => {}
        Err(error) => tracing::warn!("Failed to record wallpaper history: {}", error),
    }
}

fn original_wallpaper_backup_path(db: &AppDatabase) -> Result<String, String> {
    db.0.lock()
        .get_preference(ORIGINAL_WALLPAPER_BACKUP_KEY)
        .map_err(|e| format!("Failed to read original wallpaper backup: {}", e))?
        .ok_or_else(|| "No original wallpaper backup has been saved yet".to_string())
}

fn has_original_wallpaper_backup(path: Option<&str>) -> bool {
    path.map(|path| !path.trim().is_empty()).unwrap_or(false)
}

fn is_wallscape_tracked_wallpaper(db: &Database, path: &str) -> Result<bool, String> {
    if preference_path_matches(db, LAST_WALLPAPER_KEY, path, "last wallpaper path")? {
        return Ok(true);
    }

    if db
        .list_wallpapers()
        .map_err(|e| format!("Failed to read wallpaper library: {}", e))?
        .iter()
        .any(|wallpaper| same_wallpaper_path(&wallpaper.file_path, path))
    {
        return Ok(true);
    }

    Ok(assignments::load_from_database(db)?
        .values()
        .any(|assigned_path| same_wallpaper_path(assigned_path, path)))
}

fn preference_path_matches(
    db: &Database,
    key: &str,
    path: &str,
    label: &str,
) -> Result<bool, String> {
    Ok(db
        .get_preference(key)
        .map_err(|e| format!("Failed to read {}: {}", label, e))?
        .as_deref()
        .map(|saved_path| same_wallpaper_path(saved_path, path))
        .unwrap_or(false))
}

fn same_wallpaper_path(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();

    if cfg!(windows) {
        left.replace('/', "\\")
            .eq_ignore_ascii_case(&right.replace('/', "\\"))
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WallpaperMetadata;
    use crate::wallpaper_media::is_supported_image_path;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_db_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-lifecycle-{}-{}-{}.db",
            name,
            std::process::id(),
            stamp
        ))
    }

    fn sample_metadata(media_type: &str) -> WallpaperMetadata {
        WallpaperMetadata {
            title: "Applied Wallpaper".to_string(),
            thumbnail_path: None,
            tags: Vec::new(),
            width: 1920,
            height: 1080,
            fps: 0,
            duration_ms: 0,
            media_type: media_type.to_string(),
            source: None,
            source_id: None,
        }
    }

    #[test]
    fn image_path_detection_is_case_insensitive() {
        assert!(is_supported_image_path(Path::new(
            "C:/wallpapers/Forest.JPG"
        )));
    }

    #[test]
    fn gif_path_detection_is_unsupported() {
        let path = Path::new("C:/wallpapers/Loop.GIF");
        assert!(!is_supported_image_path(path));
        assert!(!is_supported_static_image_path(path));
        assert!(!is_supported_wallpaper_path(path));
    }

    #[test]
    fn image_path_detection_rejects_unknown_extensions() {
        assert!(!is_supported_image_path(Path::new(
            "C:/wallpapers/Forest.mp4"
        )));
    }

    #[test]
    fn previous_path_candidate_ignores_empty_or_identical_paths() {
        assert_eq!(previous_path_candidate(None, "next"), None);
        assert_eq!(previous_path_candidate(Some(" ".to_string()), "next"), None);
        assert_eq!(
            previous_path_candidate(Some("C:/same.jpg".to_string()), "C:/same.jpg"),
            None,
        );
    }

    #[test]
    fn previous_path_candidate_keeps_different_current_path() {
        assert_eq!(
            previous_path_candidate(Some("C:/old.jpg".to_string()), "C:/new.jpg"),
            Some("C:/old.jpg".to_string()),
        );
    }

    #[test]
    fn original_backup_exists_for_any_non_blank_saved_path() {
        assert!(has_original_wallpaper_backup(Some(
            "C:/missing-original.jpg"
        )));
        assert!(has_original_wallpaper_backup(Some(
            "C:/wallpapers/original.jpg"
        )));
        assert!(!has_original_wallpaper_backup(Some("")));
        assert!(!has_original_wallpaper_backup(Some("   ")));
        assert!(!has_original_wallpaper_backup(None));
    }

    #[test]
    fn same_wallpaper_path_treats_windows_separators_as_equivalent() {
        assert!(same_wallpaper_path(
            "C:/Users/me/Pictures/wallpaper.jpg",
            "C:\\Users\\me\\Pictures\\wallpaper.jpg"
        ));
    }

    #[test]
    fn wallscape_tracked_wallpaper_detects_last_wallpaper_path() {
        let db_path = unique_db_path("tracked-last");
        let db = Database::new(&db_path).expect("database should open");

        db.set_preference(LAST_WALLPAPER_KEY, "C:\\Wallscape\\applied.jpg")
            .expect("last wallpaper should save");

        assert!(
            is_wallscape_tracked_wallpaper(&db, "C:/Wallscape/applied.jpg")
                .expect("tracked path check should succeed")
        );

        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn wallscape_tracked_wallpaper_detects_library_wallpaper_path() {
        let db_path = unique_db_path("tracked-library");
        let db = Database::new(&db_path).expect("database should open");

        db.add_wallpaper(
            &sample_metadata("image"),
            "C:\\Wallscape\\library-wallpaper.jpg",
        )
        .expect("wallpaper should save");

        assert!(
            is_wallscape_tracked_wallpaper(&db, "C:/Wallscape/library-wallpaper.jpg")
                .expect("tracked path check should succeed")
        );

        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }
}
