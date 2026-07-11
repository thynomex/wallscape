use crate::settings::{Settings, SettingsState};
use crate::AppDatabase;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub async fn get_settings(app_handle: tauri::AppHandle) -> Result<Settings, String> {
    if let Some(settings) = app_handle.try_state::<SettingsState>() {
        return Ok(settings.get());
    }

    let db = app_handle
        .try_state::<AppDatabase>()
        .ok_or_else(|| "Database state is not initialized".to_string())?;
    let settings = {
        let db_guard = db.0.lock();
        Settings::load(&db_guard)
    };
    Ok(settings)
}

#[tauri::command]
pub async fn update_settings(
    app_handle: tauri::AppHandle,
    settings: Settings,
) -> Result<Settings, String> {
    let settings = settings.normalized();
    let db = app_handle
        .try_state::<AppDatabase>()
        .ok_or_else(|| "Database state is not initialized".to_string())?;
    let previous_launch_at_startup = {
        let db_guard = db.0.lock();
        let previous = Settings::load(&db_guard).launch_at_startup;
        settings.save(&db_guard)?;
        previous
    };

    if let Some(settings_state) = app_handle.try_state::<SettingsState>() {
        settings_state.set(settings.clone());
    }

    if previous_launch_at_startup != settings.launch_at_startup {
        sync_autostart(&app_handle, settings.launch_at_startup)?;
    }

    Ok(settings)
}

/// Apply the OS autostart registration after the user changes the preference.
pub(crate) fn sync_autostart(app_handle: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app_handle.autolaunch();

    if enabled {
        manager
            .enable()
            .map_err(|e| format!("Failed to enable launch at startup: {}", e))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("Failed to disable launch at startup: {}", e))?;
    }

    Ok(())
}
