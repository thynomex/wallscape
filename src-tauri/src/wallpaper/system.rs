use super::monitor::{Monitor, MonitorManager};
use anyhow::{anyhow, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{RECT, RPC_E_CHANGED_MODE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper};
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_GETDESKWALLPAPER,
    SPI_SETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

const WALLPAPER_PATH_BUFFER_LEN: usize = 32_768;

pub fn get_desktop_wallpaper() -> Result<Option<String>> {
    let mut buffer = vec![0u16; WALLPAPER_PATH_BUFFER_LEN];

    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .map_err(|e| anyhow!("Failed to read current Windows wallpaper: {}", e))?;
    }

    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..len]).trim().to_string();

    if path.is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

pub fn set_desktop_wallpaper(path: &str) -> Result<()> {
    let path = existing_wallpaper_path(path)?;

    // Use encode_wide() directly from OsStr to preserve Windows native UTF-16 encoding
    // This avoids lossy UTF-8 conversion that can corrupt paths on Windows 11
    let wide_path = wide_from_os_str(path.as_os_str());

    unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(wide_path.as_ptr() as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
        .map_err(|e| anyhow!("Failed to restore Windows wallpaper: {}", e))?;
    }

    Ok(())
}

pub fn set_desktop_wallpaper_for_monitor(path: &str, monitor_id: &str) -> Result<()> {
    let path = existing_wallpaper_path(path)?;
    let monitor_id = monitor_id.trim();

    if monitor_id.is_empty() {
        return Err(anyhow!("Monitor id cannot be empty"));
    }

    let _com = ComApartment::initialize()?;
    let wallpaper_path = wide_from_os_str(path.as_os_str());

    let desktop_wallpaper: IDesktopWallpaper =
        unsafe { CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL) }
            .map_err(|e| anyhow!("Failed to create Windows desktop wallpaper service: {}", e))?;
    let monitor_id = resolve_desktop_wallpaper_monitor_id(&desktop_wallpaper, monitor_id)?;
    let monitor_id = wide_from_str(&monitor_id);

    unsafe {
        desktop_wallpaper
            .SetWallpaper(PCWSTR(monitor_id.as_ptr()), PCWSTR(wallpaper_path.as_ptr()))
            .map_err(|e| anyhow!("Failed to set monitor wallpaper: {}", e))?;
    }

    Ok(())
}

fn resolve_desktop_wallpaper_monitor_id(
    desktop_wallpaper: &IDesktopWallpaper,
    requested_id: &str,
) -> Result<String> {
    let requested_id_wide = wide_from_str(requested_id);
    if unsafe { desktop_wallpaper.GetMonitorRECT(PCWSTR(requested_id_wide.as_ptr())) }.is_ok() {
        return Ok(requested_id.to_string());
    }

    let monitor_manager = MonitorManager::new()?;
    let requested_monitor = monitor_manager
        .get_monitors()
        .iter()
        .find(|monitor| monitor.id == requested_id)
        .cloned()
        .ok_or_else(|| anyhow!("Monitor '{}' was not found", requested_id))?;

    let monitor_count = unsafe { desktop_wallpaper.GetMonitorDevicePathCount() }
        .map_err(|e| anyhow!("Failed to enumerate desktop wallpaper monitors: {}", e))?;

    for index in 0..monitor_count {
        let desktop_monitor_id = unsafe { desktop_wallpaper.GetMonitorDevicePathAt(index) }
            .map(take_cotaskmem_string)
            .map_err(|e| anyhow!("Failed to read desktop wallpaper monitor id: {}", e))?;
        let desktop_monitor_id_wide = wide_from_str(&desktop_monitor_id);

        let Ok(rect) =
            (unsafe { desktop_wallpaper.GetMonitorRECT(PCWSTR(desktop_monitor_id_wide.as_ptr())) })
        else {
            continue;
        };

        if monitor_rect_matches(&requested_monitor, rect) {
            return Ok(desktop_monitor_id);
        }
    }

    Err(anyhow!(
        "Monitor '{}' was not found in Windows desktop wallpaper service",
        requested_id
    ))
}

fn monitor_rect_matches(monitor: &Monitor, rect: RECT) -> bool {
    rect.left == monitor.x
        && rect.top == monitor.y
        && rect.right - rect.left == monitor.width
        && rect.bottom - rect.top == monitor.height
}

fn take_cotaskmem_string(value: PWSTR) -> String {
    let ptr = value.0;
    let result = if ptr.is_null() {
        String::new()
    } else {
        let mut len = 0;
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    };

    unsafe {
        CoTaskMemFree(Some(ptr as *const _));
    }

    result
}

fn existing_wallpaper_path(path: &str) -> Result<&Path> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(anyhow!(
            "Saved Windows wallpaper file no longer exists: {}",
            path.display()
        ));
    }

    Ok(path)
}

fn wide_from_os_str(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn wide_from_str(value: &str) -> Vec<u16> {
    wide_from_os_str(OsStr::new(value))
}

struct ComApartment {
    should_uninitialize: bool,
}

impl ComApartment {
    fn initialize() -> Result<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

        if hr.is_ok() {
            return Ok(Self {
                should_uninitialize: true,
            });
        }

        if hr == RPC_E_CHANGED_MODE {
            return Ok(Self {
                should_uninitialize: false,
            });
        }

        hr.ok()
            .map_err(|e| anyhow!("Failed to initialize COM: {}", e))?;
        Ok(Self {
            should_uninitialize: true,
        })
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
