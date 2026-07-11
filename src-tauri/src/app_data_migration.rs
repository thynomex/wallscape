use crate::db::Database;
use std::path::{Path, PathBuf};
use tauri::Manager;

const CURRENT_DB_FILE_NAME: &str = "wallscape.db";
const LEGACY_DB_FILE_NAMES: &[&str] = &["wallspace.db"];
const LEGACY_APP_DATA_DIR_NAMES: &[&str] = &["com.momo.wallspace-win", "wallspace-win"];

pub(crate) fn open_database(app_handle: &tauri::AppHandle) -> Result<Database, String> {
    let db_path = resolve_database_path(app_handle)?;
    Database::new(db_path).map_err(|e| format!("Failed to open database: {}", e))
}

fn resolve_database_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    migrate_legacy_app_data(&data_dir)?;

    Ok(data_dir.join(CURRENT_DB_FILE_NAME))
}

fn migrate_legacy_app_data(current_data_dir: &Path) -> Result<(), String> {
    let current_db_path = current_data_dir.join(CURRENT_DB_FILE_NAME);
    if current_db_path.exists() {
        return Ok(());
    }

    if let Some(parent) = current_data_dir.parent() {
        for legacy_dir_name in LEGACY_APP_DATA_DIR_NAMES {
            let legacy_data_dir = parent.join(legacy_dir_name);
            if !legacy_data_dir.exists() {
                continue;
            }

            if migrate_from_dir(&legacy_data_dir, current_data_dir, &current_db_path)? {
                return Ok(());
            }
        }
    }

    for legacy_db_file_name in LEGACY_DB_FILE_NAMES {
        let legacy_db_path = current_data_dir.join(legacy_db_file_name);
        if !legacy_db_path.exists() {
            continue;
        }

        std::fs::create_dir_all(current_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {}", e))?;
        std::fs::copy(&legacy_db_path, &current_db_path)
            .map_err(|e| format!("Failed to migrate legacy database: {}", e))?;

        tracing::info!(
            "Migrated legacy Wallspace database to {}",
            current_db_path.display()
        );
        break;
    }

    Ok(())
}

fn migrate_from_dir(
    legacy_data_dir: &Path,
    current_data_dir: &Path,
    current_db_path: &Path,
) -> Result<bool, String> {
    for legacy_db_file_name in LEGACY_DB_FILE_NAMES {
        let legacy_db_path = legacy_data_dir.join(legacy_db_file_name);
        if !legacy_db_path.exists() {
            continue;
        }

        std::fs::create_dir_all(current_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {}", e))?;
        std::fs::copy(&legacy_db_path, current_db_path)
            .map_err(|e| format!("Failed to migrate legacy database: {}", e))?;

        let legacy_thumbnails_dir = legacy_data_dir.join("thumbnails");
        let current_thumbnails_dir = current_data_dir.join("thumbnails");
        if legacy_thumbnails_dir.exists() && !current_thumbnails_dir.exists() {
            copy_dir_recursive(&legacy_thumbnails_dir, &current_thumbnails_dir)?;
        }

        tracing::info!(
            "Migrated Wallspace app data from {} to {}",
            legacy_data_dir.display(),
            current_data_dir.display()
        );
        return Ok(true);
    }

    Ok(false)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::create_dir_all(destination)
        .map_err(|e| format!("Failed to create migrated data dir: {}", e))?;

    for entry in
        std::fs::read_dir(source).map_err(|e| format!("Failed to read legacy data dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read legacy data entry: {}", e))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect legacy data entry: {}", e))?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            std::fs::copy(&source_path, &destination_path)
                .map_err(|e| format!("Failed to copy legacy data file: {}", e))?;
        }
    }

    Ok(())
}
