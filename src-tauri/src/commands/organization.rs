use crate::db::{Collection, CollectionMembership, SavedFilter};
use crate::AppDatabase;
use serde_json::Value;

const MAX_NAME_CHARS: usize = 64;
const MAX_FILTER_PAYLOAD_BYTES: usize = 8192;

#[tauri::command]
pub async fn list_collections(
    db: tauri::State<'_, AppDatabase>,
) -> Result<Vec<Collection>, String> {
    db.0.lock()
        .list_collections()
        .map_err(|e| format!("Failed to list collections: {}", e))
}

#[tauri::command]
pub async fn create_collection(
    db: tauri::State<'_, AppDatabase>,
    name: String,
) -> Result<Collection, String> {
    let name = normalize_name(&name, "Collection")?;

    db.0.lock()
        .create_collection(&name)
        .map_err(|e| format!("Failed to save collection: {}", e))
}

#[tauri::command]
pub async fn delete_collection(db: tauri::State<'_, AppDatabase>, id: i64) -> Result<(), String> {
    if id <= 0 {
        return Err("Collection was not found".to_string());
    }

    let deleted =
        db.0.lock()
            .delete_collection(id)
            .map_err(|e| format!("Failed to delete collection: {}", e))?;

    if deleted {
        Ok(())
    } else {
        Err(format!("Collection {} was not found", id))
    }
}

#[tauri::command]
pub async fn list_collection_memberships(
    db: tauri::State<'_, AppDatabase>,
) -> Result<Vec<CollectionMembership>, String> {
    db.0.lock()
        .list_collection_memberships()
        .map_err(|e| format!("Failed to list collection memberships: {}", e))
}

#[tauri::command]
pub async fn set_collection_membership(
    db: tauri::State<'_, AppDatabase>,
    collection_id: i64,
    wallpaper_id: i64,
    in_collection: bool,
) -> Result<Collection, String> {
    if collection_id <= 0 {
        return Err("Collection was not found".to_string());
    }

    if wallpaper_id <= 0 {
        return Err("Only saved library wallpapers can be added to collections".to_string());
    }

    db.0.lock()
        .set_collection_membership(collection_id, wallpaper_id, in_collection)
        .map_err(|e| format!("Failed to update collection: {}", e))?
        .ok_or_else(|| "Collection or wallpaper was not found".to_string())
}

#[tauri::command]
pub async fn list_saved_filters(
    db: tauri::State<'_, AppDatabase>,
    filter_type: Option<String>,
) -> Result<Vec<SavedFilter>, String> {
    let filter_type = filter_type
        .as_deref()
        .map(normalize_filter_type)
        .transpose()?;

    db.0.lock()
        .list_saved_filters(filter_type.as_deref())
        .map_err(|e| format!("Failed to list saved filters: {}", e))
}

#[tauri::command]
pub async fn save_filter(
    db: tauri::State<'_, AppDatabase>,
    name: String,
    filter_type: String,
    payload: String,
) -> Result<SavedFilter, String> {
    let name = normalize_name(&name, "Saved filter")?;
    let filter_type = normalize_filter_type(&filter_type)?;
    let payload = normalize_filter_payload(&payload)?;

    db.0.lock()
        .save_filter(&name, &filter_type, &payload)
        .map_err(|e| format!("Failed to save filter: {}", e))
}

#[tauri::command]
pub async fn delete_saved_filter(db: tauri::State<'_, AppDatabase>, id: i64) -> Result<(), String> {
    if id <= 0 {
        return Err("Saved filter was not found".to_string());
    }

    let deleted =
        db.0.lock()
            .delete_saved_filter(id)
            .map_err(|e| format!("Failed to delete saved filter: {}", e))?;

    if deleted {
        Ok(())
    } else {
        Err(format!("Saved filter {} was not found", id))
    }
}

fn normalize_name(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{} name is required", label));
    }

    if trimmed.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "{} name must be {} characters or fewer",
            label, MAX_NAME_CHARS
        ));
    }

    Ok(trimmed.to_string())
}

fn normalize_filter_type(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "local" | "wallhaven" => Ok(normalized),
        _ => Err("Saved filter type must be local or wallhaven".to_string()),
    }
}

fn normalize_filter_payload(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Saved filter payload is required".to_string());
    }

    if trimmed.len() > MAX_FILTER_PAYLOAD_BYTES {
        return Err("Saved filter payload is too large".to_string());
    }

    let parsed: Value =
        serde_json::from_str(trimmed).map_err(|_| "Saved filter payload must be valid JSON")?;
    if !parsed.is_object() {
        return Err("Saved filter payload must be a JSON object".to_string());
    }

    Ok(parsed.to_string())
}
