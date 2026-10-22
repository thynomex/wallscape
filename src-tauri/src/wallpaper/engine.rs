use super::desktop_host::find_or_create_desktop_parent;
use super::desktop_parent::{DesktopParent, DesktopParentKind};
use super::monitor::{Monitor, MonitorManager};
use super::placement::{PlayerPlacement, ALL_MONITORS_SLOT};
use super::{VideoPlayer, WallpaperTarget};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
};

/// Playback options for video wallpapers
#[derive(Debug, Clone, Copy)]
pub struct PlaybackOptions {
    pub speed: f64,
    pub fit_mode: FitMode,
    pub fps_limit: Option<u32>,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            speed: 1.0,
            fit_mode: FitMode::Fit,
            fps_limit: None,
        }
    }
}

/// Video fit mode controlling aspect ratio and scaling behavior
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FitMode {
    /// Maintain aspect ratio, fit within screen bounds
    #[default]
    Fit,
    /// Fill screen, maintain aspect but may crop
    Fill,
    /// Stretch to fill screen, ignore aspect ratio
    Stretch,
}

/// Per-slot playback state
#[derive(Debug, Clone, Copy)]
struct SlotPlaybackState {
    speed: f64,
    fit_mode: FitMode,
}

impl Default for SlotPlaybackState {
    fn default() -> Self {
        Self {
            speed: 1.0,
            fit_mode: FitMode::Fit,
        }
    }
}

/// Main wallpaper engine that manages desktop integration
pub struct WallpaperEngine {
    desktop_parent: Option<DesktopParent>,
    video_players: HashMap<String, VideoPlayer>,
    playback_state: HashMap<String, SlotPlaybackState>,
    paused: bool,
}

impl WallpaperEngine {
    pub fn service_video_loops(&mut self) {
        for (slot, player) in self.video_players.iter_mut() {
            if let Err(error) = player.service_video_loop() {
                tracing::warn!(
                    "Failed to advance capped wallpaper loop on {}: {}",
                    slot,
                    error
                );
            }
        }
    }

    pub fn new() -> Result<Self> {
        Ok(Self {
            desktop_parent: None,
            video_players: HashMap::new(),
            playback_state: HashMap::new(),
            paused: false,
        })
    }

    /// Initialize desktop wallpaper injection
    /// This uses the "WorkerW" window technique used by Wallpaper Engine
    pub fn initialize(&mut self) -> Result<()> {
        tracing::debug!("Initializing wallpaper engine");

        // Find the WorkerW window that sits between desktop icons and wallpaper
        // This is done by sending a message to spawn it if it doesn't exist
        let desktop_parent = find_or_create_desktop_parent()?;
        self.desktop_parent = Some(desktop_parent);

        match desktop_parent.kind {
            DesktopParentKind::WorkerW => {
                tracing::info!("Wallpaper engine initialized with WorkerW desktop parent");
            }
            DesktopParentKind::RaisedDesktop => {
                tracing::info!("Wallpaper engine initialized with Windows raised-desktop parent");
            }
            DesktopParentKind::ProgmanFallback => {
                tracing::warn!(
                    "Wallpaper engine initialized with Progman fallback. Explorer did not expose a usable WorkerW host; desktop icons may need to remain above the video child."
                );
            }
        }

        Ok(())
    }

