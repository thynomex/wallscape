use crate::db::Database;
use crate::wallpaper::WallpaperTarget;
use crate::AppDatabase;
use std::collections::BTreeMap;

const MONITOR_WALLPAPER_ASSIGNMENTS_KEY: &str = "monitor_wallpaper_assignments";

pub(super) fn load(db: &AppDatabase) -> Result<BTreeMap<String, String>, String> {
    let Some(json) =
        db.0.lock()
            .get_preference(MONITOR_WALLPAPER_ASSIGNMENTS_KEY)
            .map_err(|e| format!("Failed to read monitor wallpaper assignments: {}", e))?
    else {
        return Ok(BTreeMap::new());
    };

    parse(&json)
}

pub(super) fn load_from_database(db: &Database) -> Result<BTreeMap<String, String>, String> {
    let Some(json) = db
        .get_preference(MONITOR_WALLPAPER_ASSIGNMENTS_KEY)
        .map_err(|e| format!("Failed to read monitor wallpaper assignments: {}", e))?
    else {
        return Ok(BTreeMap::new());
    };

    parse(&json)
}

pub(super) fn persist_target_with_retained_assignments(
    db: &AppDatabase,
    target: &WallpaperTarget,
    wallpaper_path: &str,
    retained_assignments: &BTreeMap<String, String>,
) {
    let current_assignments = load(db).unwrap_or_else(|error| {
        tracing::warn!("{}", error);
        BTreeMap::new()
    });

    let Some(assignments) = assignments_for_target(
        target,
        wallpaper_path,
        current_assignments,
        retained_assignments,
    ) else {
        clear(db);
        return;
    };

    persist_assignments(db, &assignments);
}

fn assignments_for_target(
    target: &WallpaperTarget,
    wallpaper_path: &str,
    current_assignments: BTreeMap<String, String>,
    retained_assignments: &BTreeMap<String, String>,
) -> Option<BTreeMap<String, String>> {
    match target {
        WallpaperTarget::All => None,
        WallpaperTarget::Monitor(monitor_id) => {
            let mut assignments = current_assignments;
            if assignments.is_empty() {
                assignments.extend(retained_assignments.clone());
            }
            assignments.insert(monitor_id.clone(), wallpaper_path.to_string());
            Some(assignments)
        }
    }
}

fn persist_assignments(db: &AppDatabase, assignments: &BTreeMap<String, String>) {
    let Ok(json) = serde_json::to_string(assignments) else {
        tracing::warn!("Failed to serialize monitor wallpaper assignments");
        return;
    };

    if let Err(error) =
        db.0.lock()
            .set_preference(MONITOR_WALLPAPER_ASSIGNMENTS_KEY, &json)
    {
        tracing::warn!("Failed to persist monitor wallpaper assignment: {}", error);
    }
}

pub(super) fn clear(db: &AppDatabase) {
    if let Err(error) =
        db.0.lock()
            .delete_preference(MONITOR_WALLPAPER_ASSIGNMENTS_KEY)
    {
        tracing::warn!("Failed to clear monitor wallpaper assignments: {}", error);
    }
}

fn parse(json: &str) -> Result<BTreeMap<String, String>, String> {
    serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse monitor wallpaper assignments: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_monitor_assignment_map() {
        let assignments = parse(r#"{"DISPLAY1":"C:\\Wallscape\\loop.mp4"}"#)
            .expect("assignment JSON should parse");

        assert_eq!(
            assignments.get("DISPLAY1").map(String::as_str),
            Some("C:\\Wallscape\\loop.mp4")
        );
    }

    #[test]
    fn parse_rejects_invalid_assignment_json() {
        let error = parse("not-json").expect_err("invalid JSON should fail");

        assert!(error.contains("Failed to parse monitor wallpaper assignments"));
    }

    #[test]
    fn monitor_assignment_keeps_retained_all_display_slots() {
        let retained = BTreeMap::from([
            (
                "DISPLAY1".to_string(),
                "C:\\Wallscape\\ambient.mp4".to_string(),
            ),
            (
                "DISPLAY2".to_string(),
                "C:\\Wallscape\\ambient.mp4".to_string(),
            ),
        ]);

        let assignments = assignments_for_target(
            &WallpaperTarget::Monitor("DISPLAY1".to_string()),
            "C:\\Wallscape\\focus.mp4",
            BTreeMap::new(),
            &retained,
        )
        .expect("monitor target should persist assignments");

        assert_eq!(
            assignments.get("DISPLAY1").map(String::as_str),
            Some("C:\\Wallscape\\focus.mp4")
        );
        assert_eq!(
            assignments.get("DISPLAY2").map(String::as_str),
            Some("C:\\Wallscape\\ambient.mp4")
        );
    }

    #[test]
    fn monitor_assignment_prefers_existing_assignments_over_retained_defaults() {
        let current = BTreeMap::from([(
            "DISPLAY2".to_string(),
            "C:\\Wallscape\\existing.mp4".to_string(),
        )]);
        let retained = BTreeMap::from([(
            "DISPLAY2".to_string(),
            "C:\\Wallscape\\ambient.mp4".to_string(),
        )]);

        let assignments = assignments_for_target(
            &WallpaperTarget::Monitor("DISPLAY1".to_string()),
            "C:\\Wallscape\\focus.mp4",
            current,
            &retained,
        )
        .expect("monitor target should persist assignments");

        assert_eq!(
            assignments.get("DISPLAY1").map(String::as_str),
            Some("C:\\Wallscape\\focus.mp4")
        );
        assert_eq!(
            assignments.get("DISPLAY2").map(String::as_str),
            Some("C:\\Wallscape\\existing.mp4")
        );
    }

    #[test]
    fn all_target_clears_assignments() {
        let current = BTreeMap::from([(
            "DISPLAY1".to_string(),
            "C:\\Wallscape\\focus.mp4".to_string(),
        )]);

        assert!(assignments_for_target(
            &WallpaperTarget::All,
            "C:\\Wallscape\\ambient.mp4",
            current,
            &BTreeMap::new()
        )
        .is_none());
    }
}
