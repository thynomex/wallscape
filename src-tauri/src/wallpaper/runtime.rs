use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

use super::engine::{FitMode, PlaybackOptions};
use super::WallpaperEngine;

#[derive(Debug, Clone)]
pub enum WallpaperTarget {
    All,
    Monitor(String),
}

/// Runs the wallpaper engine on a dedicated Win32 thread.
#[derive(Clone)]
pub struct WallpaperRuntime {
    commands: Sender<RuntimeCommand>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RuntimeSnapshot {
    pub active: bool,
    pub paused: bool,
    pub auto_paused: bool,
    pub playback_speed: Option<f64>,
    pub fit_mode: Option<FitMode>,
}

#[derive(Default, Clone, Copy)]
struct PauseIntent {
    manual: bool,
    automatic: bool,
}

impl PauseIntent {
    fn effective(self) -> bool {
        self.manual || self.automatic
    }

    fn updated(mut self, source: PauseSource, paused: bool) -> Self {
        match source {
            PauseSource::Manual => self.manual = paused,
            PauseSource::Automatic => self.automatic = paused,
        }
        self
    }
}

enum RuntimeCommand {
    SetVideoWallpaper {
        video_path: String,
        target: WallpaperTarget,
        options: PlaybackOptions,
        response: Sender<Result<String>>,
    },
    SetPaused {
        source: PauseSource,
        paused: bool,
        response: Sender<Result<()>>,
    },
    TogglePaused {
        response: Sender<Result<bool>>,
    },
    SetSpeed {
        speed: f64,
        slot: Option<String>,
        response: Sender<Result<()>>,
    },
    SetFitMode {
        mode: String,
        slot: Option<String>,
        response: Sender<Result<()>>,
    },
    Stop {
        response: Sender<Result<()>>,
    },
    StopTarget {
        target: WallpaperTarget,
        response: Sender<Result<bool>>,
    },
}

impl WallpaperRuntime {
    pub fn spawn() -> Self {
        Self::spawn_with_engine(|| {
            let mut engine = WallpaperEngine::new()?;
            engine.initialize()?;
            Ok(engine)
        })
    }

    fn spawn_with_engine<E: RuntimeEngine + 'static>(
        create_engine: impl FnMut() -> Result<E> + Send + 'static,
    ) -> Self {
        let (commands, receiver) = mpsc::channel();
        let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));
        let thread_snapshot = Arc::clone(&snapshot);

        thread::Builder::new()
            .name("wallscape-wallpaper-runtime".to_string())
            .spawn(move || {
                run_wallpaper_thread(
                    receiver,
                    thread_snapshot,
                    create_engine,
                    pump_window_messages,
                )
            })
            .expect("failed to spawn wallpaper runtime thread");

        Self { commands, snapshot }
    }

    pub fn set_video_wallpaper_for_target_with_options(
        &self,
        video_path: String,
        target: WallpaperTarget,
        options: PlaybackOptions,
    ) -> Result<String> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::SetVideoWallpaper {
                video_path,
                target,
                options,
                response,
            })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?
    }

    /// Pause or resume the active video wallpaper.
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        self.set_pause_source(PauseSource::Manual, paused)
    }

    /// Pause or resume because of automatic rules without changing manual intent.
    pub fn set_auto_paused(&self, paused: bool) -> Result<()> {
        self.set_pause_source(PauseSource::Automatic, paused)
    }

    fn set_pause_source(&self, source: PauseSource, paused: bool) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::SetPaused {
                source,
                paused,
                response,
            })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?
    }

    /// Flip the pause state, returning the new value. Caller should ensure a
    /// wallpaper is active ([`Self::is_active`]) for the result to be meaningful.
    pub fn toggle_paused(&self) -> Result<bool> {
        let (response, result) = mpsc::channel();
        self.commands
            .send(RuntimeCommand::TogglePaused { response })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;
        result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        *self.snapshot.read()
    }

    pub fn is_auto_paused(&self) -> bool {
        self.snapshot().auto_paused
    }

    pub fn is_active(&self) -> bool {
        self.snapshot().active
    }

    pub fn stop(&self) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::Stop { response })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?
    }

    pub fn stop_target(&self, target: WallpaperTarget) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::StopTarget { target, response })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        let outcome = result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?;

        outcome.map(|_| ())
    }

    /// Set playback speed for active wallpapers
    pub fn set_speed(&self, speed: f64, slot: Option<String>) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::SetSpeed {
                speed,
                slot,
                response,
            })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?
    }

    /// Set fit mode for active wallpapers
    pub fn set_fit_mode(&self, mode: String, slot: Option<String>) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::SetFitMode {
                mode,
                slot,
                response,
            })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?
    }
}

