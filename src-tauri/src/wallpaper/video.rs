use anyhow::{anyhow, Result};
use libmpv2::events::Event;
use libmpv2::{mpv_error, Error as MpvError, Mpv};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::core::Error as WindowsError;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    GetLastError, SetLastError, COLORREF, ERROR_SUCCESS, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassExW, SetLayeredWindowAttributes,
    SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, HWND_BOTTOM, LWA_ALPHA, SET_WINDOW_POS_FLAGS,
    SWP_NOZORDER, SW_SHOW, WINDOW_EX_STYLE, WM_ERASEBKGND, WNDCLASSEXW, WS_CHILD, WS_CLIPSIBLINGS,
    WS_DISABLED, WS_EX_LAYERED,
};

const ERROR_CLASS_ALREADY_EXISTS_CODE: i32 = 1410;

/// Video player using hardware acceleration via libmpv
pub struct VideoPlayer {
    video_path: String,
    hwnd: Option<HWND>,
    mpv: Option<Mpv>,
    playing: bool,
    speed: f64,
    fit_mode: String,
    fps_cap: Option<u32>,
}

impl VideoPlayer {
    pub fn new(video_path: impl AsRef<Path>) -> Result<Self> {
        let video_path = normalize_video_path(video_path.as_ref())?;

        tracing::info!("Creating video player for: {}", video_path.display());

        Ok(Self {
            video_path: video_path.to_string_lossy().to_string(),
            hwnd: None,
            mpv: None,
            playing: false,
            speed: 1.0,
            fit_mode: "fit".to_string(),
            fps_cap: None,
        })
    }