    /// Set a video as the desktop wallpaper with custom playback options
    pub fn set_video_wallpaper_with_options(
        &mut self,
        video_path: &str,
        target: WallpaperTarget,
        options: PlaybackOptions,
    ) -> Result<()> {
        use std::time::Instant;
        let start_total = Instant::now();
        tracing::debug!("Setting video wallpaper: {}", video_path);

        let paused = self.paused;
        let desktop_parent = self.desktop_parent;
        let replace_all = matches!(target, WallpaperTarget::All);
        let (slots, retired_players) = replace_players(
            &mut self.video_players,
            replace_all,
            || {
                // Prepare the replacement without disturbing any running players.
                let t1 = Instant::now();
                let desktop_parent = desktop_parent
                    .ok_or_else(|| anyhow!("Engine not initialized - no desktop parent window"))?;
                tracing::debug!("Desktop parent check: {:?}", t1.elapsed());

                let t2 = Instant::now();
                let placements = Self::prepare_target(target)?;
                tracing::debug!("Prepare target: {:?}", t2.elapsed());

                if matches!(desktop_parent.kind, DesktopParentKind::ProgmanFallback) {
                    tracing::warn!(
                "Creating video child under Progman fallback. If the video is not visible, collect the wallpaper diagnostic log; Explorer did not provide a normal WorkerW host."
            );
                }

                let mut next_players = Vec::with_capacity(placements.len());
                for placement in placements {
                    // Create video player
                    let t3 = Instant::now();
                    let mut player = VideoPlayer::new(video_path)?;
                    tracing::debug!("VideoPlayer::new: {:?}", t3.elapsed());

                    // Create the player window as a child of the selected desktop parent.
                    let t4 = Instant::now();
                    let (active_parent, placement, player_hwnd) =
                        Self::create_player_window(&mut player, desktop_parent, &placement)?;
                    tracing::debug!(
                        "Created video player window: {:?} using {:?} parent (took {:?})",
                        player_hwnd,
                        active_parent.kind,
                        t4.elapsed()
                    );

                    let t5 = Instant::now();
                    if active_parent.requires_layered_child() {
                        player.set_bounds_preserve_z_order(
                            placement.x,
                            placement.y,
                            placement.width,
                            placement.height,
                        )?;
                    } else {
                        player.set_bounds(
                            placement.x,
                            placement.y,
                            placement.width,
                            placement.height,
                        )?;
                    }
                    Self::configure_player_z_order(active_parent, player_hwnd)?;
                    tracing::debug!("Set bounds and z-order: {:?}", t5.elapsed());

                    // Start playback (video will appear when ready)
                    let t7 = Instant::now();
                    player.play()?;
                    tracing::debug!("Play and wait for video output: {:?}", t7.elapsed());

                    // Apply playback options after playback starts
                    let t8 = Instant::now();
                    if options.speed != 1.0 {
                        player.set_speed(options.speed)?;
                    }
                    if options.fit_mode != FitMode::Fit {
                        player.set_aspect_mode(options.fit_mode)?;
                    }
                    if let Some(fps) = options.fps_limit {
                        player.set_fps_cap(Some(fps))?;
                    }
                    tracing::debug!("Applied playback options: {:?}", t8.elapsed());

                    player.set_paused(paused)?;
                    next_players.push((placement.slot, player));
                }
                // Keep every old player until all replacements have started.
                for (_, player) in &next_players {
                    player.show()?;
                }
                Ok(next_players)
            },
        )?;

        // Committing succeeded. Retiring the old players cannot undo the replacement.
        for (retired_slot, mut player) in retired_players {
            if let Err(error) = player.stop() {
                tracing::warn!(
                    "Failed to stop retired player for slot '{}': {}",
                    retired_slot,
                    error
                );
            }
        }
        self.playback_state
            .retain(|slot, _| self.video_players.contains_key(slot));

        // Store the player state
        let state = SlotPlaybackState {
            speed: options.speed,
            fit_mode: options.fit_mode,
        };
        for slot in slots {
            self.playback_state.insert(slot, state);
        }

        tracing::info!(
            "Video wallpaper set successfully (total: {:?})",
            start_total.elapsed()
        );
        Ok(())
    }

    fn create_player_window(
        player: &mut VideoPlayer,
        desktop_parent: DesktopParent,
        placement: &PlayerPlacement,
    ) -> Result<(DesktopParent, PlayerPlacement, HWND)> {
        let primary_placement = placement
            .clone()
            .relative_to_parent(desktop_parent.hwnd())?;
        match player.create_window(
            Some(desktop_parent.hwnd()),
            desktop_parent.requires_layered_child(),
        ) {
            Ok(hwnd) => Ok((desktop_parent, primary_placement, hwnd)),
            Err(primary_error) => {
                if !matches!(desktop_parent.kind, DesktopParentKind::RaisedDesktop) {
                    return Err(primary_error);
                }

                let Some(workerw) = desktop_parent.workerw else {
                    return Err(primary_error.context(
                        "Raised desktop parent failed and no WorkerW fallback was available",
                    ));
                };

                let fallback_parent = DesktopParent::workerw(workerw);
                tracing::warn!(
                    "Raised desktop parent rejected the video child window; retrying under WorkerW {:?}: {}",
                    fallback_parent.hwnd(),
                    primary_error
                );

                let fallback_placement =
                    placement
                        .clone()
                        .relative_to_parent(fallback_parent.hwnd())
                        .map_err(|fallback_error| {
                            anyhow!(
                                "Raised desktop parent rejected the video child window, and WorkerW fallback placement failed. Raised-desktop error: {}; WorkerW placement error: {}",
                                primary_error,
                                fallback_error
                            )
                        })?;

                match player.create_window(Some(fallback_parent.hwnd()), false) {
                    Ok(hwnd) => {
                        tracing::info!(
                            "Created video player window under WorkerW fallback after raised-desktop parent failed"
                        );
                        Ok((fallback_parent, fallback_placement, hwnd))
                    }
                    Err(fallback_error) => Err(anyhow!(
                        "Failed to create video player window under raised-desktop parent and WorkerW fallback. Raised-desktop error: {}; WorkerW fallback error: {}",
                        primary_error,
                        fallback_error
                    )),
                }
            }
        }
    }

