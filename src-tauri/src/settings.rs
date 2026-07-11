use crate::db::Database;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Preference key under which the JSON-serialized [`Settings`] blob is stored.
pub const SETTINGS_KEY: &str = "app_settings";
/// Preference key holding the most recently applied video wallpaper path.
pub const LAST_WALLPAPER_KEY: &str = "last_wallpaper_path";
/// Preference key holding the wallpaper that was active before the latest apply.
pub const PREVIOUS_WALLPAPER_KEY: &str = "previous_wallpaper_path";

pub const DEFAULT_FAVORITE_ROTATION_INTERVAL_MINUTES: u32 = 30;
pub const MIN_FAVORITE_ROTATION_INTERVAL_MINUTES: u32 = 1;
pub const MAX_FAVORITE_ROTATION_INTERVAL_MINUTES: u32 = 1_440;

/// Playback speed constraints for video wallpapers.
pub const MIN_PLAYBACK_SPEED: f64 = 0.25;
pub const MAX_PLAYBACK_SPEED: f64 = 2.0;
pub const DEFAULT_PLAYBACK_SPEED: f64 = 1.0;

/// User-facing quality-of-life settings, persisted as a single JSON blob in the
/// existing `preferences` table.
///
/// `#[serde(default)]` keeps previously stored blobs loadable as new fields are
/// added over time — missing keys fall back to [`Default`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Launch Wallscape automatically when the user signs in to Windows.
    pub launch_at_startup: bool,
    /// Start hidden in the system tray instead of showing the main window.
    pub start_minimized: bool,
    /// Closing the window hides it to the tray instead of quitting the app.
    pub close_to_tray: bool,
    /// Minimizing the window hides it to the tray instead of the taskbar.
    pub minimize_to_tray: bool,
    /// Re-apply the most recently used video wallpaper on launch.
    pub restore_last_wallpaper: bool,
    /// Rotate through favorite wallpapers on a timer while Wallscape is running.
    pub favorite_rotation_enabled: bool,
    /// Minutes between automatic favorite wallpaper rotations.
    pub favorite_rotation_interval_minutes: u32,
    /// Apply a random favorite wallpaper when Wallscape starts.
    pub favorite_rotation_on_startup: bool,
    /// Automatically pause live wallpapers when selected rules match.
    pub auto_pause_enabled: bool,
    /// Pause live wallpapers while the machine is running on battery power.
    pub auto_pause_on_battery: bool,
    /// Pause live wallpapers while a fullscreen foreground app is active.
    pub auto_pause_fullscreen_apps: bool,
    /// Pause live wallpapers while every monitor is covered by other windows.
    pub auto_pause_occluded: bool,
    /// Pause live wallpapers while the session is remote desktop.
    pub auto_pause_remote_session: bool,
    /// Pause live wallpapers when the interactive desktop is unavailable.
    pub auto_pause_display_sleep: bool,
    /// Default playback speed for video wallpapers (0.25x to 2.0x).
    pub default_playback_speed: f64,
    /// Default video fit mode for wallpapers: "fit", "fill", or "stretch".
    pub default_fit_mode: String,
    /// Default FPS cap for video wallpapers (None = uncapped).
    pub default_fps_cap: Option<u32>,
}

pub struct SettingsState {
    settings: RwLock<Settings>,
}

impl SettingsState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: RwLock::new(settings.normalized()),
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn set(&self, settings: Settings) {
        *self.settings.write() = settings.normalized();
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            start_minimized: false,
            close_to_tray: true,
            minimize_to_tray: false,
            restore_last_wallpaper: true,
            favorite_rotation_enabled: false,
            favorite_rotation_interval_minutes: DEFAULT_FAVORITE_ROTATION_INTERVAL_MINUTES,
            favorite_rotation_on_startup: false,
            auto_pause_enabled: true,
            auto_pause_on_battery: true,
            auto_pause_fullscreen_apps: true,
            auto_pause_occluded: true,
            auto_pause_remote_session: true,
            auto_pause_display_sleep: true,
            default_playback_speed: DEFAULT_PLAYBACK_SPEED,
            default_fit_mode: "fit".to_string(),
            default_fps_cap: None,
        }
    }
}

impl Settings {
    /// Load settings from the database, falling back to defaults when unset or
    /// when the stored blob cannot be parsed.
    pub fn load(db: &Database) -> Self {
        let settings = match db.get_preference(SETTINGS_KEY) {
            Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
            Ok(None) => Settings::default(),
            Err(error) => {
                tracing::warn!("Failed to read settings, using defaults: {}", error);
                Settings::default()
            }
        };

        settings.normalized()
    }

