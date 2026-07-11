use crate::settings::{Settings, SettingsState};
use crate::wallpaper::WallpaperRuntime;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Manager;
use windows::Win32::Foundation::{BOOL, HANDLE, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, CreateRectRgnIndirect, DeleteObject, EnumDisplayMonitors,
    GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    NULLREGION, RGN_DIFF, RGN_OR,
};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, PowerSettingRegisterNotification, DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS,
    POWERBROADCAST_SETTING, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP,
};
use windows::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW,
    GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible, DEVICE_NOTIFY_CALLBACK,
    GWL_EXSTYLE, SM_REMOTESESSION, WS_EX_TRANSPARENT,
};

const AUTO_PAUSE_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Set by the console display state power notification callback: true while
/// the console displays are off (or dimming to off). Unlike the
/// `OpenInputDesktop` probe, this covers plain monitor power-off with the
/// session still unlocked.
static CONSOLE_DISPLAY_OFF: AtomicBool = AtomicBool::new(false);

pub fn spawn(app_handle: tauri::AppHandle, runtime: WallpaperRuntime) {
    register_display_state_notification();

    thread::Builder::new()
        .name("wallscape-auto-pause".to_string())
        .spawn(move || run_auto_pause_loop(app_handle, runtime))
        .expect("failed to spawn auto-pause thread");
}

fn run_auto_pause_loop(app_handle: tauri::AppHandle, runtime: WallpaperRuntime) {
    loop {
        let Some(settings) = app_handle
            .try_state::<SettingsState>()
            .map(|settings| settings.get())
        else {
            thread::sleep(AUTO_PAUSE_POLL_INTERVAL);
            continue;
        };

        // Skip the snapshot syscalls entirely when the feature is off or no
        // wallpaper is running.
        let desired = settings.auto_pause_enabled
            && runtime.is_active()
            && should_auto_pause(&settings, AutoPauseSnapshot::detect(&settings));

        if runtime.is_auto_paused() != desired {
            if let Err(error) = runtime.set_auto_paused(desired) {
                tracing::warn!("Failed to apply auto-pause state: {}", error);
            } else {
                tracing::debug!("Auto-pause state changed: {}", desired);
            }
        }

        thread::sleep(AUTO_PAUSE_POLL_INTERVAL);
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct AutoPauseSnapshot {
    on_battery: bool,
    fullscreen_app: bool,
    desktop_occluded: bool,
    remote_session: bool,
    display_unavailable: bool,
}

impl AutoPauseSnapshot {
    fn detect(settings: &Settings) -> Self {
        Self {
            on_battery: settings.auto_pause_on_battery && is_on_battery_power(),
            fullscreen_app: settings.auto_pause_fullscreen_apps
                && foreground_window_is_fullscreen(),
            // The occlusion sweep (EnumWindows + GDI regions) is the most
            // expensive probe, so only run it when its rule is enabled.
            desktop_occluded: settings.auto_pause_occluded && all_monitors_occluded(),
            remote_session: settings.auto_pause_remote_session && is_remote_session(),
            display_unavailable: settings.auto_pause_display_sleep
                && (CONSOLE_DISPLAY_OFF.load(Ordering::Relaxed)
                    || interactive_desktop_unavailable()),
        }
    }
}

fn should_auto_pause(settings: &Settings, snapshot: AutoPauseSnapshot) -> bool {
    settings.auto_pause_enabled
        && ((settings.auto_pause_on_battery && snapshot.on_battery)
            || (settings.auto_pause_fullscreen_apps && snapshot.fullscreen_app)
            || (settings.auto_pause_occluded && snapshot.desktop_occluded)
            || (settings.auto_pause_remote_session && snapshot.remote_session)
            || (settings.auto_pause_display_sleep && snapshot.display_unavailable))
}

/// Register for console display state changes (on/dim/off). The callback keeps
/// [`CONSOLE_DISPLAY_OFF`] current so playback pauses while monitors are dark
/// even though the session is unlocked. The registration handle is
/// intentionally leaked — it must live for the process lifetime.
fn register_display_state_notification() {
    unsafe extern "system" fn on_power_setting_change(
        _context: *const core::ffi::c_void,
        _event_type: u32,
        setting: *const core::ffi::c_void,
    ) -> u32 {
        let setting = setting as *const POWERBROADCAST_SETTING;
        if !setting.is_null() && (*setting).PowerSetting == GUID_CONSOLE_DISPLAY_STATE {
            // Data[0]: 0 = off, 1 = on, 2 = dimmed.
            let display_on = (*setting).Data[0] != 0;
            CONSOLE_DISPLAY_OFF.store(!display_on, Ordering::Relaxed);
            tracing::debug!("Console display state changed: on = {}", display_on);
        }
        0
    }

    // Leaked deliberately: the OS may read the subscription parameters for the
    // lifetime of the registration, which lasts until process exit.
    let params = Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
        Callback: Some(on_power_setting_change),
        Context: std::ptr::null_mut(),
    }));
    let mut registration: *mut core::ffi::c_void = std::ptr::null_mut();

    let result = unsafe {
        PowerSettingRegisterNotification(
            &GUID_CONSOLE_DISPLAY_STATE,
            DEVICE_NOTIFY_CALLBACK,
            HANDLE(params as *const _ as *mut core::ffi::c_void),
            &mut registration,
        )
    };

    if result.is_err() {
        tracing::warn!(
            "Failed to register console display state notification: {:?}",
            result
        );
    }
}