    fn prepare_target(target: WallpaperTarget) -> Result<Vec<PlayerPlacement>> {
        match target {
            WallpaperTarget::All => {
                let manager = MonitorManager::new()?;
                placements_for_monitors(manager.get_monitors())
            }
            WallpaperTarget::Monitor(monitor_id) => {
                Ok(vec![placement_for_monitor(monitor_by_id(&monitor_id)?)])
            }
        }
    }

    /// Pause or resume the active video wallpaper, if one is running.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        let result = update_players_pause(
            &mut self.video_players,
            self.paused,
            paused,
            VideoPlayer::set_paused,
        );
        self.playback_state
            .retain(|slot, _| self.video_players.contains_key(slot));
        result?;
        self.paused = paused;
        Ok(())
    }

    /// Set playback speed for all active players or a specific slot
    pub fn set_playback_speed(&mut self, speed: f64, slot: Option<&str>) -> Result<()> {
        let clamped_speed = speed.clamp(0.1, 10.0);

        match slot {
            Some(slot_id) => {
                // Set speed for specific slot
                if let Some(player) = self.video_players.get_mut(slot_id) {
                    player.set_speed(clamped_speed)?;
                    if let Some(state) = self.playback_state.get_mut(slot_id) {
                        state.speed = clamped_speed;
                    }
                    tracing::debug!(
                        "Set playback speed to {}x for slot '{}'",
                        clamped_speed,
                        slot_id
                    );
                } else {
                    return Err(anyhow!("No active player for slot '{}'", slot_id));
                }
            }
            None => {
                // Set speed for all active players
                for (slot_id, player) in self.video_players.iter_mut() {
                    player.set_speed(clamped_speed)?;
                    if let Some(state) = self.playback_state.get_mut(slot_id) {
                        state.speed = clamped_speed;
                    }
                }
                tracing::debug!("Set playback speed to {}x for all players", clamped_speed);
            }
        }

        Ok(())
    }

    /// Set fit mode for all active players or a specific slot
    pub fn set_fit_mode(&mut self, fit_mode: FitMode, slot: Option<&str>) -> Result<()> {
        match slot {
            Some(slot_id) => {
                // Set fit mode for specific slot
                if let Some(player) = self.video_players.get_mut(slot_id) {
                    player.set_aspect_mode(fit_mode)?;
                    if let Some(state) = self.playback_state.get_mut(slot_id) {
                        state.fit_mode = fit_mode;
                    }
                    tracing::debug!("Set fit mode to {:?} for slot '{}'", fit_mode, slot_id);
                } else {
                    return Err(anyhow!("No active player for slot '{}'", slot_id));
                }
            }
            None => {
                // Set fit mode for all active players
                for (slot_id, player) in self.video_players.iter_mut() {
                    player.set_aspect_mode(fit_mode)?;
                    if let Some(state) = self.playback_state.get_mut(slot_id) {
                        state.fit_mode = fit_mode;
                    }
                }
                tracing::debug!("Set fit mode to {:?} for all players", fit_mode);
            }
        }

        Ok(())
    }

    /// Stop the wallpaper engine
    pub fn stop(&mut self) -> Result<()> {
        self.stop_all_players()?;
        self.desktop_parent = None;

        tracing::info!("Wallpaper engine stopped");
        Ok(())
    }

    pub fn stop_target(&mut self, target: WallpaperTarget) -> Result<()> {
        match target {
            WallpaperTarget::All => self.stop_all_players()?,
            WallpaperTarget::Monitor(monitor_id) => {
                self.stop_player(ALL_MONITORS_SLOT)?;
                self.stop_player(&monitor_id)?;
            }
        }

        Ok(())
    }

    pub fn has_active_players(&self) -> bool {
        !self.video_players.is_empty()
    }

    /// None means no active player or differing values across displays.
    pub fn playback_values(&self) -> (Option<f64>, Option<FitMode>) {
        let Some(first) = self.playback_state.values().next() else {
            return (None, None);
        };
        let speed = self
            .playback_state
            .values()
            .all(|state| state.speed == first.speed)
            .then_some(first.speed);
        let fit_mode = self
            .playback_state
            .values()
            .all(|state| state.fit_mode == first.fit_mode)
            .then_some(first.fit_mode);
        (speed, fit_mode)
    }

    fn stop_player(&mut self, slot: &str) -> Result<()> {
        if let Some(mut player) = self.video_players.remove(slot) {
            self.playback_state.remove(slot);
            if let Err(error) = player.stop() {
                // Dropping the removed player still releases mpv and its window.
                tracing::warn!(
                    "Failed to send stop to retired player '{}': {}",
                    slot,
                    error
                );
            }
            tracing::debug!("Stopped video player for slot '{}'", slot);
        }

        Ok(())
    }

    fn stop_all_players(&mut self) -> Result<()> {
        self.playback_state.clear();
        for (slot, mut player) in self.video_players.drain() {
            if let Err(error) = player.stop() {
                tracing::warn!(
                    "Failed to send stop to retired player '{}': {}",
                    slot,
                    error
                );
            }
            tracing::debug!("Stopped video player for slot '{}'", slot);
        }
        Ok(())
    }

    fn configure_player_z_order(desktop_parent: DesktopParent, player_hwnd: HWND) -> Result<()> {
        if !matches!(desktop_parent.kind, DesktopParentKind::RaisedDesktop) {
            return Ok(());
        }

        let Some(shell_defview) = desktop_parent.shell_defview else {
            return Err(anyhow!(
                "Raised desktop parent is missing SHELLDLL_DefView for wallpaper z-order"
            ));
        };

        unsafe {
            if let Err(error) = SetWindowPos(
                player_hwnd,
                Some(HWND(shell_defview as *mut _)),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) {
                tracing::warn!("Failed to place wallpaper below desktop icons: {}", error);
            }
        }

        ensure_raised_workerw_bottom(desktop_parent)?;
        Ok(())
    }
}

