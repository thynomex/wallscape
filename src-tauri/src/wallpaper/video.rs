use anyhow::{anyhow, Result};
use libmpv2::events::Event;
use libmpv2::{mpv_error, Error as MpvError, Mpv};
use std::ffi::{c_char, c_void, CString};
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
const FPS_FILTER_LABEL: &str = "@wallscape-fps";

#[path = "loop_clock.rs"]
mod loop_clock;
use loop_clock::{FilterTiming, LoopClock};

unsafe extern "C" {
    fn mpv_command_string(ctx: *mut c_void, args: *const c_char) -> i32;
}

#[derive(Clone)]
struct CappedLoop {
    clock: LoopClock,
    last_update: Instant,
    image_display_duration: String,
}

/// Video player using hardware acceleration via libmpv
pub struct VideoPlayer {
    video_path: String,
    hwnd: Option<HWND>,
    mpv: Option<Mpv>,
    playing: bool,
    speed: f64,
    fit_mode: String,
    fps_cap: Option<u32>,
    capped_loop: Option<CappedLoop>,
}

impl VideoPlayer {
    pub fn new(video_path: impl AsRef<Path>) -> Result<Self> {
        let video_path = normalize_video_path(video_path.as_ref())?;

        tracing::debug!("Creating video player for: {}", video_path.display());

        Ok(Self {
            video_path: video_path.to_string_lossy().to_string(),
            hwnd: None,
            mpv: None,
            playing: false,
            speed: 1.0,
            fit_mode: "fit".to_string(),
            fps_cap: None,
            capped_loop: None,
        })
    }

