mod app_data_migration;
mod auto_pause;
#[cfg(test)]
mod bindings;
mod commands;
mod db;
mod downloads;
mod importing;
mod motionbgs;
mod motionbgs_cache;
mod settings;
mod storage;
mod wallhaven;
mod wallhaven_cache;
mod wallpaper;
mod wallpaper_lifecycle;
mod wallpaper_media;
mod windows_util;

use anyhow::anyhow;
use db::Database;
use parking_lot::Mutex;
use settings::{Settings, SettingsState};
use std::sync::OnceLock;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_single_instance::init as single_instance_init;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use wallpaper::WallpaperRuntime;

/// Label of the single application window (see `tauri.conf.json`).
const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_HISTORY_ID_PREFIX: &str = "history:";
const TRAY_HISTORY_LIMIT: i64 = 5;
static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub struct AppDatabase(pub Mutex<Database>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(single_instance_init(|app, _args, _cwd| {
            tracing::info!("Another Wallscape instance was launched; focusing the existing window");
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            tracing::info!("Wallscape starting up...");
            app.manage(WallpaperRuntime::spawn());

            let database =
                app_data_migration::open_database(app.handle()).map_err(|error| anyhow!(error))?;
            let settings = Settings::load(&database);
            if let Err(error) = wallpaper_lifecycle::backup_original_wallpaper_if_absent(&database)
            {
                tracing::warn!("{}", error);
            }
            app.manage(SettingsState::new(settings.clone()));
            app.manage(AppDatabase(Mutex::new(database)));
            auto_pause::spawn(
                app.handle().clone(),
                app.state::<WallpaperRuntime>().inner().clone(),
            );

            // Only touch OS autostart registration at launch when the user has
            // explicitly enabled it. Disabling happens when the setting changes.
            if settings.launch_at_startup {
                if let Err(error) = commands::sync_autostart(app.handle(), true) {
                    tracing::warn!("{}", error);
                }
            }

            // The window is created hidden (visible:false in tauri.conf.json) so
            // we control first paint here and avoid a flash when starting in tray.
            if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                if settings.start_minimized {
                    tracing::info!("Starting minimized to the system tray");
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Build the system tray. Non-fatal: the app is still usable without it.
            if let Err(error) = build_tray(app) {
                tracing::error!("Failed to build system tray: {}", error);
            }

            // Optionally re-apply the most recent video wallpaper on launch.
            if settings.restore_last_wallpaper {
                restore_last_wallpaper(app.handle());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Intercept the window close: hide to tray instead of quitting when
            // the user has opted into that behavior.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if should_close_to_tray(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::diagnostic::collect_diagnostic_info,
            commands::organization::create_collection,
            commands::organization::delete_collection,
            commands::organization::delete_saved_filter,
            commands::organization::list_collection_memberships,
            commands::organization::list_collections,
            commands::organization::list_saved_filters,
            commands::organization::save_filter,
            commands::organization::set_collection_membership,
            commands::monitors::get_monitors,
            commands::wallpapers::detect_broken_wallpapers,
            commands::wallpapers::import_wallpaper,
            commands::wallpapers::import_wallpapers,
            commands::wallpapers::list_wallpapers,
            commands::wallpapers::remove_wallpaper,
            commands::wallpapers::regenerate_wallpaper_thumbnail,
            commands::wallpapers::reveal_wallpaper_in_explorer,
            commands::wallpapers::set_wallpaper_favorite,
            commands::motionbgs::download_motionbgs_wallpaper,
            commands::motionbgs::search_motionbgs,
            commands::wallhaven::download_wallhaven_wallpaper,
            commands::wallhaven::open_external_url,
            commands::wallpapers::scan_import_paths,
            commands::wallpapers::search_wallpapers,
            commands::wallhaven::search_wallhaven,
            commands::wallpapers::set_wallpaper,
            commands::wallpapers::get_original_wallpaper_backup,
            commands::wallpapers::get_previous_wallpaper,
            commands::wallpapers::restore_previous_wallpaper,
            commands::wallpapers::restore_original_wallpaper,
            commands::wallpapers::rotate_random_favorite_wallpaper,
            commands::wallpapers::list_wallpaper_history,
            commands::wallpapers::undo_wallpaper_history,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::storage::cleanup_missing_library_entries,
            commands::storage::cleanup_unused_thumbnails,
            commands::storage::clear_wallhaven_cache,
            commands::storage::get_storage_stats,
            commands::runtime::get_runtime_status,
            commands::runtime::set_wallpaper_paused,
            commands::runtime::set_wallpaper_speed,
            commands::runtime::set_wallpaper_fit_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging() {
    let log_dir = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Wallscape")
        .join("logs");

    if let Err(error) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create Wallscape log directory: {error}");
    }

    let file_appender = tracing_appender::rolling::never(log_dir, "wallscape.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    #[cfg(debug_assertions)]
    let log_filter = EnvFilter::new("debug");

    #[cfg(not(debug_assertions))]
    let log_filter = EnvFilter::new("wallscape_win_lib=debug");

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(log_filter)
        .init();
}

/// Build the system tray icon and its context menu.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show Wallscape", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "toggle_pause", "Pause wallpaper", true, None::<&str>)?;
    let next_favorite_item =
        MenuItem::with_id(app, "next_favorite", "Next favorite", true, None::<&str>)?;
    let restore_item = MenuItem::with_id(
        app,
        "restore",
        "Restore original wallpaper",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Wallscape", true, None::<&str>)?;
    let recent_history_items = recent_history_items(app)?;
    let recent_history_item_refs = recent_history_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<_>)
        .collect::<Vec<_>>();
    let recent_history_menu = Submenu::with_items(
        app,
        "Recently Used",
        !recent_history_items.is_empty(),
        &recent_history_item_refs,
    )?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &pause_item,
            &next_favorite_item,
            &recent_history_menu,
            &restore_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    // The pause item's label flips between Pause/Resume, so the menu-event
    // handler needs a handle to it.
    let pause_item_handle = pause_item.clone();

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("Wallscape")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "toggle_pause" => {
                let runtime = app.state::<WallpaperRuntime>();
                if runtime.is_active() {
                    match runtime.toggle_paused() {
                        Ok(paused) => {
                            let _ = pause_item_handle.set_text(if paused {
                                "Resume wallpaper"
                            } else {
                                "Pause wallpaper"
                            });
                        }
                        Err(error) => tracing::warn!("Failed to toggle pause: {}", error),
                    }
                }
            }
            "next_favorite" => {
                let app_handle = app.clone();
                let runtime = app.state::<WallpaperRuntime>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    let Some(db) = app_handle.try_state::<AppDatabase>() else {
                        tracing::warn!("Tray favorite rotation skipped: database is unavailable");
                        return;
                    };

                    if let Err(error) =
                        commands::wallpapers::rotate_random_favorite_wallpaper_inner(
                            db.inner(),
                            &runtime,
                        )
                        .await
                    {
                        tracing::warn!("Tray favorite rotation failed: {}", error);
                    }
                });
            }
            "restore" => {
                let runtime = app.state::<WallpaperRuntime>();
                if let Err(error) =
                    commands::restore_original_wallpaper_blocking(app, runtime.inner())
                {
                    tracing::warn!("Tray restore failed: {}", error);
                }
                let _ = pause_item_handle.set_text("Pause wallpaper");
            }
            "quit" => app.exit(0),
            id if id.starts_with(TRAY_HISTORY_ID_PREFIX) => {
                let Some(history_id) = tray_history_id_from_menu_id(id) else {
                    return;
                };
                let app_handle = app.clone();
                let runtime = app.state::<WallpaperRuntime>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    let Some(db) = app_handle.try_state::<AppDatabase>() else {
                        tracing::warn!("Tray history apply skipped: database is unavailable");
                        return;
                    };

                    let entry = match db.0.lock().list_wallpaper_history(50) {
                        Ok(entries) => entries.into_iter().find(|entry| entry.id == history_id),
                        Err(error) => {
                            tracing::warn!("Tray history lookup failed: {}", error);
                            return;
                        }
                    };

                    let Some(entry) = entry else {
                        tracing::warn!("Tray history entry {} is no longer available", history_id);
                        return;
                    };

                    if let Err(error) = wallpaper_lifecycle::apply_wallpaper_path_with_source(
                        db.inner(),
                        &runtime,
                        entry.file_path,
                        entry.target_monitor_id,
                        "tray_history",
                    )
                    .await
                    {
                        tracing::warn!("Tray history apply failed: {}", error);
                    }
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left click toggles the main window's visibility.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

fn recent_history_items(app: &tauri::App) -> tauri::Result<Vec<MenuItem<tauri::Wry>>> {
    let entries = app
        .try_state::<AppDatabase>()
        .and_then(|db| db.0.lock().list_wallpaper_history(TRAY_HISTORY_LIMIT).ok())
        .unwrap_or_default();

    entries
        .into_iter()
        .map(|entry| {
            MenuItem::with_id(
                app,
                tray_history_menu_id(entry.id),
                tray_history_menu_label(&entry.title, entry.target_monitor_id.as_deref()),
                true,
                None::<&str>,
            )
        })
        .collect()
}

fn tray_history_menu_id(history_id: i64) -> String {
    format!("{TRAY_HISTORY_ID_PREFIX}{history_id}")
}

fn tray_history_id_from_menu_id(id: &str) -> Option<i64> {
    id.strip_prefix(TRAY_HISTORY_ID_PREFIX)?.parse().ok()
}

fn tray_history_menu_label(title: &str, target_monitor_id: Option<&str>) -> String {
    match target_monitor_id {
        Some(_) => format!("{title} (Display)"),
        None => title.to_string(),
    }
}

/// Reveal and focus the main window.
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Hide the window if it is currently shown, otherwise reveal and focus it.
fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let visible = window.is_visible().unwrap_or(false);
        let minimized = window.is_minimized().unwrap_or(false);

        if visible && !minimized {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
}

/// Read the `close_to_tray` preference, defaulting when the DB can't be read.
fn should_close_to_tray(app: &tauri::AppHandle) -> bool {
    if let Some(settings) = app.try_state::<SettingsState>() {
        return settings.get().close_to_tray;
    }

    app.try_state::<AppDatabase>()
        .map(|db| Settings::load(&db.0.lock()).close_to_tray)
        .unwrap_or_else(|| Settings::default().close_to_tray)
}

/// Re-apply the most recently used wallpaper without blocking startup.
fn restore_last_wallpaper(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    let runtime = app.state::<WallpaperRuntime>().inner().clone();
    std::thread::spawn(move || {
        let Some(db) = app_handle.try_state::<AppDatabase>() else {
            return;
        };

        tracing::info!("Restoring last wallpaper on launch");
        if let Err(error) =
            wallpaper_lifecycle::restore_last_wallpaper_on_launch(db.inner(), &runtime)
        {
            tracing::warn!("Failed to restore last wallpaper on launch: {}", error);
        }
    });
}

#[cfg(test)]
mod tray_history_tests {
    use super::{tray_history_id_from_menu_id, tray_history_menu_id, tray_history_menu_label};

    #[test]
    fn tray_history_menu_id_round_trips() {
        let id = tray_history_menu_id(42);
        assert_eq!(tray_history_id_from_menu_id(&id), Some(42));
        assert_eq!(tray_history_id_from_menu_id("history:invalid"), None);
    }

    #[test]
    fn tray_history_label_marks_monitor_specific_entries() {
        assert_eq!(tray_history_menu_label("Forest", None), "Forest");
        assert_eq!(
            tray_history_menu_label("Forest", Some("monitor-1")),
            "Forest (Display)"
        );
    }
}