type PlayerReplacement<P> = (Vec<String>, Vec<(String, P)>);

fn replace_players<P>(
    players: &mut HashMap<String, P>,
    replace_all: bool,
    prepare: impl FnOnce() -> Result<Vec<(String, P)>>,
) -> Result<PlayerReplacement<P>> {
    let replacements = prepare()?;
    if replacements.is_empty() {
        return Err(anyhow!("No replacement wallpaper players were prepared"));
    }
    let slots: Vec<_> = replacements.iter().map(|(slot, _)| slot.clone()).collect();
    let conflicts: Vec<_> = players
        .keys()
        .filter(|existing| {
            replace_all || *existing == ALL_MONITORS_SLOT || slots.contains(existing)
        })
        .cloned()
        .collect();
    let retired = conflicts
        .into_iter()
        .filter_map(|key| players.remove(&key).map(|player| (key, player)))
        .collect();
    players.extend(replacements);
    Ok((slots, retired))
}

fn update_players_pause<P>(
    players: &mut HashMap<String, P>,
    previous: bool,
    next: bool,
    mut apply: impl FnMut(&mut P, bool) -> Result<()>,
) -> Result<()> {
    let slots: Vec<_> = players.keys().cloned().collect();
    for (index, slot) in slots.iter().enumerate() {
        if let Err(error) = apply(players.get_mut(slot).expect("existing player"), next) {
            for changed_slot in &slots[..=index] {
                if let Err(rollback_error) = apply(
                    players.get_mut(changed_slot).expect("existing player"),
                    previous,
                ) {
                    // A player that cannot recover must not remain active with a different pause state.
                    tracing::warn!(
                        "Removing player '{}' after pause rollback failed: {}",
                        changed_slot,
                        rollback_error
                    );
                    players.remove(changed_slot);
                }
            }
            return Err(error);
        }
    }
    Ok(())
}

fn ensure_raised_workerw_bottom(desktop_parent: DesktopParent) -> Result<()> {
    let Some(workerw) = desktop_parent.workerw else {
        return Err(anyhow!(
            "Raised desktop parent is missing WorkerW for wallpaper z-order"
        ));
    };

    unsafe {
        if let Err(error) = SetWindowPos(
            HWND(workerw as *mut _),
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        ) {
            tracing::warn!(
                "Failed to keep raised desktop WorkerW behind wallpaper: {}",
                error
            );
        }
    }

    Ok(())
}

