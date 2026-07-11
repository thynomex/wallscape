use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

use super::engine::PlaybackOptions;
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
    /// Effective pause state of the active wallpaper (manual OR automatic).
    paused: Arc<AtomicBool>,
    /// Pause state requested by the user from UI/tray controls.
    manual_paused: Arc<AtomicBool>,
    /// Pause state requested by auto-pause rules.
    auto_paused: Arc<AtomicBool>,
    /// Whether a video wallpaper is currently running.
    active: Arc<AtomicBool>,
}

enum RuntimeCommand {
    SetVideoWallpaper {
        video_path: String,
        target: WallpaperTarget,
        options: PlaybackOptions,
        response: Sender<Result<String>>,
    },
    SetPaused {
        paused: bool,
        response: Sender<Result<()>>,
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
        let (commands, receiver) = mpsc::channel();

        thread::Builder::new()
            .name("wallscape-wallpaper-runtime".to_string())
            .spawn(move || run_wallpaper_thread(receiver))
            .expect("failed to spawn wallpaper runtime thread");

        Self {
            commands,
            paused: Arc::new(AtomicBool::new(false)),
            manual_paused: Arc::new(AtomicBool::new(false)),
            auto_paused: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
        }
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

        let outcome = result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?;

        if outcome.is_ok() {
            self.active.store(true, Ordering::SeqCst);
            self.paused.store(false, Ordering::SeqCst);
            self.manual_paused.store(false, Ordering::SeqCst);
            self.auto_paused.store(false, Ordering::SeqCst);
        }

        outcome
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
        let previous_manual = self.manual_paused.load(Ordering::SeqCst);
        let previous_auto = self.auto_paused.load(Ordering::SeqCst);
        let previous_effective = previous_manual || previous_auto;

        match source {
            PauseSource::Manual => self.manual_paused.store(paused, Ordering::SeqCst),
            PauseSource::Automatic => self.auto_paused.store(paused, Ordering::SeqCst),
        }

        let next_manual = self.manual_paused.load(Ordering::SeqCst);
        let next_auto = self.auto_paused.load(Ordering::SeqCst);
        let next_effective = next_manual || next_auto;

        if previous_effective == next_effective {
            self.paused.store(next_effective, Ordering::SeqCst);
            return Ok(());
        }

        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::SetPaused {
                paused: next_effective,
                response,
            })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        let outcome = result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?;

        if outcome.is_ok() {
            self.paused.store(next_effective, Ordering::SeqCst);
        } else {
            self.manual_paused.store(previous_manual, Ordering::SeqCst);
            self.auto_paused.store(previous_auto, Ordering::SeqCst);
            self.paused.store(previous_effective, Ordering::SeqCst);
        }

        outcome
    }

    /// Flip the pause state, returning the new value. Caller should ensure a
    /// wallpaper is active ([`Self::is_active`]) for the result to be meaningful.
    pub fn toggle_paused(&self) -> Result<bool> {
        let manual = self.manual_paused.load(Ordering::SeqCst);
        let automatic = self.auto_paused.load(Ordering::SeqCst);
        let next = if automatic && !manual { false } else { !manual };
        self.set_paused(next)?;
        Ok(self.is_paused())
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn is_auto_paused(&self) -> bool {
        self.auto_paused.load(Ordering::SeqCst)
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn stop(&self) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::Stop { response })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        let outcome = result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?;

        if outcome.is_ok() {
            self.active.store(false, Ordering::SeqCst);
            self.paused.store(false, Ordering::SeqCst);
            self.manual_paused.store(false, Ordering::SeqCst);
            self.auto_paused.store(false, Ordering::SeqCst);
        }

        outcome
    }

    pub fn stop_target(&self, target: WallpaperTarget) -> Result<()> {
        let (response, result) = mpsc::channel();

        self.commands
            .send(RuntimeCommand::StopTarget { target, response })
            .map_err(|_| anyhow!("Wallpaper runtime is not available"))?;

        let outcome = result
            .recv()
            .map_err(|_| anyhow!("Wallpaper runtime stopped before responding"))?;

        if let Ok(has_active_players) = outcome.as_ref() {
            self.active.store(*has_active_players, Ordering::SeqCst);

            if !has_active_players {
                self.paused.store(false, Ordering::SeqCst);
                self.manual_paused.store(false, Ordering::SeqCst);
                self.auto_paused.store(false, Ordering::SeqCst);
            }
        }

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

fn run_wallpaper_thread(receiver: Receiver<RuntimeCommand>) {
    let mut engine: Option<WallpaperEngine> = None;

    loop {
        pump_window_messages();

        let timeout = if engine.is_some() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(100)
        };

        match receiver.recv_timeout(timeout) {
            Ok(RuntimeCommand::SetVideoWallpaper {
                video_path,
                target,
                options,
                response,
            }) => {
                let result = set_video_wallpaper(&mut engine, &video_path, target, options);
                let _ = response.send(result.map(|_| format!("Wallpaper set to: {}", video_path)));
            }
            Ok(RuntimeCommand::SetPaused { paused, response }) => {
                let result = match engine.as_mut() {
                    Some(engine) => engine.set_paused(paused),
                    None => Ok(()),
                };
                let _ = response.send(result);
            }
            Ok(RuntimeCommand::SetSpeed {
                speed,
                slot,
                response,
            }) => {
                let result = match engine.as_mut() {
                    Some(engine) => engine.set_playback_speed(speed, slot.as_deref()),
                    None => Err(anyhow!("No active wallpaper engine")),
                };
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
                let _ = response.send(result);
            }
            Ok(RuntimeCommand::Stop { response }) => {
                let result = match engine.as_mut() {
                    Some(engine) => engine.stop(),
                    None => Ok(()),
                };

                if result.is_ok() {
                    engine = None;
                }

                let _ = response.send(result);
            }
            Ok(RuntimeCommand::StopTarget { target, response }) => {
                let mut clear_engine = false;
                let result = match engine.as_mut() {
                    Some(active_engine) => {
                        let stopped = active_engine.stop_target(target).map(|_| {
                            let has_active_players = active_engine.has_active_players();
                            if !has_active_players {
                                let _ = active_engine.stop();
                            }
                            has_active_players
                        });

                        if matches!(stopped, Ok(false)) {
                            clear_engine = true;
                        }

                        stopped
                    }
                    None => Ok(false),
                };

                if clear_engine {
                    engine = None;
                }

                let _ = response.send(result);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                if let Some(mut engine) = engine.take() {
                    let _ = engine.stop();
                }
                break;
            }
        }
    }
}

fn set_video_wallpaper(
    engine: &mut Option<WallpaperEngine>,
    video_path: &str,
    target: WallpaperTarget,
    options: PlaybackOptions,
) -> Result<()> {
    if engine.is_none() {
        let mut next_engine = WallpaperEngine::new()?;
        next_engine.initialize()?;
        *engine = Some(next_engine);
    }

    engine
        .as_mut()
        .expect("engine initialized above")
        .set_video_wallpaper_with_options(video_path, target, options)?;

    Ok(())
}

fn pump_window_messages() {
    unsafe {
        let mut message = MSG::default();

        while PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