    /// Persist the settings as a JSON blob in the `preferences` table.
    pub fn save(&self, db: &Database) -> Result<(), String> {
        let settings = self.normalized();
        let json = serde_json::to_string(&settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        db.set_preference(SETTINGS_KEY, &json)
            .map_err(|e| format!("Failed to save settings: {}", e))
    }

    pub fn normalized(&self) -> Self {
        let mut settings = self.clone();
        settings.favorite_rotation_interval_minutes =
            settings.favorite_rotation_interval_minutes.clamp(
                MIN_FAVORITE_ROTATION_INTERVAL_MINUTES,
                MAX_FAVORITE_ROTATION_INTERVAL_MINUTES,
            );
        settings.default_playback_speed = settings
            .default_playback_speed
            .clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED);

        // Normalize fit mode to valid values
        if !matches!(
            settings.default_fit_mode.as_str(),
            "fit" | "fill" | "stretch"
        ) {
            settings.default_fit_mode = "fit".to_string();
        }

        // Validate FPS cap if present
        if let Some(fps) = settings.default_fps_cap {
            if !(1..=240).contains(&fps) {
                settings.default_fps_cap = None;
            }
        }

        settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_settings_default_for_existing_saved_blobs() {
        let settings: Settings = serde_json::from_str("{}").expect("empty blob should deserialize");

        assert!(!settings.favorite_rotation_enabled);
        assert_eq!(
            settings.favorite_rotation_interval_minutes,
            DEFAULT_FAVORITE_ROTATION_INTERVAL_MINUTES
        );
        assert!(!settings.favorite_rotation_on_startup);
    }

    #[test]
    fn auto_pause_settings_default_for_existing_saved_blobs() {
        let settings: Settings = serde_json::from_str("{}").expect("empty blob should deserialize");

        assert!(settings.auto_pause_enabled);
        assert!(settings.auto_pause_on_battery);
        assert!(settings.auto_pause_fullscreen_apps);
        assert!(settings.auto_pause_occluded);
        assert!(settings.auto_pause_remote_session);
        assert!(settings.auto_pause_display_sleep);
    }

    #[test]
    fn rotation_interval_is_clamped() {
        let low = Settings {
            favorite_rotation_interval_minutes: 0,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(
            low.favorite_rotation_interval_minutes,
            MIN_FAVORITE_ROTATION_INTERVAL_MINUTES
        );

        let high = Settings {
            favorite_rotation_interval_minutes: MAX_FAVORITE_ROTATION_INTERVAL_MINUTES + 1,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(
            high.favorite_rotation_interval_minutes,
            MAX_FAVORITE_ROTATION_INTERVAL_MINUTES
        );
    }

    #[test]
    fn playback_settings_default_for_existing_saved_blobs() {
        let settings: Settings = serde_json::from_str("{}").expect("empty blob should deserialize");

        assert_eq!(settings.default_playback_speed, DEFAULT_PLAYBACK_SPEED);
        assert_eq!(settings.default_fit_mode, "fit");
        assert_eq!(settings.default_fps_cap, None);
    }

    #[test]
    fn playback_speed_is_clamped() {
        let low = Settings {
            default_playback_speed: 0.1,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(low.default_playback_speed, MIN_PLAYBACK_SPEED);

        let high = Settings {
            default_playback_speed: 5.0,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(high.default_playback_speed, MAX_PLAYBACK_SPEED);

        let valid = Settings {
            default_playback_speed: 1.5,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(valid.default_playback_speed, 1.5);
    }

    #[test]
    fn fit_mode_is_normalized() {
        let invalid = Settings {
            default_fit_mode: "invalid".to_string(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(invalid.default_fit_mode, "fit");

        let valid_fit = Settings {
            default_fit_mode: "fit".to_string(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(valid_fit.default_fit_mode, "fit");

        let valid_fill = Settings {
            default_fit_mode: "fill".to_string(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(valid_fill.default_fit_mode, "fill");

        let valid_stretch = Settings {
            default_fit_mode: "stretch".to_string(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(valid_stretch.default_fit_mode, "stretch");
    }

    #[test]
    fn fps_cap_is_validated() {
        let too_low = Settings {
            default_fps_cap: Some(0),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(too_low.default_fps_cap, None);

        let too_high = Settings {
            default_fps_cap: Some(300),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(too_high.default_fps_cap, None);

        let valid_30 = Settings {
            default_fps_cap: Some(30),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(valid_30.default_fps_cap, Some(30));

        let valid_60 = Settings {
            default_fps_cap: Some(60),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(valid_60.default_fps_cap, Some(60));

        let none = Settings {
            default_fps_cap: None,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(none.default_fps_cap, None);
    }
}