enum PauseSource {
    Manual,
    Automatic,
}

trait RuntimeEngine {
    fn set_video(
        &mut self,
        path: &str,
        target: WallpaperTarget,
        options: PlaybackOptions,
    ) -> Result<()>;
    fn set_paused(&mut self, paused: bool) -> Result<()>;
    fn set_speed(&mut self, speed: f64, slot: Option<&str>) -> Result<()>;
    fn set_fit_mode(&mut self, mode: super::engine::FitMode, slot: Option<&str>) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn stop_target(&mut self, target: WallpaperTarget) -> Result<()>;
    fn has_active_players(&self) -> bool;
    fn playback_values(&self) -> (Option<f64>, Option<FitMode>);
    fn service_video_loops(&mut self) {}
}

impl RuntimeEngine for WallpaperEngine {
    fn set_video(
        &mut self,
        path: &str,
        target: WallpaperTarget,
        options: PlaybackOptions,
    ) -> Result<()> {
        self.set_video_wallpaper_with_options(path, target, options)
    }

    fn set_paused(&mut self, paused: bool) -> Result<()> {
        WallpaperEngine::set_paused(self, paused)
    }

    fn set_speed(&mut self, speed: f64, slot: Option<&str>) -> Result<()> {
        self.set_playback_speed(speed, slot)
    }

    fn set_fit_mode(&mut self, mode: super::engine::FitMode, slot: Option<&str>) -> Result<()> {
        WallpaperEngine::set_fit_mode(self, mode, slot)
    }

    fn stop(&mut self) -> Result<()> {
        WallpaperEngine::stop(self)
    }

    fn stop_target(&mut self, target: WallpaperTarget) -> Result<()> {
        WallpaperEngine::stop_target(self, target)
    }

    fn has_active_players(&self) -> bool {
        WallpaperEngine::has_active_players(self)
    }

    fn playback_values(&self) -> (Option<f64>, Option<FitMode>) {
        WallpaperEngine::playback_values(self)
    }

    fn service_video_loops(&mut self) {
        WallpaperEngine::service_video_loops(self);
    }
}

fn publish_snapshot<E: RuntimeEngine>(
    snapshot: &RwLock<RuntimeSnapshot>,
    engine: &Option<E>,
    intent: PauseIntent,
) {
    let active = engine
        .as_ref()
        .is_some_and(RuntimeEngine::has_active_players);
    let (playback_speed, fit_mode) = engine
        .as_ref()
        .filter(|_| active)
        .map(RuntimeEngine::playback_values)
        .unwrap_or_default();
    *snapshot.write() = RuntimeSnapshot {
        active,
        paused: active && intent.effective(),
        auto_paused: active && intent.automatic,
        playback_speed,
        fit_mode,
    };
}

fn apply_pause_intent<E: RuntimeEngine>(
    engine: &mut Option<E>,
    intent: &mut PauseIntent,
    next: PauseIntent,
) -> Result<()> {
    if intent.effective() != next.effective() {
        if let Some(engine) = engine.as_mut() {
            engine.set_paused(next.effective())?;
        }
    }
    *intent = next;
    Ok(())
}