fn is_on_battery_power() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();

    unsafe {
        if GetSystemPowerStatus(&mut status).is_err() {
            return false;
        }
    }

    status.ACLineStatus == 0
}

fn is_remote_session() -> bool {
    unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
}

/// True when every monitor's work area is completely covered by visible,
/// non-cloaked application windows (e.g. a maximized browser on each screen).
/// The wallpaper sits behind all of them, so decoding is pure waste.
fn all_monitors_occluded() -> bool {
    unsafe {
        // Union of all opaque window rects, accumulated by the EnumWindows
        // callback below.
        let covered = CreateRectRgn(0, 0, 0, 0);
        if covered.is_invalid() {
            return false;
        }

        unsafe extern "system" fn accumulate_window_region(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let covered = windows::Win32::Graphics::Gdi::HRGN(lparam.0 as *mut _);

            if !IsWindowVisible(hwnd).as_bool()
                || IsIconic(hwnd).as_bool()
                || is_click_through_overlay(hwnd)
                || is_window_cloaked(hwnd)
                || is_current_process_window(hwnd)
                || is_desktop_shell_window(hwnd)
            {
                return true.into();
            }

            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err()
                || rect.right <= rect.left
                || rect.bottom <= rect.top
            {
                return true.into();
            }

            let window_region = CreateRectRgnIndirect(&rect);
            if !window_region.is_invalid() {
                let _ = CombineRgn(covered, covered, window_region, RGN_OR);
                let _ = DeleteObject(window_region);
            }

            true.into()
        }

        let enum_ok =
            EnumWindows(Some(accumulate_window_region), LPARAM(covered.0 as isize)).is_ok();

        let mut all_occluded = enum_ok;

        if enum_ok {
            // For each monitor, check whether monitor_rect - covered == empty.
            unsafe extern "system" fn check_monitor_occluded(
                monitor: HMONITOR,
                _hdc: HDC,
                _rect: *mut RECT,
                lparam: LPARAM,
            ) -> BOOL {
                let state = &mut *(lparam.0 as *mut MonitorOcclusionState);
                state.monitor_count += 1;

                let mut info = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                    state.all_occluded = false;
                    return false.into();
                }

                let monitor_region = CreateRectRgnIndirect(&info.rcMonitor);
                if monitor_region.is_invalid() {
                    state.all_occluded = false;
                    return false.into();
                }

                let uncovered = CombineRgn(monitor_region, monitor_region, state.covered, RGN_DIFF);
                let occluded = uncovered == NULLREGION;
                let _ = DeleteObject(monitor_region);

                if !occluded {
                    state.all_occluded = false;
                    return false.into(); // Stop enumerating: one visible monitor is enough.
                }

                true.into()
            }

            struct MonitorOcclusionState {
                covered: windows::Win32::Graphics::Gdi::HRGN,
                all_occluded: bool,
                monitor_count: usize,
            }

            let mut state = MonitorOcclusionState {
                covered,
                all_occluded: true,
                monitor_count: 0,
            };

            let monitors_enumerated = EnumDisplayMonitors(
                HDC(std::ptr::null_mut()),
                None,
                Some(check_monitor_occluded),
                LPARAM(&mut state as *mut _ as isize),
            )
            .as_bool();

            all_occluded = monitors_enumerated && state.monitor_count > 0 && state.all_occluded;
        }

        let _ = DeleteObject(covered);
        all_occluded
    }
}

