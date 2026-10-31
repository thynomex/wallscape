use crate::wallpaper::engine::{FitMode, PlayerStatus};
use crate::wallpaper::WallpaperRuntime;
use serde::Serialize;

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
    /// Each display's video wallpaper.
    pub players: Vec<RuntimePlayerStatus>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RuntimePlayerStatus {
    /// Monitor id the player runs on; pass it as `monitorId` to target it.
    pub monitor_id: String,
    /// File the display is playing, as passed to `set_wallpaper`.
    pub video_path: String,
    pub playback_speed: f64,
    pub fit_mode: String,
}

fn fit_mode_name(mode: FitMode) -> String {
    match mode {
        FitMode::Fit => "fit",
        FitMode::Fill => "fill",
        FitMode::Stretch => "stretch",
    }
    .to_string()
}

impl From<PlayerStatus> for RuntimePlayerStatus {
    fn from(player: PlayerStatus) -> Self {
        Self {
            monitor_id: player.slot,
            video_path: player.video_path,
            playback_speed: player.speed,
            fit_mode: fit_mode_name(player.fit_mode),
        }
    }
}

impl From<crate::wallpaper::runtime::RuntimeSnapshot> for RuntimeStatus {
    fn from(snapshot: crate::wallpaper::runtime::RuntimeSnapshot) -> Self {
        Self {
            active: snapshot.active,
            paused: snapshot.paused,
            auto_paused: snapshot.auto_paused,
            playback_speed: snapshot.playback_speed,
            fit_mode: snapshot.fit_mode.map(fit_mode_name),
            players: snapshot.players.into_iter().map(Into::into).collect(),
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

/// Changes the speed of the running wallpaper on one display (or all). The
/// saved default in Settings is left alone; it applies to the next wallpaper.
#[tauri::command]
pub async fn set_wallpaper_speed(
    runtime: tauri::State<'_, WallpaperRuntime>,
    speed: f64,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    if !speed.is_finite() {
        return Err("Playback speed must be a finite number".to_string());
    }
    let runtime = runtime.inner().clone();
    run_live_playback_change(runtime, move |runtime| {
        runtime
            .set_speed(speed, monitor_id)
            .map_err(|error| format!("Failed to set playback speed: {}", error))
    })
    .await
}

/// Changes the fit mode of the running wallpaper on one display (or all). The
/// saved default in Settings is left alone; it applies to the next wallpaper.
#[tauri::command]
pub async fn set_wallpaper_fit_mode(
    runtime: tauri::State<'_, WallpaperRuntime>,
    mode: String,
    monitor_id: Option<String>,
) -> Result<RuntimeStatus, String> {
    let runtime = runtime.inner().clone();
    run_live_playback_change(runtime, move |runtime| {
        runtime
            .set_fit_mode(mode, monitor_id)
            .map_err(|error| format!("Failed to set fit mode: {}", error))
    })
    .await
}

async fn run_live_playback_change(
    runtime: WallpaperRuntime,
    change: impl FnOnce(&WallpaperRuntime) -> Result<(), String> + Send + 'static,
) -> Result<RuntimeStatus, String> {
    tokio::task::spawn_blocking(move || {
        if !runtime.is_active() {
            return Err("No video wallpaper is playing".to_string());
        }
        change(&runtime)?;
        Ok(runtime.snapshot().into())
    })
    .await
    .map_err(|error| format!("Playback update task failed: {}", error))?
}