fn monitor_by_id(monitor_id: &str) -> Result<Monitor> {
    MonitorManager::new()?
        .get_monitors()
        .iter()
        .find(|monitor| monitor.id == monitor_id)
        .cloned()
        .ok_or_else(|| anyhow!("Monitor '{}' was not found", monitor_id))
}

fn placements_for_monitors(monitors: &[Monitor]) -> Result<Vec<PlayerPlacement>> {
    if monitors.is_empty() {
        return Err(anyhow!("No monitors were detected"));
    }

    Ok(monitors
        .iter()
        .cloned()
        .map(placement_for_monitor)
        .collect())
}

fn placement_for_monitor(monitor: Monitor) -> PlayerPlacement {
    PlayerPlacement {
        slot: monitor.id,
        x: monitor.x,
        y: monitor.y,
        width: monitor.width,
        height: monitor.height,
    }
}

impl Drop for WallpaperEngine {
    fn drop(&mut self) {
        tracing::debug!("Dropping wallpaper engine");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, x: i32, y: i32, width: i32, height: i32) -> Monitor {
        Monitor {
            id: id.to_string(),
            name: id.to_string(),
            x,
            y,
            width,
            height,
            is_primary: x == 0 && y == 0,
        }
    }

    #[test]
    fn all_monitor_placement_uses_one_slot_per_monitor() {
        let monitors = vec![
            monitor("\\\\.\\DISPLAY1", 0, 0, 1920, 1080),
            monitor("\\\\.\\DISPLAY2", 1920, 0, 1920, 1080),
        ];

        let placements = placements_for_monitors(&monitors).expect("monitors should map");

        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].slot, "\\\\.\\DISPLAY1");
        assert_eq!((placements[0].x, placements[0].y), (0, 0));
        assert_eq!((placements[0].width, placements[0].height), (1920, 1080));
        assert_eq!(placements[1].slot, "\\\\.\\DISPLAY2");
        assert_eq!((placements[1].x, placements[1].y), (1920, 0));
        assert_eq!((placements[1].width, placements[1].height), (1920, 1080));
    }

    #[test]
    fn all_monitor_placement_preserves_negative_monitor_origins() {
        let monitors = vec![
            monitor("\\\\.\\DISPLAY1", 0, 0, 2560, 1440),
            monitor("\\\\.\\DISPLAY2", -1920, 180, 1920, 1080),
        ];

        let placements = placements_for_monitors(&monitors).expect("monitors should map");

        assert_eq!(placements[1].slot, "\\\\.\\DISPLAY2");
        assert_eq!((placements[1].x, placements[1].y), (-1920, 180));
        assert_eq!((placements[1].width, placements[1].height), (1920, 1080));
    }

    #[test]
    fn all_monitor_placement_rejects_empty_monitor_list() {
        let error = placements_for_monitors(&[])
            .err()
            .expect("empty monitor list should fail");

        assert!(error.to_string().contains("No monitors were detected"));
    }

    #[test]
    fn playback_values_report_current_values_and_mixed_displays() {
        let mut engine = WallpaperEngine::new().unwrap();
        assert_eq!(engine.playback_values(), (None, None));
        engine.playback_state.insert(
            "A".into(),
            SlotPlaybackState {
                speed: 1.5,
                fit_mode: FitMode::Fill,
            },
        );
        assert_eq!(engine.playback_values(), (Some(1.5), Some(FitMode::Fill)));
        engine.playback_state.insert(
            "B".into(),
            SlotPlaybackState {
                speed: 2.0,
                fit_mode: FitMode::Fill,
            },
        );
        assert_eq!(engine.playback_values(), (None, Some(FitMode::Fill)));
        engine.playback_state.get_mut("B").unwrap().fit_mode = FitMode::Fit;
        assert_eq!(engine.playback_values(), (None, None));
    }

    #[test]
    fn failed_preparation_keeps_all_existing_players() {
        let original = HashMap::from([("A".into(), 1), ("B".into(), 2)]);
        let mut players = original.clone();
        let result = replace_players(&mut players, false, || {
            Err(anyhow!("Invalid monitor or startup failed"))
        });
        assert!(result.is_err());
        assert_eq!(players, original);
    }

    #[test]
    fn failed_second_monitor_preparation_keeps_every_existing_player() {
        let mut players = HashMap::from([("A".into(), 1), ("B".into(), 2)]);
        let original = players.clone();
        let mut prepared_count = 0;
        let result = replace_players(&mut players, true, || {
            let mut prepared = Vec::new();
            for monitor in ["A", "B"] {
                if monitor == "B" {
                    return Err(anyhow!("Second monitor failed to start"));
                }
                prepared.push((monitor.to_string(), 3));
                prepared_count += 1;
            }
            Ok(prepared)
        });
        assert!(result.is_err());
        assert_eq!(prepared_count, 1);
        assert_eq!(players, original);
    }

    #[test]
    fn all_display_replacement_retires_disconnected_slots_after_success() {
        let mut players =
            HashMap::from([("A".into(), 1), ("B".into(), 2), ("disconnected".into(), 3)]);
        let (_, retired) = replace_players(&mut players, true, || {
            Ok(vec![("A".into(), 4), ("B".into(), 4)])
        })
        .unwrap();
        assert_eq!(retired.len(), 3);
        assert_eq!(players, HashMap::from([("A".into(), 4), ("B".into(), 4)]));
    }

    #[test]
    fn invalid_monitor_does_not_replace_existing_player() {
        let mut players = HashMap::from([(ALL_MONITORS_SLOT.into(), 1)]);
        let result = replace_players(&mut players, false, || {
            let placement = WallpaperEngine::prepare_target(WallpaperTarget::Monitor(
                "nonexistent-wallscape-test-monitor".into(),
            ))?;
            Ok(vec![(placement.into_iter().next().unwrap().slot, 2)])
        });
        assert!(result.is_err());
        assert_eq!(players, HashMap::from([(ALL_MONITORS_SLOT.into(), 1)]));
    }

    #[test]
    fn replacement_retires_only_conflicting_slots_after_startup() {
        let mut players = HashMap::from([("A".into(), 1), ("B".into(), 2)]);
        let (_, retired) =
            replace_players(&mut players, false, || Ok(vec![("A".into(), 3)])).unwrap();
        assert_eq!(retired, vec![("A".into(), 1)]);
        assert_eq!(players, HashMap::from([("A".into(), 3), ("B".into(), 2)]));
        let (_, retired) = replace_players(&mut players, true, || {
            Ok(vec![("A".into(), 4), ("B".into(), 4)])
        })
        .unwrap();
        assert_eq!(retired.len(), 2);
        assert_eq!(players, HashMap::from([("A".into(), 4), ("B".into(), 4)]));
        let (_, retired) =
            replace_players(&mut players, false, || Ok(vec![("B".into(), 5)])).unwrap();
        assert_eq!(retired, vec![("B".into(), 4)]);
        assert_eq!(players, HashMap::from([("A".into(), 4), ("B".into(), 5)]));
    }

    #[test]
    fn pause_failure_rolls_back_players_already_updated() {
        let mut players = HashMap::from([("A".into(), false), ("B".into(), false)]);
        let mut calls = 0;
        let result = update_players_pause(&mut players, false, true, |paused, next| {
            calls += 1;
            if calls == 2 {
                return Err(anyhow!("Pause rejected"));
            }
            *paused = next;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(players.len(), 2);
        assert!(players.values().all(|paused| !paused));
    }

    #[test]
    fn rollback_failure_removes_player_with_unknown_pause_state() {
        let mut players = HashMap::from([("A".into(), false), ("B".into(), false)]);
        let mut calls = 0;
        let result = update_players_pause(&mut players, false, true, |paused, next| {
            calls += 1;
            if calls == 2 || calls == 3 {
                return Err(anyhow!("Player unavailable"));
            }
            *paused = next;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(players.len(), 1);
        assert!(players.values().all(|paused| !paused));
    }

    #[test]
    fn stopping_retired_players_succeeds_even_when_mpv_rejects_stop() {
        let mut engine = WallpaperEngine::new().unwrap();
        // These players have no mpv instance, so their explicit stop commands fail.
        // Retirement must still report success after their resources are dropped.
        let path = std::env::current_exe().unwrap();
        for slot in ["A", "B"] {
            engine
                .video_players
                .insert(slot.into(), VideoPlayer::new(&path).unwrap());
            engine
                .playback_state
                .insert(slot.into(), SlotPlaybackState::default());
        }
        engine
            .stop_target(WallpaperTarget::Monitor("A".into()))
            .unwrap();
        assert!(engine.has_active_players());
        assert!(!engine.playback_state.contains_key("A"));
        engine.stop().unwrap();
        assert!(!engine.has_active_players());
        assert!(engine.playback_state.is_empty());
    }
}