fn run_wallpaper_thread<E: RuntimeEngine>(
    receiver: Receiver<RuntimeCommand>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    mut create_engine: impl FnMut() -> Result<E>,
    mut pump_messages: impl FnMut(),
) {
    let mut engine: Option<E> = None;
    let mut intent = PauseIntent::default();

    loop {
        pump_messages();
        if let Some(engine) = engine.as_mut() {
            engine.service_video_loops();
        }

        let command = if engine.is_some() {
            receiver.recv_timeout(Duration::from_millis(16))
        } else {
            // With no engine, owned player windows have been destroyed. Wait for
            // work without polling; paused engines still need the message pump.
            receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
        };

        match command {
            Ok(RuntimeCommand::SetVideoWallpaper {
                video_path,
                target,
                options,
                response,
            }) => {
                let result = set_video_wallpaper(
                    &mut engine,
                    &video_path,
                    target,
                    options,
                    intent,
                    &mut create_engine,
                );
                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result.map(|_| format!("Wallpaper set to: {}", video_path)));
            }
            Ok(RuntimeCommand::SetPaused {
                source,
                paused,
                response,
            }) => {
                let next = intent.updated(source, paused);
                let result = apply_pause_intent(&mut engine, &mut intent, next);
                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result);
            }
            Ok(RuntimeCommand::TogglePaused { response }) => {
                let next_manual = if intent.automatic && !intent.manual {
                    false
                } else {
                    !intent.manual
                };
                let next = intent.updated(PauseSource::Manual, next_manual);
                let result = apply_pause_intent(&mut engine, &mut intent, next);
                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result.map(|_| snapshot.read().paused));
            }
            Ok(RuntimeCommand::SetSpeed {
                speed,
                slot,
                response,
            }) => {
                let result = match engine.as_mut() {
                    Some(engine) => engine.set_speed(speed, slot.as_deref()),
                    None => Err(anyhow!("No active wallpaper engine")),
                };
                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result);
            }
            Ok(RuntimeCommand::SetFitMode {
                mode,
                slot,
                response,
            }) => {
                let result = match engine.as_mut() {
                    Some(engine) => {
                        use super::engine::FitMode;
                        let fit_mode = match mode.as_str() {
                            "fit" => FitMode::Fit,
                            "fill" => FitMode::Fill,
                            "stretch" => FitMode::Stretch,
                            _ => {
                                let _ = response.send(Err(anyhow!(
                                    "Invalid fit mode: {}. Valid modes: fit, fill, stretch",
                                    mode
                                )));
                                continue;
                            }
                        };
                        engine.set_fit_mode(fit_mode, slot.as_deref())
                    }
                    None => Err(anyhow!("No active wallpaper engine")),
                };
                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result);
            }
            Ok(RuntimeCommand::Stop { response }) => {
                let result = match engine.as_mut() {
                    Some(engine) => engine.stop(),
                    None => Ok(()),
                };

                if !engine
                    .as_ref()
                    .is_some_and(RuntimeEngine::has_active_players)
                {
                    engine = None;
                    intent = PauseIntent::default();
                }

                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result);
            }
            Ok(RuntimeCommand::StopTarget { target, response }) => {
                let result = match engine.as_mut() {
                    Some(active_engine) => active_engine.stop_target(target),
                    None => Ok(()),
                };
                let active = engine
                    .as_ref()
                    .is_some_and(RuntimeEngine::has_active_players);
                if !active {
                    engine = None;
                    intent = PauseIntent::default();
                }

                publish_snapshot(&snapshot, &engine, intent);
                let _ = response.send(result.map(|_| active));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                if let Some(mut engine) = engine.take() {
                    let _ = engine.stop();
                }
                *snapshot.write() = RuntimeSnapshot::default();
                break;
            }
        }
    }
}

fn set_video_wallpaper<E: RuntimeEngine>(
    engine: &mut Option<E>,
    video_path: &str,
    target: WallpaperTarget,
    options: PlaybackOptions,
    intent: PauseIntent,
    create_engine: &mut impl FnMut() -> Result<E>,
) -> Result<()> {
    if engine.is_none() {
        let mut next_engine = create_engine()?;
        next_engine.set_paused(intent.effective())?;
        next_engine.set_video(video_path, target, options)?;
        *engine = Some(next_engine);
        return Ok(());
    }

    engine
        .as_mut()
        .expect("engine initialized above")
        .set_video(video_path, target, options)?;

    Ok(())
}

