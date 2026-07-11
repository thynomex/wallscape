use crate::wallpaper::MonitorManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_primary: bool,
}

#[tauri::command]
pub async fn get_monitors() -> Result<Vec<Monitor>, String> {
    let manager = MonitorManager::new().map_err(|e| format!("Failed to get monitors: {}", e))?;

    Ok(manager
        .get_monitors()
        .iter()
        .map(|monitor| Monitor {
            id: monitor.id.clone(),
            name: monitor.name.clone(),
            x: monitor.x,
            y: monitor.y,
            width: monitor.width,
            height: monitor.height,
            is_primary: monitor.is_primary,
        })
        .collect())
}