    /// Create a window for video playback
    pub fn create_window(&mut self, parent: Option<HWND>, layered_opaque: bool) -> Result<HWND> {
        tracing::info!("Creating video player window for: {}", self.video_path);

        unsafe {
            register_window_class()?;

            let class_name = encode_wide("WallscapeVideoPlayer");
            let window_name = encode_wide("Wallscape Video Player");

            let hinstance = GetModuleHandleW(PCWSTR::null())
                .map_err(|e| anyhow!("Failed to get module handle: {}", e))?;

            let ex_style = if layered_opaque {
                WS_EX_LAYERED
            } else {
                WINDOW_EX_STYLE(0)
            };
            let parent = parent.unwrap_or(HWND(std::ptr::null_mut()));
            let style = WS_CHILD | WS_CLIPSIBLINGS | WS_DISABLED;

            SetLastError(ERROR_SUCCESS);
            let hwnd = match CreateWindowExW(
                ex_style,
                PCWSTR::from_raw(class_name.as_ptr()),
                PCWSTR::from_raw(window_name.as_ptr()),
                style,
                0,
                0, // Position
                1920,
                1080, // Initial size (will be adjusted)
                parent,
                None, // No menu
                hinstance,
                None,
            ) {
                Ok(hwnd) => hwnd,
                Err(error) => {
                    return Err(describe_create_window_error(
                        error,
                        parent,
                        ex_style,
                        style,
                        layered_opaque,
                    ));
                }
            };

            if layered_opaque {
                if let Err(error) = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA) {
                    tracing::warn!("Failed to configure layered video window: {}", error);
                }
            }

            tracing::info!("Created video player window: {:?}", hwnd);

            self.hwnd = Some(hwnd);
            self.initialize_mpv(hwnd)?;

            Ok(hwnd)
        }
    }

    fn initialize_mpv(&mut self, hwnd: HWND) -> Result<()> {
        let wid = hwnd.0 as i64;
        let fit_mode = self.fit_mode.clone();
        let fps_cap = self.fps_cap;

        let mpv = Mpv::with_initializer(|init| {
            init.set_option("wid", wid)?;
            init.set_option("hwdec", "auto")?;
            init.set_option("loop-file", "inf")?;
            init.set_option("audio", "no")?;
            init.set_option("vo", "gpu")?;

            // Cheap render settings: wallpaper content doesn't justify mpv's
            // default lanczos scaling + dithering, especially multiplied per monitor.
            init.set_option("scale", "bilinear")?;
            init.set_option("dscale", "bilinear")?;
            init.set_option("dither", "no")?;
            init.set_option("deband", "no")?;
            init.set_option("correct-downscaling", "no")?;

            // Shrink demuxer readahead: mpv defaults to 150MiB + 50MiB back-buffer
            // per instance, which is wasted RAM for local looping files.
            init.set_option("demuxer-max-bytes", 40i64 * 1024 * 1024)?;
            init.set_option("demuxer-max-back-bytes", 10i64 * 1024 * 1024)?;

            // Cap software-decode threads so an hwdec fallback can't saturate
            // all cores multiplied by the number of monitor instances.
            init.set_option("vd-lavc-threads", 2i64)?;

            // Apply fit mode
            match fit_mode.as_str() {
                "fit" => {
                    init.set_option("keepaspect", "yes")?;
                    init.set_option("video-aspect-override", "0")?;
                }
                "fill" => {
                    init.set_option("keepaspect", "no")?;
                    init.set_option("video-aspect-override", "-1")?;
                }
                "stretch" => {
                    init.set_option("keepaspect", "no")?;
                    init.set_option("video-aspect-override", "0")?;
                }
                _ => {
                    init.set_option("keepaspect", "yes")?;
                    init.set_option("video-aspect-override", "0")?;
                }
            }

            // Apply FPS cap if specified. The fps filter drops frames before
            // render, actually reducing decode-adjacent and render work — unlike
            // `display-fps-override`, which only declares the refresh rate.
            // Must go through the `lavfi` wrapper: the bundled libmpv build
            // rejects the bare `fps=<n>` filter syntax with PropertyFormat (-9).
            if let Some(fps) = fps_cap {
                init.set_option("vf", format!("lavfi=[fps={}]", fps).as_str())?;
            }

            Ok(())
        })
        .map_err(|e| anyhow!("Failed to initialize mpv: {}", describe_mpv_error(&e)))?;

        tracing::info!(
            "libmpv initialized successfully and attached to window: {}",
            wid
        );
        self.mpv = Some(mpv);
        Ok(())
    }

    /// Start video playback
    pub fn play(&mut self) -> Result<()> {
        tracing::info!("Starting video playback: {}", self.video_path);

        if let Some(ref mpv) = self.mpv {
            // Load the video file
            mpv.command("loadfile", &[&self.video_path, "replace"])
                .map_err(|e| anyhow!("Failed to load video file: {}", describe_mpv_error(&e)))?;

            self.wait_for_video_output(mpv)?;

            // Apply speed after video is loaded
            if self.speed != 1.0 {
                if let Err(e) = mpv.set_property("speed", self.speed) {
                    tracing::warn!("Failed to set initial playback speed: {:?}", e);
                }
            }

            tracing::info!("Video loaded successfully, playback starting");
            self.playing = true;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    fn wait_for_video_output(&self, mpv: &Mpv) -> Result<()> {
        let readiness_timeout = Duration::from_secs(5);
        let deadline = Instant::now() + readiness_timeout;
        let mut loaded_file = false;
        let mut observed_events = Vec::new();

        while Instant::now() < deadline {
            match mpv.wait_event(0.01) {
                Some(Ok(event)) => {
                    let event_name = format!("{:?}", event);
                    observed_events.push(event_name.clone());

                    match event {
                        Event::FileLoaded => {
                            loaded_file = true;
                            tracing::debug!("mpv loaded video file");
                        }
                        Event::VideoReconfig => {
                            tracing::debug!("mpv configured video output");
                            return Ok(());
                        }
                        Event::PlaybackRestart => {
                            tracing::debug!("mpv playback restarted");
                            return Ok(());
                        }
                        Event::EndFile(reason) => {
                            return Err(anyhow!(
                                "mpv ended playback before video output was ready: {:?}",
                                reason
                            ));
                        }
                        _ => {}
                    }
                }
                Some(Err(error)) => {
                    return Err(anyhow!(
                        "mpv reported playback error for '{}': {}",
                        self.video_path,
                        describe_mpv_error(&error)
                    ));
                }
                _ => {}
            }
        }

        let events = if observed_events.is_empty() {
            "<none>".to_string()
        } else {
            observed_events.join(", ")
        };

        let message = if loaded_file {
            format!(
                "mpv loaded '{}', but did not report video output readiness within {:?}; events: {}",
                self.video_path, readiness_timeout, events
            )
        } else {
            format!(
                "mpv did not report file load or video output readiness for '{}' within {:?}; events: {}",
                self.video_path, readiness_timeout, events
            )
        };

        tracing::warn!("{}", message);
        Err(anyhow!(message))
    }

    /// Pause or resume playback by toggling mpv's `pause` property.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        tracing::info!("Setting video playback paused = {}", paused);

        if let Some(ref mpv) = self.mpv {
            mpv.set_property("pause", paused)
                .map_err(|e| anyhow!("Failed to set pause state: {:?}", e))?;
            self.playing = !paused;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    /// Set playback speed (0.1x to 10.0x)
    pub fn set_speed(&mut self, speed: f64) -> Result<()> {
        let clamped_speed = speed.clamp(0.1, 10.0);
        tracing::info!("Setting video playback speed to {}x", clamped_speed);

        if let Some(ref mpv) = self.mpv {
            mpv.set_property("speed", clamped_speed)
                .map_err(|e| anyhow!("Failed to set playback speed: {}", describe_mpv_error(&e)))?;
            self.speed = clamped_speed;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    /// Set video fit mode: "fit" (preserve aspect), "fill" (crop to fill), "stretch" (ignore aspect)
    pub fn set_fit_mode(&mut self, mode: &str) -> Result<()> {
        tracing::info!("Setting video fit mode to: {}", mode);

        if let Some(ref mpv) = self.mpv {
            match mode {
                "fit" => {
                    mpv.set_property("keepaspect", "yes").map_err(|e| {
                        anyhow!("Failed to set keepaspect: {}", describe_mpv_error(&e))
                    })?;
                    mpv.set_property("video-aspect-override", "0")
                        .map_err(|e| {
                            anyhow!(
                                "Failed to set video-aspect-override: {}",
                                describe_mpv_error(&e)
                            )
                        })?;
                }
                "fill" => {
                    mpv.set_property("keepaspect", "no").map_err(|e| {
                        anyhow!("Failed to set keepaspect: {}", describe_mpv_error(&e))
                    })?;
                    mpv.set_property("video-aspect-override", "-1")
                        .map_err(|e| {
                            anyhow!(
                                "Failed to set video-aspect-override: {}",
                                describe_mpv_error(&e)
                            )
                        })?;
                }
                "stretch" => {
                    mpv.set_property("keepaspect", "no").map_err(|e| {
                        anyhow!("Failed to set keepaspect: {}", describe_mpv_error(&e))
                    })?;
                    mpv.set_property("video-aspect-override", "0")
                        .map_err(|e| {
                            anyhow!(
                                "Failed to set video-aspect-override: {}",
                                describe_mpv_error(&e)
                            )
                        })?;
                }
                _ => {
                    return Err(anyhow!(
                        "Invalid fit mode: {}. Use 'fit', 'fill', or 'stretch'",
                        mode
                    ));
                }
            }
            self.fit_mode = mode.to_string();
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    /// Set video aspect mode using FitMode enum
    pub fn set_aspect_mode(&mut self, mode: super::engine::FitMode) -> Result<()> {
        let mode_str = match mode {
            super::engine::FitMode::Fit => "fit",
            super::engine::FitMode::Fill => "fill",
            super::engine::FitMode::Stretch => "stretch",
        };
        self.set_fit_mode(mode_str)
    }

    /// Set FPS cap (None for uncapped, Some(fps) to limit frame rate)
    pub fn set_fps_cap(&mut self, fps: Option<u32>) -> Result<()> {
        let validated_fps = fps.and_then(|f| (1..=240).contains(&f).then_some(f));

        if let Some(fps_value) = validated_fps {
            tracing::info!("Setting FPS cap to: {}", fps_value);
        } else {
            tracing::info!("Removing FPS cap (uncapped)");
        }

        if let Some(ref mpv) = self.mpv {
            if let Some(fps_value) = validated_fps {
                // The fps filter drops frames before render, actually reducing
                // work — unlike `display-fps-override`, which only declares the
                // refresh rate. Must go through the `lavfi` wrapper: the bundled
                // libmpv build rejects bare `fps=<n>` with PropertyFormat (-9).
                mpv.set_property("vf", format!("lavfi=[fps={}]", fps_value).as_str())
                    .map_err(|e| anyhow!("Failed to set FPS cap: {}", describe_mpv_error(&e)))?;
            } else {
                // Clear the filter chain to remove the cap
                mpv.set_property("vf", "")
                    .map_err(|e| anyhow!("Failed to remove FPS cap: {}", describe_mpv_error(&e)))?;
            }
            self.fps_cap = validated_fps;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    /// Stop video playback
    pub fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping video playback");

        if let Some(ref mpv) = self.mpv {
            mpv.command("stop", &[])
                .map_err(|e| anyhow!("Failed to stop: {:?}", e))?;
            self.playing = false;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    /// Set window position and size
    pub fn set_bounds(&mut self, x: i32, y: i32, width: i32, height: i32) -> Result<()> {
        self.set_bounds_with_flags(x, y, width, height, SET_WINDOW_POS_FLAGS(0))
    }

    /// Set window bounds without changing the z-order established by the desktop host.
    pub fn set_bounds_preserve_z_order(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.set_bounds_with_flags(x, y, width, height, SWP_NOZORDER)
    }

    fn set_bounds_with_flags(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: SET_WINDOW_POS_FLAGS,
    ) -> Result<()> {
        if let Some(hwnd) = self.hwnd {
            unsafe {
                if let Err(error) = SetWindowPos(hwnd, HWND_BOTTOM, x, y, width, height, flags) {
                    handle_set_window_pos_error("video player bounds", error)?;
                }
            }
            tracing::info!("Set bounds: {}x{} at ({}, {})", width, height, x, y);
        }
        Ok(())
    }

    /// Show the player window after mpv has been attached and playback requested.
    pub fn show(&self) -> Result<()> {
        if let Some(hwnd) = self.hwnd {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            tracing::info!("Showed video player window: {:?}", hwnd);
        }
        Ok(())
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        if self.playing {
            let _ = self.stop();
        }

        // mpv will be dropped automatically, which sends quit command

        // Destroy the window
        if let Some(hwnd) = self.hwnd {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            tracing::debug!("Destroyed video player window");
        }
    }
}

fn normalize_video_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Err(anyhow!("Video file does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(anyhow!("Video path is not a file: {}", path.display()));
    }

    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn describe_mpv_error(error: &MpvError) -> String {
    match error {
        MpvError::Raw(code) => format!("{} ({})", mpv_error_name(*code), code),
        _ => format!("{:?}", error),
    }
}

fn mpv_error_name(code: i32) -> &'static str {
    match code {
        mpv_error::LoadingFailed => "LoadingFailed",
        mpv_error::VoInitFailed => "VoInitFailed",
        mpv_error::AoInitFailed => "AoInitFailed",
        mpv_error::NothingToPlay => "NothingToPlay",
        mpv_error::UnknownFormat => "UnknownFormat",
        mpv_error::Unsupported => "Unsupported",
        mpv_error::InvalidParameter => "InvalidParameter",
        mpv_error::Command => "CommandFailed",
        mpv_error::OptionError => "OptionError",
        mpv_error::OptionFormat => "OptionFormat",
        mpv_error::OptionNotFound => "OptionNotFound",
        mpv_error::PropertyError => "PropertyError",
        mpv_error::PropertyFormat => "PropertyFormat",
        mpv_error::PropertyNotFound => "PropertyNotFound",
        mpv_error::PropertyUnavailable => "PropertyUnavailable",
        mpv_error::Uninitialized => "Uninitialized",
        mpv_error::NoMem => "NoMem",
        mpv_error::NotImplemented => "NotImplemented",
        mpv_error::Generic => "Generic",
        _ => "UnknownMpvError",
    }
}

fn handle_set_window_pos_error(context: &str, error: WindowsError) -> Result<()> {
    if error.code().0 == 0 {
        tracing::warn!(
            "SetWindowPos reported failure while updating {}, but GetLastError was ERROR_SUCCESS",
            context
        );
        Ok(())
    } else {
        Err(anyhow!("Failed to set {}: {}", context, error))
    }
}

/// Register the window class for the video player
fn register_window_class() -> Result<()> {
    unsafe {
        let class_name = encode_wide("WallscapeVideoPlayer");
        let hinstance = GetModuleHandleW(PCWSTR::null())
            .map_err(|e| anyhow!("Failed to get module handle: {}", e))?;

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            hIconSm: Default::default(),
        };

        SetLastError(ERROR_SUCCESS);
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            let last_error = GetLastError();
            let error = WindowsError::from_win32();
            if last_error.0 as i32 == ERROR_CLASS_ALREADY_EXISTS_CODE {
                tracing::debug!("Video player window class is already registered");
                return Ok(());
            }

            if last_error == ERROR_SUCCESS {
                return Err(anyhow!(
                    "Failed to register video player window class: RegisterClassExW returned 0 but GetLastError was ERROR_SUCCESS"
                ));
            }

            return Err(anyhow!(
                "Failed to register video player window class: {}",
                error
            ));
        }

        tracing::debug!("Registered window class: WallscapeVideoPlayer");
        Ok(())
    }
}

fn describe_create_window_error(
    error: WindowsError,
    parent: HWND,
    ex_style: WINDOW_EX_STYLE,
    style: windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE,
    layered_opaque: bool,
) -> anyhow::Error {
    let last_error = unsafe { GetLastError() };
    if error.code().0 == 0 {
        return anyhow!(
            "Failed to create video player window: CreateWindowExW returned NULL without setting GetLastError (parent={:?}, ex_style=0x{:x}, style=0x{:x}, layered_opaque={}, last_error={:?})",
            parent,
            ex_style.0,
            style.0,
            layered_opaque,
            last_error
        );
    }

    anyhow!(
        "Failed to create video player window: {} (parent={:?}, ex_style=0x{:x}, style=0x{:x}, layered_opaque={}, last_error={:?})",
        error,
        parent,
        ex_style.0,
        style.0,
        layered_opaque,
        last_error
    )
}

/// Window procedure for the video player window
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Helper function to encode a Rust string to wide (UTF-16) for Windows API
fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
