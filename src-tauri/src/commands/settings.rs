use crate::settings::{Settings, SettingsState};
use crate::AppDatabase;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

static SETTINGS_UPDATE: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
static RECONCILE_AUTOSTART: AtomicBool = AtomicBool::new(false);

pub(super) fn settings_update_lock() -> parking_lot::MutexGuard<'static, ()> {
    SETTINGS_UPDATE.lock()
}

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
    tokio::task::spawn_blocking(move || {
        let _update = settings_update_lock();
        let settings = settings.normalized();
        let db = app_handle
            .try_state::<AppDatabase>()
            .ok_or_else(|| "Database state is not initialized".to_string())?;
        let db_guard = db.0.lock();
        let previous = Settings::load(&db_guard);
        let mut reconcile_autostart = RECONCILE_AUTOSTART.load(Ordering::Relaxed);
        let result = commit_settings_change(
            &previous,
            &settings,
            &mut reconcile_autostart,
            |enabled| sync_autostart(&app_handle, enabled),
            |settings| settings.save(&db_guard),
        );
        RECONCILE_AUTOSTART.store(reconcile_autostart, Ordering::Relaxed);
        result?;
        if let Some(settings_state) = app_handle.try_state::<SettingsState>() {
            settings_state.set(settings.clone());
        }
        Ok(settings)
    })
    .await
    .map_err(|error| format!("Settings update task failed: {}", error))?
}

fn commit_settings_change(
    previous: &Settings,
    next: &Settings,
    needs_reconciliation: &mut bool,
    mut sync: impl FnMut(bool) -> Result<(), String>,
    persist: impl FnOnce(&Settings) -> Result<(), String>,
) -> Result<(), String> {
    // Unrelated preferences must not depend on registry availability. Retry an
    // unchanged preference only when an earlier rollback left registration dirty.
    if previous.launch_at_startup == next.launch_at_startup && !*needs_reconciliation {
        return persist(next);
    }
    let result = sync(next.launch_at_startup).and_then(|()| persist(next));
    if let Err(error) = result {
        return match sync(previous.launch_at_startup) {
            Ok(()) => {
                *needs_reconciliation = false;
                Err(error)
            }
            Err(rollback) => {
                *needs_reconciliation = true;
                Err(format!(
                    "{}; failed to restore autostart: {}",
                    error, rollback
                ))
            }
        };
    }
    *needs_reconciliation = false;
    Ok(())
}

/// Apply the OS autostart registration after the user changes the preference.
pub(crate) fn sync_autostart(app_handle: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app_handle.autolaunch();

    sync_autostart_registration(
        enabled,
        || {
            manager
                .enable()
                .map_err(|error| format!("Failed to enable launch at startup: {}", error))
        },
        || {
            manager
                .disable()
                .map_err(|error| format!("Failed to disable launch at startup: {}", error))
        },
    )
}

