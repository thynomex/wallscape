use crate::settings::{Settings, SettingsState};
use crate::wallpaper::WallpaperRuntime;
use crate::AppDatabase;
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    /// A video wallpaper is currently running.
    pub active: bool,
    /// The active wallpaper is paused.
    pub paused: bool,
    /// The active wallpaper is currently paused by an automatic rule.
    pub auto_paused: bool,
    /// Current speed, or null when inactive or different across displays.
    pub playback_speed: Option<f64>,
    /// Current fit mode, or null when inactive or different across displays.
    pub fit_mode: Option<String>,
}

impl From<crate::wallpaper::runtime::RuntimeSnapshot> for RuntimeStatus {
    fn from(snapshot: crate::wallpaper::runtime::RuntimeSnapshot) -> Self {
        use crate::wallpaper::engine::FitMode;
        Self {
            active: snapshot.active,
            paused: snapshot.paused,
            auto_paused: snapshot.auto_paused,
            playback_speed: snapshot.playback_speed,
            fit_mode: snapshot.fit_mode.map(|mode| match mode {
                FitMode::Fit => "fit".to_string(),
                FitMode::Fill => "fill".to_string(),
                FitMode::Stretch => "stretch".to_string(),
            }),
        }
    }
}

#[tauri::command]
pub async fn get_runtime_status(
    runtime: tauri::State<'_, WallpaperRuntime>,
) -> Result<RuntimeStatus, String> {
    let snapshot = runtime.snapshot();
    Ok(snapshot.into())
}

#[tauri::command]
pub async fn set_wallpaper_paused(
    runtime: tauri::State<'_, WallpaperRuntime>,
    paused: bool,
) -> Result<RuntimeStatus, String> {
    let runtime = runtime.inner().clone();

    if runtime.is_active() {
        let runtime_for_task = runtime.clone();
        tokio::task::spawn_blocking(move || runtime_for_task.set_paused(paused))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to set pause state: {}", e))?;
    }

    let snapshot = runtime.snapshot();
    Ok(snapshot.into())
}

#[tauri::command]
pub async fn set_wallpaper_speed(
    app_handle: tauri::AppHandle,
    runtime: tauri::State<'_, WallpaperRuntime>,
    speed: f64,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    if !speed.is_finite() {
        return Err("Playback speed must be a finite number".to_string());
    }
    update_playback_default(
        app_handle,
        runtime.inner().clone(),
        PlaybackSetting::Speed(speed),
        monitor_id,
    )
    .await
}

#[tauri::command]
pub async fn set_wallpaper_fit_mode(
    app_handle: tauri::AppHandle,
    runtime: tauri::State<'_, WallpaperRuntime>,
    mode: String,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    update_playback_default(
        app_handle,
        runtime.inner().clone(),
        PlaybackSetting::FitMode(mode),
        monitor_id,
    )
    .await
}

enum PlaybackSetting {
    Speed(f64),
    FitMode(String),
}

async fn update_playback_default(
    app_handle: tauri::AppHandle,
    runtime: WallpaperRuntime,
    setting: PlaybackSetting,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    tokio::task::spawn_blocking(move || {
        let _update = super::settings::settings_update_lock();
        let db = app_handle
            .try_state::<AppDatabase>()
            .ok_or_else(|| "Database state is not initialized".to_string())?;
        // Release the database lock before waiting for the wallpaper actor.
        let mut settings = Settings::load(&db.0.lock());
        match &setting {
            PlaybackSetting::Speed(speed) => settings.default_playback_speed = *speed,
            PlaybackSetting::FitMode(mode) => settings.default_fit_mode = mode.clone(),
        }
        let settings = settings.normalized();
        commit_playback_change(
            settings,
            |settings| {
                if !runtime.is_active() {
                    return Ok(());
                }
                match setting {
                    PlaybackSetting::Speed(_) => runtime
                        .set_speed(settings.default_playback_speed, monitor_id)
                        .map_err(|error| format!("Failed to set playback speed: {}", error)),
                    PlaybackSetting::FitMode(_) => runtime
                        .set_fit_mode(settings.default_fit_mode.clone(), monitor_id)
                        .map_err(|error| format!("Failed to set fit mode: {}", error)),
                }
            },
            |settings| settings.save(&db.0.lock()),
            |settings| {
                if let Some(state) = app_handle.try_state::<SettingsState>() {
                    state.set(settings);
                }
            },
        )?;
        let snapshot = runtime.snapshot();
        Ok(snapshot.into())
    })
    .await
    .map_err(|error| format!("Playback update task failed: {}", error))?
}

fn commit_playback_change(
    settings: Settings,
    apply: impl FnOnce(&Settings) -> Result<(), String>,
    persist: impl FnOnce(&Settings) -> Result<(), String>,
    publish: impl FnOnce(Settings),
) -> Result<(), String> {
    apply(&settings)?;
    persist(&settings).map_err(|error| {
        format!(
            "Playback update completed, but its default could not be saved: {}",
            error
        )
    })?;
    publish(settings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn failed_playback_change_does_not_save_or_publish_defaults() {
        let result = commit_playback_change(
            Settings::default(),
            |_| Err("Monitor unavailable".into()),
            |_| panic!("Failed playback must not persist settings"),
            |_| panic!("Failed playback must not publish settings"),
        );
        assert_eq!(result, Err("Monitor unavailable".into()));
    }

    #[test]
    fn playback_save_failure_reports_partial_completion_without_publishing() {
        let calls = RefCell::new(Vec::new());
        let result = commit_playback_change(
            Settings::default(),
            |_| {
                calls.borrow_mut().push("apply");
                Ok(())
            },
            |_| {
                calls.borrow_mut().push("persist");
                Err("Disk full".into())
            },
            |_| panic!("Failed persistence must not publish settings"),
        );
        assert_eq!(*calls.borrow(), vec!["apply", "persist"]);
        let error = result.unwrap_err();
        assert!(error.contains("Playback update completed"));
        assert!(error.contains("Disk full"));
    }

    #[test]
    fn playback_defaults_publish_only_after_apply_and_save_succeed() {
        let calls = RefCell::new(Vec::new());
        let settings = Settings {
            default_playback_speed: 1.5,
            ..Settings::default()
        };
        commit_playback_change(
            settings,
            |settings| {
                assert_eq!(settings.default_playback_speed, 1.5);
                calls.borrow_mut().push("apply");
                Ok(())
            },
            |_| {
                calls.borrow_mut().push("persist");
                Ok(())
            },
            |settings| {
                assert_eq!(settings.default_playback_speed, 1.5);
                calls.borrow_mut().push("publish");
            },
        )
        .unwrap();
        assert_eq!(*calls.borrow(), vec!["apply", "persist", "publish"]);
    }
}