fn is_window_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .map(|_| cloaked != 0)
        .unwrap_or(false)
    }
}

/// Click-through overlays (game overlays, screen tinting tools) span whole
/// monitors while being visually transparent — they must not count as cover.
fn is_click_through_overlay(hwnd: HWND) -> bool {
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    ex_style & WS_EX_TRANSPARENT.0 != 0
}

fn interactive_desktop_unavailable() -> bool {
    unsafe {
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_SWITCHDESKTOP) {
            Ok(desktop) => {
                let _ = CloseDesktop(desktop);
                false
            }
            Err(_) => true,
        }
    }
}

fn foreground_window_is_fullscreen() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null()
            || !IsWindowVisible(hwnd).as_bool()
            || is_current_process_window(hwnd)
            || is_desktop_shell_window(hwnd)
        {
            return false;
        }

        let mut window_rect = RECT::default();
        if GetWindowRect(hwnd, &mut window_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return false;
        }

        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return false;
        }

        covers_monitor(window_rect, monitor_info.rcMonitor)
    }
}

fn is_current_process_window(hwnd: HWND) -> bool {
    let mut process_id = 0;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        process_id == GetCurrentProcessId()
    }
}

fn is_desktop_shell_window(hwnd: HWND) -> bool {
    is_desktop_shell_class(&window_class_name(hwnd))
}

fn is_desktop_shell_class(class_name: &str) -> bool {
    matches!(class_name, "Progman" | "WorkerW")
}

fn window_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);

        if len > 0 {
            String::from_utf16_lossy(&class_name[..len as usize])
        } else {
            String::new()
        }
    }
}

fn covers_monitor(window: RECT, monitor: RECT) -> bool {
    let window_width = window.right - window.left;
    let window_height = window.bottom - window.top;
    let monitor_width = monitor.right - monitor.left;
    let monitor_height = monitor.bottom - monitor.top;

    window_width >= monitor_width
        && window_height >= monitor_height
        && window.left <= monitor.left
        && window.top <= monitor.top
        && window.right >= monitor.right
        && window.bottom >= monitor.bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_pause_requires_master_toggle() {
        let settings = Settings {
            auto_pause_enabled: false,
            auto_pause_on_battery: true,
            ..Settings::default()
        };

        assert!(!should_auto_pause(
            &settings,
            AutoPauseSnapshot {
                on_battery: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn auto_pause_uses_enabled_rules_only() {
        let settings = Settings {
            auto_pause_enabled: true,
            auto_pause_on_battery: false,
            auto_pause_fullscreen_apps: true,
            auto_pause_remote_session: false,
            auto_pause_display_sleep: false,
            ..Settings::default()
        };

        assert!(!should_auto_pause(
            &settings,
            AutoPauseSnapshot {
                on_battery: true,
                ..Default::default()
            }
        ));
        assert!(should_auto_pause(
            &settings,
            AutoPauseSnapshot {
                fullscreen_app: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn auto_pause_respects_occlusion_rule() {
        let settings = Settings {
            auto_pause_enabled: true,
            auto_pause_on_battery: false,
            auto_pause_fullscreen_apps: false,
            auto_pause_occluded: true,
            auto_pause_remote_session: false,
            auto_pause_display_sleep: false,
            ..Settings::default()
        };

        assert!(should_auto_pause(
            &settings,
            AutoPauseSnapshot {
                desktop_occluded: true,
                ..Default::default()
            }
        ));

        let disabled = Settings {
            auto_pause_occluded: false,
            ..settings
        };
        assert!(!should_auto_pause(
            &disabled,
            AutoPauseSnapshot {
                desktop_occluded: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn fullscreen_window_must_cover_monitor() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };

        assert!(covers_monitor(monitor, monitor));
        assert!(!covers_monitor(
            RECT {
                left: 10,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            monitor,
        ));
    }

    #[test]
    fn desktop_shell_windows_are_not_fullscreen_apps() {
        assert!(is_desktop_shell_class("Progman"));
        assert!(is_desktop_shell_class("WorkerW"));
        assert!(!is_desktop_shell_class("Chrome_WidgetWin_1"));
    }
}