fn pump_window_messages() {
    unsafe {
        let mut message = MSG::default();

        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, IsWindow, SendMessageTimeoutW, HWND_MESSAGE,
        SMTO_ABORTIFHUNG, WM_GETTEXTLENGTH,
    };

    // A message-only window cannot appear on, or change, the user's desktop.
    struct MessageOnlyWindow(HWND);

    impl MessageOnlyWindow {
        fn new() -> Result<Self> {
            unsafe {
                Ok(Self(CreateWindowExW(
                    Default::default(),
                    w!("STATIC"),
                    w!("runtime-probe"),
                    Default::default(),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    None,
                    None,
                )?))
            }
        }
    }

    impl Drop for MessageOnlyWindow {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    #[derive(Default)]
    struct FakePlayback {
        players: Mutex<HashMap<String, bool>>,
        options: Mutex<HashMap<String, PlaybackOptions>>,
        reject_pause: AtomicBool,
    }

    struct FakeEngine {
        playback: Arc<FakePlayback>,
        paused: bool,
        window: Option<MessageOnlyWindow>,
    }

    impl RuntimeEngine for FakeEngine {
        fn set_video(
            &mut self,
            path: &str,
            target: WallpaperTarget,
            options: PlaybackOptions,
        ) -> Result<()> {
            if path == "invalid" {
                return Err(anyhow!("Unable to start replacement"));
            }
            let mut players = self.playback.players.lock();
            let mut playback_options = self.playback.options.lock();
            match target {
                WallpaperTarget::All => {
                    players.clear();
                    players.insert("all".into(), self.paused);
                    playback_options.clear();
                    playback_options.insert("all".into(), options);
                }
                WallpaperTarget::Monitor(id) => {
                    players.remove("all");
                    playback_options.remove("all");
                    playback_options.insert(id.clone(), options);
                    players.insert(id, self.paused);
                }
            }
            Ok(())
        }

        fn set_paused(&mut self, paused: bool) -> Result<()> {
            if self.playback.reject_pause.load(Ordering::SeqCst) {
                return Err(anyhow!("Pause failed"));
            }
            self.paused = paused;
            self.playback
                .players
                .lock()
                .values_mut()
                .for_each(|value| *value = paused);
            Ok(())
        }

        fn set_speed(&mut self, speed: f64, slot: Option<&str>) -> Result<()> {
            for (id, options) in self.playback.options.lock().iter_mut() {
                if slot.is_none_or(|slot| slot == id) {
                    options.speed = speed;
                }
            }
            Ok(())
        }
        fn set_fit_mode(&mut self, mode: FitMode, slot: Option<&str>) -> Result<()> {
            for (id, options) in self.playback.options.lock().iter_mut() {
                if slot.is_none_or(|slot| slot == id) {
                    options.fit_mode = mode;
                }
            }
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            self.playback.players.lock().clear();
            self.playback.options.lock().clear();
            self.window.take();
            Ok(())
        }

        fn stop_target(&mut self, target: WallpaperTarget) -> Result<()> {
            match target {
                WallpaperTarget::All => self.stop(),
                WallpaperTarget::Monitor(id) => {
                    let mut players = self.playback.players.lock();
                    players.remove("all");
                    players.remove(&id);
                    let mut options = self.playback.options.lock();
                    options.remove("all");
                    options.remove(&id);
                    Ok(())
                }
            }
        }

        fn has_active_players(&self) -> bool {
            !self.playback.players.lock().is_empty()
        }

        fn playback_values(&self) -> (Option<f64>, Option<FitMode>) {
            let options = self.playback.options.lock();
            let Some(first) = options.values().next() else {
                return (None, None);
            };
            (
                options
                    .values()
                    .all(|value| value.speed == first.speed)
                    .then_some(first.speed),
                options
                    .values()
                    .all(|value| value.fit_mode == first.fit_mode)
                    .then_some(first.fit_mode),
            )
        }
    }

    fn fake_runtime() -> (WallpaperRuntime, Arc<FakePlayback>) {
        let playback = Arc::new(FakePlayback::default());
        let shared = playback.clone();
        let runtime = WallpaperRuntime::spawn_with_engine(move || {
            Ok(FakeEngine {
                playback: shared.clone(),
                paused: false,
                window: None,
            })
        });
        (runtime, playback)
    }

    fn apply(runtime: &WallpaperRuntime, monitor: &str, path: &str) -> Result<String> {
        runtime.set_video_wallpaper_for_target_with_options(
            path.into(),
            WallpaperTarget::Monitor(monitor.into()),
            PlaybackOptions::default(),
        )
    }

    #[test]
    fn snapshot_tracks_applied_playback_options_and_changes() {
        let (runtime, _) = fake_runtime();
        runtime
            .set_video_wallpaper_for_target_with_options(
                "valid".into(),
                WallpaperTarget::All,
                PlaybackOptions {
                    speed: 1.5,
                    fit_mode: FitMode::Fill,
                    fps_limit: None,
                },
            )
            .unwrap();
        assert_eq!(runtime.snapshot().playback_speed, Some(1.5));
        assert_eq!(runtime.snapshot().fit_mode, Some(FitMode::Fill));
        runtime.set_speed(2.0, None).unwrap();
        runtime.set_fit_mode("stretch".into(), None).unwrap();
        assert_eq!(runtime.snapshot().playback_speed, Some(2.0));
        assert_eq!(runtime.snapshot().fit_mode, Some(FitMode::Stretch));
        apply(&runtime, "A", "valid").unwrap();
        assert_eq!(runtime.snapshot().playback_speed, Some(1.0));
        assert_eq!(runtime.snapshot().fit_mode, Some(FitMode::Fit));
        runtime.stop().unwrap();
        assert_eq!(runtime.snapshot().playback_speed, None);
        assert_eq!(runtime.snapshot().fit_mode, None);
    }

    struct ObservedRuntime {
        runtime: WallpaperRuntime,
        pumps: Receiver<()>,
        finished: Receiver<()>,
        thread: thread::JoinHandle<()>,
    }

    impl ObservedRuntime {
        fn new(create_engine: impl FnMut() -> Result<FakeEngine> + Send + 'static) -> Self {
            let (commands, receiver) = mpsc::channel();
            let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));
            let thread_snapshot = snapshot.clone();
            let (pump, pumps) = mpsc::channel();
            let (finish, finished) = mpsc::channel();
            let thread = thread::spawn(move || {
                run_wallpaper_thread(receiver, thread_snapshot, create_engine, || {
                    pump_window_messages();
                    let _ = pump.send(());
                });
                let _ = finish.send(());
            });
            Self {
                runtime: WallpaperRuntime { commands, snapshot },
                pumps,
                finished,
                thread,
            }
        }

        fn assert_idle(&self) {
            // A command response can arrive before the final message drain.
            self.pumps.try_iter().for_each(drop);
            let _ = self.pumps.recv_timeout(Duration::from_millis(150));
            assert_eq!(
                self.pumps.recv_timeout(Duration::from_millis(150)),
                Err(RecvTimeoutError::Timeout),
                "an engine-free runtime must not keep polling"
            );
        }

        fn assert_pumping(&self) {
            self.pumps.try_iter().for_each(drop);
            for _ in 0..2 {
                self.pumps.recv_timeout(Duration::from_secs(1)).unwrap();
            }
        }

        fn disconnect(self) {
            let snapshot = self.runtime.snapshot.clone();
            drop(self.runtime);
            self.finished.recv_timeout(Duration::from_secs(1)).unwrap();
            self.thread.join().unwrap();
            assert_eq!(*snapshot.read(), RuntimeSnapshot::default());
        }
    }

    #[test]
    fn idle_runtime_blocks_until_a_command_or_disconnect() {
        let observed = ObservedRuntime::new(|| Err(anyhow!("Initialization failed")));
        observed.assert_idle();
        let (response, result) = mpsc::channel();
        observed
            .runtime
            .commands
            .send(RuntimeCommand::SetPaused {
                source: PauseSource::Manual,
                paused: true,
                response,
            })
            .unwrap();
        result
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert!(apply(&observed.runtime, "A", "valid").is_err());
        observed.assert_idle();
        observed.disconnect();
    }

    #[test]
    fn live_paused_and_empty_engines_keep_pumping_until_stopped() {
        let playback = Arc::new(FakePlayback::default());
        let shared = playback.clone();
        let observed = ObservedRuntime::new(move || {
            Ok(FakeEngine {
                playback: shared.clone(),
                paused: false,
                window: None,
            })
        });
        apply(&observed.runtime, "A", "valid").unwrap();
        observed.assert_pumping();
        observed.runtime.set_paused(true).unwrap();
        observed.assert_pumping();
        playback.players.lock().clear();
        observed.assert_pumping();
        observed.runtime.stop().unwrap();
        observed.assert_idle();
        // Starting another wallpaper wakes a runtime that returned to blocking.
        apply(&observed.runtime, "A", "valid").unwrap();
        observed.assert_pumping();
        observed.disconnect();
        assert!(playback.players.lock().is_empty());
    }

    #[test]
    fn message_only_window_is_pumped_when_paused_and_destroyed_before_idle() {
        let playback = Arc::new(FakePlayback::default());
        let (created, windows) = mpsc::channel();
        let observed = ObservedRuntime::new(move || {
            let window = MessageOnlyWindow::new()?;
            created.send(window.0 .0 as usize).unwrap();
            Ok(FakeEngine {
                playback: playback.clone(),
                paused: false,
                window: Some(window),
            })
        });

        // A failed first apply drops the temporary engine and its native window.
        assert!(apply(&observed.runtime, "A", "invalid").is_err());
        let failed_hwnd = HWND(windows.recv().unwrap() as *mut _);
        assert!(!unsafe { IsWindow(Some(failed_hwnd)) }.as_bool());
        observed.assert_idle();

        apply(&observed.runtime, "A", "valid").unwrap();
        let hwnd = HWND(windows.recv().unwrap() as *mut _);
        for paused in [false, true] {
            observed.runtime.set_paused(paused).unwrap();
            let mut result = 0;
            let delivered = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    WM_GETTEXTLENGTH,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    1000,
                    Some(&mut result),
                )
            };
            assert_ne!(delivered.0, 0, "the runtime must dispatch native messages");
            assert_eq!(result, "runtime-probe".len());
        }
        observed.runtime.stop_target(WallpaperTarget::All).unwrap();
        assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
        observed.assert_idle();

        apply(&observed.runtime, "A", "valid").unwrap();
        let hwnd = HWND(windows.recv().unwrap() as *mut _);
        observed.disconnect();
        assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
    }

    #[test]
    fn applying_another_monitor_preserves_pause_and_resume_reaches_both_players() {
        let (runtime, playback) = fake_runtime();
        apply(&runtime, "A", "valid").unwrap();
        runtime.set_paused(true).unwrap();
        apply(&runtime, "B", "valid").unwrap();
        assert_eq!(
            runtime.snapshot(),
            RuntimeSnapshot {
                active: true,
                paused: true,
                auto_paused: false,
                playback_speed: Some(1.0),
                fit_mode: Some(FitMode::Fit),
            }
        );
        assert_eq!(
            *playback.players.lock(),
            HashMap::from([("A".into(), true), ("B".into(), true)])
        );
        runtime.set_paused(false).unwrap();
        assert!(!runtime.snapshot().paused);
        assert!(playback.players.lock().values().all(|paused| !paused));
    }

    #[test]
    fn automatic_pause_survives_apply_and_manual_resume() {
        let (runtime, playback) = fake_runtime();
        apply(&runtime, "A", "valid").unwrap();
        runtime.set_auto_paused(true).unwrap();
        runtime.set_paused(true).unwrap();
        apply(&runtime, "B", "valid").unwrap();
        runtime.set_paused(false).unwrap();
        assert!(runtime.snapshot().auto_paused);
        assert!(playback.players.lock().values().all(|paused| *paused));
        runtime.set_auto_paused(false).unwrap();
        assert!(!runtime.snapshot().paused);
        assert!(playback.players.lock().values().all(|paused| !paused));
    }

    #[test]
    fn failed_replacement_and_failed_pause_keep_existing_status() {
        let (runtime, playback) = fake_runtime();
        assert!(apply(&runtime, "A", "invalid").is_err());
        assert_eq!(runtime.snapshot(), RuntimeSnapshot::default());
        apply(&runtime, "A", "valid").unwrap();
        playback.reject_pause.store(true, Ordering::SeqCst);
        assert!(runtime.set_paused(true).is_err());
        assert!(!runtime.snapshot().paused);
        playback.reject_pause.store(false, Ordering::SeqCst);
        runtime.set_paused(true).unwrap();
        let before = runtime.snapshot();
        assert!(apply(&runtime, "A", "invalid").is_err());
        assert_eq!(runtime.snapshot(), before);
        assert_eq!(
            *playback.players.lock(),
            HashMap::from([("A".into(), true)])
        );
    }

    #[test]
    fn concurrent_toggles_are_serialized_without_losing_intent() {
        let (runtime, playback) = fake_runtime();
        apply(&runtime, "A", "valid").unwrap();
        let threads: Vec<_> = (0..20)
            .map(|_| {
                let runtime = runtime.clone();
                thread::spawn(move || runtime.toggle_paused().unwrap())
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(!runtime.snapshot().paused);
        assert!(playback.players.lock().values().all(|paused| !paused));
    }

    #[test]
    fn stopping_one_monitor_keeps_pause_until_last_player_stops() {
        let (runtime, _) = fake_runtime();
        apply(&runtime, "A", "valid").unwrap();
        apply(&runtime, "B", "valid").unwrap();
        runtime.set_paused(true).unwrap();
        runtime
            .stop_target(WallpaperTarget::Monitor("A".into()))
            .unwrap();
        assert!(runtime.snapshot().paused);
        runtime
            .stop_target(WallpaperTarget::Monitor("B".into()))
            .unwrap();
        assert_eq!(runtime.snapshot(), RuntimeSnapshot::default());
        apply(&runtime, "C", "valid").unwrap();
        assert!(!runtime.snapshot().paused);
    }
}
