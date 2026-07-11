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
}

#[tauri::command]
pub async fn get_runtime_status(
    runtime: tauri::State<'_, WallpaperRuntime>,
) -> Result<RuntimeStatus, String> {
    Ok(RuntimeStatus {
        active: runtime.is_active(),
        paused: runtime.is_paused(),
        auto_paused: runtime.is_auto_paused(),
    })
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

    Ok(RuntimeStatus {
        active: runtime.is_active(),
        paused: runtime.is_paused(),
        auto_paused: runtime.is_auto_paused(),
    })
}

#[tauri::command]
pub async fn set_wallpaper_speed(
    app_handle: tauri::AppHandle,
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
    speed: f64,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    let settings = persist_default_speed(db.inner(), speed)?;
    if let Some(settings_state) = app_handle.try_state::<SettingsState>() {
        settings_state.set(settings.clone());
    }
    let speed = settings.default_playback_speed;
    let runtime = runtime.inner().clone();

    if runtime.is_active() {
        let runtime_for_task = runtime.clone();
        tokio::task::spawn_blocking(move || runtime_for_task.set_speed(speed, monitor_id))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to set playback speed: {}", e))?;
    }

    Ok(RuntimeStatus {
        active: runtime.is_active(),
        paused: runtime.is_paused(),
        auto_paused: runtime.is_auto_paused(),
    })
}

#[tauri::command]
pub async fn set_wallpaper_fit_mode(
    app_handle: tauri::AppHandle,
    db: tauri::State<'_, AppDatabase>,
    runtime: tauri::State<'_, WallpaperRuntime>,
    mode: String,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    let settings = persist_default_fit_mode(db.inner(), &mode)?;
    if let Some(settings_state) = app_handle.try_state::<SettingsState>() {
        settings_state.set(settings.clone());
    }
    let mode = settings.default_fit_mode.clone();
    let runtime = runtime.inner().clone();

    if runtime.is_active() {
        let runtime_for_task = runtime.clone();
        tokio::task::spawn_blocking(move || runtime_for_task.set_fit_mode(mode, monitor_id))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to set fit mode: {}", e))?;
    }

    Ok(RuntimeStatus {
        active: runtime.is_active(),
        paused: runtime.is_paused(),
        auto_paused: runtime.is_auto_paused(),
    })
}

fn persist_default_speed(db: &AppDatabase, speed: f64) -> Result<Settings, String> {
    if !speed.is_finite() {
        return Err("Playback speed must be a finite number".to_string());
    }

    let db_guard = db.0.lock();
    let mut settings = Settings::load(&db_guard);
    settings.default_playback_speed = speed;
    let settings = settings.normalized();
    settings.save(&db_guard)?;
    Ok(settings)
}

fn persist_default_fit_mode(db: &AppDatabase, mode: &str) -> Result<Settings, String> {
    let db_guard = db.0.lock();
    let mut settings = Settings::load(&db_guard);
    settings.default_fit_mode = mode.to_string();
    let settings = settings.normalized();
    settings.save(&db_guard)?;
    Ok(settings)
}