    /// Create a window for video playback
    pub fn create_window(&mut self, parent: Option<HWND>, layered_opaque: bool) -> Result<HWND> {
        tracing::debug!("Creating video player window for: {}", self.video_path);

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
                Some(parent),
                None, // No menu
                Some(hinstance.into()),
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

            tracing::debug!("Created video player window: {:?}", hwnd);

            self.hwnd = Some(hwnd);
            if let Err(error) = self.initialize_mpv(hwnd) {
                let _ = DestroyWindow(hwnd);
                self.hwnd = None;
                return Err(error);
            }

            Ok(hwnd)
        }
    }

    fn initialize_mpv(&mut self, hwnd: HWND) -> Result<()> {
        let wid = hwnd.0 as i64;
        let fit_options = fit_mode_options(&self.fit_mode)?;
        let fps_filter = self.fps_cap.map(|fps| fps_filter(fps, self.speed, None));

        let mpv = Mpv::with_initializer(|init| {
            init.set_option("wid", wid)?;
            init.set_option("hwdec", "auto")?;
            init.set_option("loop-file", "inf")?;
            init.set_option("audio", "no")?;
            init.set_option("vo", "gpu")?;

            for (name, value) in fit_options {
                init.set_option(name, value)?;
            }

            if let Some(ref filter) = fps_filter {
                init.set_option("vf", filter.as_str())?;
            }

            Ok(())
        })
        .map_err(|e| anyhow!("Failed to initialize mpv: {}", describe_mpv_error(&e)))?;

        tracing::debug!(
            "libmpv initialized successfully and attached to window: {}",
            wid
        );
        self.mpv = Some(mpv);
        Ok(())
    }

    /// Start video playback
    pub fn play(&mut self) -> Result<()> {
        tracing::debug!("Starting video playback: {}", self.video_path);

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

            tracing::debug!("Video loaded successfully, playback starting");
            self.playing = true;
            if self.fps_cap.is_some() {
                self.capped_loop = None;
                self.set_fps_cap(self.fps_cap)?;
            }
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
        tracing::debug!("Setting video playback paused = {}", paused);
        self.advance_loop_clock();

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
        if !speed.is_finite() {
            return Err(anyhow!("Playback speed must be a finite number"));
        }
        let clamped_speed = speed.clamp(0.1, 10.0);
        tracing::debug!("Setting video playback speed to {}x", clamped_speed);
        self.advance_loop_clock();
        let mut next_loop = self.capped_loop.clone();
        if let Some(state) = next_loop.as_mut() {
            state.clock.set_speed(clamped_speed);
        }

        if let Some(ref mpv) = self.mpv {
            if let Some(fps) = self.fps_cap {
                apply_fps_filter(mpv, fps, clamped_speed, loop_timing(&next_loop))?;
            }
            if let Err(error) = mpv.set_property("speed", clamped_speed) {
                if let Some(fps) = self.fps_cap {
                    if let Err(rollback_error) =
                        apply_fps_filter(mpv, fps, self.speed, loop_timing(&self.capped_loop))
                    {
                        tracing::warn!("Failed to restore FPS cap: {}", rollback_error);
                    }
                }
                return Err(anyhow!(
                    "Failed to set playback speed: {}",
                    describe_mpv_error(&error)
                ));
            }
            self.speed = clamped_speed;
            self.capped_loop = next_loop;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    /// Set video fit mode: "fit" (preserve aspect), "fill" (crop to fill), "stretch" (ignore aspect)
    pub fn set_fit_mode(&mut self, mode: &str) -> Result<()> {
        tracing::debug!("Setting video fit mode to: {}", mode);
        let options = fit_mode_options(mode)?;

        if let Some(ref mpv) = self.mpv {
            for (name, value) in options {
                mpv.set_property(name, value)
                    .map_err(|e| anyhow!("Failed to set {}: {}", name, describe_mpv_error(&e)))?;
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
        self.advance_loop_clock();

        if let Some(fps_value) = validated_fps {
            tracing::debug!("Setting FPS cap to: {}", fps_value);
        } else {
            tracing::debug!("Removing FPS cap (uncapped)");
        }

        if let Some(ref mpv) = self.mpv {
            let mut next_loop = self.capped_loop.clone();
            if let Some(fps_value) = validated_fps {
                if let Some(state) = next_loop.as_mut() {
                    state.clock.update_cap(fps_value);
                } else if let Ok(duration) = mpv.get_property::<f64>("duration") {
                    next_loop = LoopClock::new(
                        duration,
                        mpv.get_property::<f64>("time-pos").unwrap_or(0.0),
                        self.speed,
                        fps_value,
                    )
                    .map(|clock| CappedLoop {
                        clock,
                        last_update: Instant::now(),
                        image_display_duration: mpv
                            .get_property::<String>("image-display-duration")
                            .unwrap_or_else(|_| "5".to_string()),
                    });
                }
                if next_loop.is_none() && mpv.get_property::<String>("path").is_ok() {
                    return Err(anyhow!(
                        "Cannot cap this video's frame rate because its duration is unavailable"
                    ));
                }
                apply_fps_filter(mpv, fps_value, self.speed, loop_timing(&next_loop))?;
                if next_loop.is_some() {
                    // libmpv loses the final frame duration through AV filters.
                    // Hold that image and let the source clock schedule looping.
                    mpv.set_property("keep-open", "yes")?;
                    mpv.set_property("keep-open-pause", false)?;
                    mpv.set_property("loop-file", "no")?;
                    mpv.set_property("image-display-duration", 0.0)?;
                }
            } else if self.fps_cap.is_some() {
                mpv.command("vf", &["remove", FPS_FILTER_LABEL])
                    .map_err(|e| anyhow!("Failed to remove FPS cap: {}", describe_mpv_error(&e)))?;
                if let Some(state) = next_loop.as_ref() {
                    let was_at_end = mpv.get_property::<bool>("eof-reached").unwrap_or(false);
                    mpv.set_property("loop-file", "inf")?;
                    mpv.set_property("keep-open", "no")?;
                    mpv.set_property(
                        "image-display-duration",
                        state.image_display_duration.as_str(),
                    )?;
                    if was_at_end {
                        mpv.command("seek", &["0", "absolute+exact"])?;
                    }
                }
                next_loop = None;
            }
            self.fps_cap = validated_fps;
            self.capped_loop = next_loop;
            Ok(())
        } else {
            Err(anyhow!("MPV not initialized"))
        }
    }

    fn advance_loop_clock(&mut self) {
        if let Some(state) = self.capped_loop.as_mut() {
            let now = Instant::now();
            state.clock.advance(now - state.last_update, self.playing);
            state.last_update = now;
        }
    }

    /// Called by the existing wallpaper thread. Keeps capped clips on their
    /// original timeline and carries the frame-selection phase across loops.
    pub fn service_video_loop(&mut self) -> Result<()> {
        self.advance_loop_clock();
        if !self.playing {
            return Ok(());
        }
        let (Some(mpv), Some(state), Some(fps)) = (&self.mpv, &self.capped_loop, self.fps_cap)
        else {
            return Ok(());
        };
        if !mpv.get_property::<bool>("eof-reached").unwrap_or(false) {
            return Ok(());
        }
        let mut next = state.clock.clone();
        if let Some(timing) = next.restart_due() {
            replace_filter_and_restart(mpv, fps, self.speed, timing)?;
            self.capped_loop.as_mut().unwrap().clock = next;
        }
        Ok(())
    }

    /// Stop video playback
    pub fn stop(&mut self) -> Result<()> {
        tracing::debug!("Stopping video playback");

        if let Some(ref mpv) = self.mpv {
            mpv.command("stop", &[])
                .map_err(|e| anyhow!("Failed to stop: {:?}", e))?;
            self.playing = false;
            self.capped_loop = None;
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
                if let Err(error) =
                    SetWindowPos(hwnd, Some(HWND_BOTTOM), x, y, width, height, flags)
                {
                    handle_set_window_pos_error("video player bounds", error)?;
                }
            }
            tracing::debug!("Set bounds: {}x{} at ({}, {})", width, height, x, y);
        }
        Ok(())
    }

    /// Show the player window after mpv has been attached and playback requested.
    pub fn show(&self) -> Result<()> {
        if let Some(hwnd) = self.hwnd {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            tracing::debug!("Showed video player window: {:?}", hwnd);
        }
        Ok(())
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        if self.playing {
            let _ = self.stop();
        }

        // Release mpv before destroying the window it renders into, including paused players.
        self.mpv.take();

        // Destroy the window
        if let Some(hwnd) = self.hwnd {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            tracing::debug!("Destroyed video player window");
        }
    }
}

fn loop_timing(state: &Option<CappedLoop>) -> Option<FilterTiming> {
    state.as_ref().map(|state| state.clock.filter_timing())
}

fn fps_filter(fps: u32, speed: f64, timing: Option<FilterTiming>) -> String {
    let FilterTiming {
        offset_seconds,
        first_bucket,
    } = timing.unwrap_or(FilterTiming {
        offset_seconds: 0.0,
        first_bucket: 0,
    });
    // Keep the first frame in each time bucket. Unlike the fps filter, select
    // never duplicates low-FPS input. Account for playback speed and timestamp
    // rounding to half a filter timebase tick; preserve the original PTS.
    // The direct select alias also rejects runtimes missing this filter early.
    let bucket = format!("floor(((t+TB/2)/{speed}+{offset_seconds})*{fps}+0.0000001)");
    let previous =
        format!("floor(((prev_selected_t+TB/2)/{speed}+{offset_seconds})*{fps}+0.0000001)");
    format!("{FPS_FILTER_LABEL}:select=expr=[if(isnan(prev_selected_t),gte({bucket},{first_bucket}),gt({bucket},{previous}))]")
}

fn apply_fps_filter(mpv: &Mpv, fps: u32, speed: f64, timing: Option<FilterTiming>) -> Result<()> {
    // Adding the same label replaces only our filter, preserving other filters.
    mpv.command("vf", &["add", &fps_filter(fps, speed, timing)])
        .map_err(|error| {
            anyhow!(
                "Failed to set FPS cap: {}. The libmpv runtime must include the FFmpeg select filter; run npm run setup:mpv.",
                describe_mpv_error(&error)
            )
        })
}

fn replace_filter_and_restart(mpv: &Mpv, fps: u32, speed: f64, timing: FilterTiming) -> Result<()> {
    // Updating vf at retained EOF triggers a refresh seek. Submit both commands
    // together so that refresh cannot render the previous loop's last frame.
    // The expression contains only our fixed syntax and finite numeric values.
    let filter = fps_filter(fps, speed, Some(timing));
    let command = CString::new(format!(
        "no-osd vf add \"{filter}\"; no-osd seek 0 absolute+exact"
    ))?;
    // SAFETY: ctx is the live handle owned by mpv; command stays NUL-terminated
    // and alive for this synchronous libmpv API call. libmpv2 lacks this wrapper.
    let result = unsafe { mpv_command_string(mpv.ctx.as_ptr().cast(), command.as_ptr()) };
    if result < 0 {
        return Err(anyhow!(
            "Failed to restart capped video: {}",
            describe_mpv_error(&MpvError::Raw(result))
        ));
    }
    Ok(())
}

fn fit_mode_options(mode: &str) -> Result<[(&'static str, &'static str); 3]> {
    let (keepaspect, panscan) = match mode {
        "fit" => ("yes", "0"),
        "fill" => ("yes", "1"),
        "stretch" => ("no", "0"),
        _ => {
            return Err(anyhow!(
                "Invalid fit mode: {}. Use 'fit', 'fill', or 'stretch'",
                mode
            ));
        }
    };

    // Preserve the video's display aspect ratio, including non-square pixels.
    // Panscan fills the window by cropping, and must be cleared when leaving Fill.
    Ok([
        ("video-aspect-override", "no"),
        ("keepaspect", keepaspect),
        ("panscan", panscan),
    ])
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
            let error = WindowsError::from_thread();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn headless_player(mode: &str) -> VideoPlayer {
        let fit_options = fit_mode_options(mode).unwrap();
        let mpv = Mpv::with_initializer(|init| {
            init.set_option("vo", "null")?;
            init.set_option("audio", "no")?;
            for (name, value) in fit_options {
                init.set_option(name, value)?;
            }
            Ok(())
        })
        .unwrap();

        VideoPlayer {
            video_path: String::new(),
            hwnd: None,
            mpv: Some(mpv),
            playing: false,
            speed: 1.0,
            fit_mode: mode.to_string(),
            fps_cap: None,
            capped_loop: None,
        }
    }

    fn assert_fit_properties(player: &VideoPlayer, keepaspect: bool, panscan: f64) {
        let mpv = player.mpv.as_ref().unwrap();
        assert_eq!(mpv.get_property::<bool>("keepaspect").unwrap(), keepaspect);
        assert_eq!(mpv.get_property::<f64>("panscan").unwrap(), panscan);
        // libmpv exposes the "no" option as its numeric sentinel, -2.
        assert_eq!(
            mpv.get_property::<f64>("video-aspect-override").unwrap(),
            -2.0
        );
    }

    #[test]
    fn video_fit_modes_preserve_aspect_and_clear_fill_crop() {
        // Exercise the linked libmpv, including option acceptance before init
        // and every runtime mode transition, without creating a desktop window.
        for initial_mode in ["fit", "fill", "stretch"] {
            let mut player = headless_player(initial_mode);
            assert_fit_properties(
                &player,
                initial_mode != "stretch",
                if initial_mode == "fill" { 1.0 } else { 0.0 },
            );

            for (mode, keepaspect, panscan) in [
                ("fill", true, 1.0),
                ("fit", true, 0.0),
                ("stretch", false, 0.0),
            ] {
                player.set_fit_mode("fill").unwrap();
                player.set_fit_mode(mode).unwrap();
                assert_fit_properties(&player, keepaspect, panscan);
            }
        }
    }

    #[test]
    fn invalid_video_fit_mode_leaves_player_unchanged() {
        let mut player = headless_player("fill");
        assert!(player.set_fit_mode("invalid").is_err());
        assert_eq!(player.fit_mode, "fill");
        assert_fit_properties(&player, true, 1.0);
    }

    fn write_test_video(video_path: &Path, source_fps: u32) {
        write_test_video_frames(video_path, source_fps, source_fps * 2);
    }

    fn write_test_video_frames(video_path: &Path, source_fps: u32, frame_count: u32) {
        let mut video = std::fs::File::create(video_path).unwrap();
        writeln!(video, "YUV4MPEG2 W2 H2 F{source_fps}:1 Ip A1:1 C420jpeg").unwrap();
        for _ in 0..frame_count {
            video.write_all(b"FRAME\n\x10\x10\x10\x10\x80\x80").unwrap();
        }
    }

    fn played_frame_count(
        source_fps: u32,
        cap: Option<u32>,
        speed: f64,
        repeat: bool,
        update: impl FnOnce(&mut VideoPlayer),
    ) -> usize {
        let dir = tempfile::tempdir().unwrap();
        let video_path = dir.path().join("frames.y4m");
        let log_path = dir.path().join("frames.log");
        write_test_video(&video_path, source_fps);

        let mut player = headless_player("fit");
        player.video_path = video_path.to_string_lossy().into_owned();
        let mpv = player.mpv.as_ref().unwrap();
        mpv.set_property("untimed", true).unwrap();
        if repeat {
            mpv.set_property("loop-file", "1").unwrap();
        }
        player.set_fps_cap(cap).unwrap();
        player.set_speed(speed).unwrap();
        update(&mut player);
        // FFmpeg's global logger can point at another libmpv instance. Write
        // per-filter metadata instead, counting actual frames after the cap.
        let escaped_log_path = log_path
            .to_string_lossy()
            .replace('\\', "/")
            .replace(':', "\\:")
            .replace('\'', "'\\''");
        let frame_counter = format!(
            "@frame-count:lavfi=[metadata=mode=add:key=wallscape_frame:value=1,metadata=mode=print:file='{escaped_log_path}']"
        );
        player
            .mpv
            .as_ref()
            .unwrap()
            .command("vf", &["add", &frame_counter])
            .unwrap();
        player
            .mpv
            .as_ref()
            .unwrap()
            .command("loadfile", &[&player.video_path, "replace"])
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut saw_loop_seek = false;
        loop {
            assert!(Instant::now() < deadline, "headless video did not finish");
            match player.mpv.as_ref().unwrap().wait_event(0.01) {
                Some(Ok(Event::EndFile(_))) => break,
                Some(Ok(Event::Seek)) => saw_loop_seek = true,
                Some(Err(error)) => panic!("headless playback failed: {error:?}"),
                _ => {}
            }
        }
        if repeat {
            assert!(saw_loop_seek, "video did not loop");
        }
        drop(player);
        let log = std::fs::read_to_string(log_path).unwrap();
        let count = log
            .lines()
            .filter(|line| line.starts_with("frame:"))
            .count();
        assert!(count > 0, "metadata did not report any frames:\n{log}");
        count
    }

    #[test]
    fn video_fps_cap_drops_frames_without_upsampling_or_changing_speed() {
        for (source_fps, cap, speed, expected_frames) in [
            (60, None, 1.0, 120),
            (60, Some(30), 1.0, 60),
            (60, Some(24), 1.0, 48),
            (24, Some(30), 1.0, 48),
            (60, Some(30), 2.0, 30),
            // Covers filter timestamps rounded to microseconds at 60 FPS.
            (60, Some(30), 0.5, 120),
        ] {
            assert_eq!(
                played_frame_count(source_fps, cap, speed, false, |_| {}),
                expected_frames,
                "source={source_fps}, cap={cap:?}, speed={speed}"
            );
        }
    }

    #[test]
    fn video_fps_cap_replacement_removal_and_looping_keep_playback_working() {
        assert_eq!(
            played_frame_count(60, Some(30), 1.0, false, |player| {
                player.set_fps_cap(Some(60)).unwrap();
            }),
            120
        );
        assert_eq!(
            played_frame_count(60, Some(30), 2.0, false, |player| {
                player.set_speed(1.0).unwrap();
            }),
            60
        );
        assert_eq!(
            played_frame_count(60, Some(30), 1.0, false, |player| {
                let mpv = player.mpv.as_ref().unwrap();
                mpv.command("vf", &["add", "@unrelated:format=fmt=yuv420p"])
                    .unwrap();
                player.set_fps_cap(None).unwrap();
                player.set_fps_cap(None).unwrap();
                assert!(player
                    .mpv
                    .as_ref()
                    .unwrap()
                    .get_property::<String>("vf")
                    .unwrap()
                    .contains("unrelated"));
            }),
            120
        );
        // Seeking recreates the metadata writer; it contains the final loop.
        assert_eq!(played_frame_count(60, Some(30), 1.0, true, |_| {}), 60);
    }

    #[test]
    fn video_fps_filter_accepts_initial_options_and_live_changes() {
        let dir = tempfile::tempdir().unwrap();
        let video_path = dir.path().join("live.y4m");
        write_test_video(&video_path, 60);
        let filter = fps_filter(30, 1.0, None);
        let mpv = Mpv::with_initializer(|init| {
            init.set_option("vo", "null")?;
            init.set_option("audio", "no")?;
            init.set_option("pause", true)?;
            init.set_option("vf", filter.as_str())?;
            Ok(())
        })
        .unwrap();
        let mut player = VideoPlayer {
            video_path: video_path.to_string_lossy().into_owned(),
            hwnd: None,
            mpv: Some(mpv),
            playing: false,
            speed: 1.0,
            fit_mode: "fit".to_string(),
            fps_cap: Some(30),
            capped_loop: None,
        };
        player.play().unwrap();
        // The decoder and filter graph now exist, unlike pre-load option tests.
        player.set_speed(2.0).unwrap();
        player.set_fps_cap(Some(24)).unwrap();
        player.set_speed(1.0).unwrap();
        player.set_fps_cap(None).unwrap();
        player.set_fps_cap(Some(30)).unwrap();
        assert_eq!(
            player
                .mpv
                .as_ref()
                .unwrap()
                .get_property::<f64>("speed")
                .unwrap(),
            1.0
        );
        player.set_paused(false).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline, "live video did not resume");
            let mpv = player.mpv.as_ref().unwrap();
            if mpv.get_property::<f64>("time-pos").unwrap_or(0.0) >= 0.2 {
                break;
            }
            if let Some(Err(error)) = mpv.wait_event(0.01) {
                panic!("live playback failed: {error:?}");
            }
        }
        // Change speed and remove the cap while playback is advancing.
        player.set_speed(2.0).unwrap();
        player.set_fps_cap(None).unwrap();
        loop {
            assert!(Instant::now() < deadline, "uncapped video did not advance");
            let mpv = player.mpv.as_ref().unwrap();
            if mpv.get_property::<f64>("time-pos").unwrap_or(0.0) >= 0.5 {
                break;
            }
            if let Some(Err(error)) = mpv.wait_event(0.01) {
                panic!("live playback failed: {error:?}");
            }
        }
    }

    fn service_for(player: &mut VideoPlayer, duration: Duration) -> usize {
        let deadline = Instant::now() + duration;
        let mut seeks = 0;
        while Instant::now() < deadline {
            player.service_video_loop().unwrap();
            while let Some(event) = player.mpv.as_ref().unwrap().wait_event(0.0) {
                match event.unwrap() {
                    Event::Seek => seeks += 1,
                    Event::EndFile(reason) => panic!("managed loop ended: {reason:?}"),
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        seeks
    }

    #[test]
    fn video_capped_loop_holds_tail_and_freezes_during_pause() {
        let dir = tempfile::tempdir().unwrap();
        let video_path = dir.path().join("hold.y4m");
        write_test_video(&video_path, 60);
        let mut player = headless_player("fit");
        player.video_path = video_path.to_string_lossy().into_owned();
        player.set_fps_cap(Some(1)).unwrap();
        player.play().unwrap();

        // The filter emits frames at 0s and 1s. It must retain the last image
        // through the full 2s source duration rather than loop at the 1s EOF.
        service_for(&mut player, Duration::from_millis(1200));
        assert!(player
            .mpv
            .as_ref()
            .unwrap()
            .get_property::<bool>("eof-reached")
            .unwrap());
        player.set_paused(true).unwrap();
        assert_eq!(service_for(&mut player, Duration::from_millis(150)), 0);
        player.set_paused(false).unwrap();
        assert_eq!(service_for(&mut player, Duration::from_millis(400)), 0);
        assert!(player
            .mpv
            .as_ref()
            .unwrap()
            .get_property::<bool>("eof-reached")
            .unwrap());
        assert!(service_for(&mut player, Duration::from_millis(550)) >= 1);
    }

    #[test]
    fn video_removing_cap_at_eof_resumes_native_looping() {
        let dir = tempfile::tempdir().unwrap();
        let video_path = dir.path().join("resume.y4m");
        write_test_video(&video_path, 60);
        let mut player = headless_player("fit");
        player.video_path = video_path.to_string_lossy().into_owned();
        player.set_fps_cap(Some(1)).unwrap();
        player.play().unwrap();
        service_for(&mut player, Duration::from_millis(1200));
        assert!(player
            .mpv
            .as_ref()
            .unwrap()
            .get_property::<bool>("eof-reached")
            .unwrap());
        let held_position = player
            .mpv
            .as_ref()
            .unwrap()
            .get_property::<f64>("time-pos")
            .unwrap();
        player.set_fps_cap(None).unwrap();
        assert!(player.capped_loop.is_none());
        service_for(&mut player, Duration::from_millis(150));
        let position = player
            .mpv
            .as_ref()
            .unwrap()
            .get_property::<f64>("time-pos")
            .unwrap();
        assert!(
            (position - held_position).abs() > 0.05,
            "did not advance: {position}"
        );
        assert!(!player
            .mpv
            .as_ref()
            .unwrap()
            .get_property::<bool>("eof-reached")
            .unwrap());
    }

    #[test]
    fn video_single_frame_cap_enters_managed_hold() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("single-output.y4m");
        write_test_video(&path, 60);
        let mut player = headless_player("fit");
        player.video_path = path.to_string_lossy().into_owned();
        player.set_speed(10.0).unwrap();
        player.set_fps_cap(Some(4)).unwrap();
        player.play().unwrap();
        // A single retained frame must not trigger mpv's default 5s image hold.
        // The next eligible frame belongs at 0.25s across these 0.2s loops.
        assert!(service_for(&mut player, Duration::from_millis(375)) >= 1);
        assert!(player.capped_loop.is_some());
    }

    #[test]
    fn video_fast_capped_loops_limit_actual_frames() {
        const CHILD_FLAG: &str = "WALLSCAPE_MPV_TIMING_TEST_CHILD";
        if std::env::var_os(CHILD_FLAG).is_none() {
            // FFmpeg logging is global inside libmpv. Isolate the aggregate
            // counter from other tests' players and asynchronous destruction.
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "wallpaper::video::tests::video_fast_capped_loops_limit_actual_frames",
                    "--nocapture",
                ])
                .env(CHILD_FLAG, "1")
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fast.y4m");
        let log_path = dir.path().join("aggregate.log");
        write_test_video_frames(&path, 60, 60);
        let filter = format!("{},@counter:lavfi=[showinfo]", fps_filter(15, 10.0, None));
        let mpv = Mpv::with_initializer(|init| {
            init.set_option("vo", "null")?;
            init.set_option("audio", "no")?;
            init.set_option("log-file", log_path.to_str().unwrap())?;
            init.set_option("speed", 10.0)?;
            init.set_option("vf", filter.as_str())?;
            Ok(())
        })
        .unwrap();
        let mut player = VideoPlayer {
            video_path: path.to_string_lossy().into_owned(),
            hwnd: None,
            mpv: Some(mpv),
            playing: false,
            speed: 10.0,
            fit_mode: "fit".to_string(),
            fps_cap: Some(15),
            capped_loop: None,
        };
        player.play().unwrap();
        let seeks = service_for(&mut player, Duration::from_millis(985));
        player.stop().unwrap();
        drop(player);
        let log = std::fs::read_to_string(log_path).unwrap();
        let frames = log
            .lines()
            .filter(|line| line.contains(" n:") && line.contains(" pts:"))
            .count();
        // Ten short logical loops must share fifteen output buckets, rather
        // than each restart admitting an extra frame or refreshing the old one.
        assert!(seeks >= 7, "short loops did not advance: {seeks}");
        assert!(
            (12..=15).contains(&frames),
            "expected at most 15 output frames, got {frames}"
        );
    }
}