fn sync_autostart_registration(
    enabled: bool,
    enable: impl FnOnce() -> Result<(), String>,
    disable: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if enabled {
        // Windows status does not verify the registered executable's path.
        // Refresh it after moves/upgrades even when startup is already enabled.
        enable()
    } else {
        // A Task Manager override can report disabled while a Run entry remains.
        // The plugin treats an absent registration as a successful no-op.
        disable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn enabled_autostart_repairs_registration_for_previous_executable() {
        let registered_command = RefCell::new("C:\\old\\Wallscape.exe".to_string());
        sync_autostart_registration(
            true,
            || {
                *registered_command.borrow_mut() = "C:\\new\\Wallscape.exe".to_string();
                Ok(())
            },
            || panic!("Enabled autostart must not disable registration"),
        )
        .unwrap();

        assert_eq!(*registered_command.borrow(), "C:\\new\\Wallscape.exe");
    }

    #[test]
    fn disabled_autostart_accepts_absent_registration() {
        let registered_command: RefCell<Option<String>> = RefCell::new(None);
        let removal_attempted = Cell::new(false);
        sync_autostart_registration(
            false,
            || panic!("Disabled autostart must not enable registration"),
            || {
                removal_attempted.set(true);
                registered_command.borrow_mut().take();
                Ok(())
            },
        )
        .unwrap();

        assert!(removal_attempted.get());
        assert!(registered_command.borrow().is_none());
    }

    #[test]
    fn disabled_autostart_removes_task_manager_disabled_registration() {
        let registered_command = RefCell::new(Some("C:\\old\\Wallscape.exe".to_string()));
        let task_manager_enabled = false;
        let reported_enabled = registered_command.borrow().is_some() && task_manager_enabled;
        assert!(!reported_enabled);

        sync_autostart_registration(
            false,
            || panic!("Disabled autostart must not enable registration"),
            || {
                registered_command.borrow_mut().take();
                Ok(())
            },
        )
        .unwrap();

        assert!(registered_command.borrow().is_none());
    }

    #[test]
    fn failed_autostart_change_is_not_persisted_and_retry_runs_again() {
        let previous = Settings::default();
        let next = Settings {
            launch_at_startup: true,
            ..previous.clone()
        };
        let calls = RefCell::new(Vec::new());
        let persisted = Cell::new(false);
        let result = commit_settings_change(
            &previous,
            &next,
            &mut false,
            |enabled| {
                calls.borrow_mut().push(enabled);
                if enabled {
                    Err("registration failed".to_string())
                } else {
                    Ok(())
                }
            },
            |_| {
                persisted.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!persisted.get());
        assert_eq!(*calls.borrow(), vec![true, false]);

        commit_settings_change(
            &previous,
            &next,
            &mut false,
            |enabled| {
                calls.borrow_mut().push(enabled);
                Ok(())
            },
            |_| {
                persisted.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(persisted.get());
        assert_eq!(*calls.borrow(), vec![true, false, true]);
    }

    #[test]
    fn failed_persistence_rolls_back_registration() {
        let previous = Settings::default();
        let next = Settings {
            launch_at_startup: true,
            ..previous.clone()
        };
        let actual = Cell::new(false);
        let result = commit_settings_change(
            &previous,
            &next,
            &mut false,
            |enabled| {
                actual.set(enabled);
                Ok(())
            },
            |_| Err("database unavailable".to_string()),
        );
        assert_eq!(result.unwrap_err(), "database unavailable");
        assert!(!actual.get());
    }

    #[test]
    fn unchanged_preference_reconciles_registration_after_failed_rollback() {
        let settings = Settings::default();
        let actual = Cell::new(true);
        commit_settings_change(
            &settings,
            &settings,
            &mut true,
            |enabled| {
                actual.set(enabled);
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert!(!actual.get());
    }

    #[test]
    fn reports_both_persistence_and_rollback_failures() {
        let previous = Settings::default();
        let next = Settings {
            launch_at_startup: true,
            ..previous.clone()
        };
        let mut needs_reconciliation = false;
        let result = commit_settings_change(
            &previous,
            &next,
            &mut needs_reconciliation,
            |enabled| {
                if enabled {
                    Ok(())
                } else {
                    Err("rollback denied".to_string())
                }
            },
            |_| Err("save failed".to_string()),
        );
        let error = result.unwrap_err();
        assert!(error.contains("save failed"));
        assert!(error.contains("rollback denied"));
        assert!(needs_reconciliation);
    }

    #[test]
    fn unrelated_settings_save_does_not_access_autostart_registry() {
        let previous = Settings::default();
        let next = Settings {
            close_to_tray: false,
            ..previous.clone()
        };
        let persisted = Cell::new(false);
        commit_settings_change(
            &previous,
            &next,
            &mut false,
            |_| panic!("unrelated preferences should not access autostart"),
            |_| {
                persisted.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(persisted.get());
    }
}
