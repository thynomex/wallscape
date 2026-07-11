use anyhow::Result;
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};

/// Monitor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_primary: bool,
}

/// Manages multiple monitors for wallpaper display
pub struct MonitorManager {
    monitors: Vec<Monitor>,
}

impl MonitorManager {
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            monitors: Vec::new(),
        };
        manager.refresh()?;
        Ok(manager)
    }

    /// Enumerate all monitors
    pub fn refresh(&mut self) -> Result<()> {
        self.monitors.clear();

        unsafe {
            // Enumerate all display monitors
            let _ = EnumDisplayMonitors(
                HDC(std::ptr::null_mut()),
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut self.monitors as *mut Vec<Monitor> as isize),
            );
        }

        tracing::info!("Detected {} monitor(s)", self.monitors.len());
        for monitor in &self.monitors {
            tracing::debug!(
                "  Monitor: {} ({}x{} at {}, {})",
                monitor.name,
                monitor.width,
                monitor.height,
                monitor.x,
                monitor.y
            );
        }

        Ok(())
    }

    pub fn get_monitors(&self) -> &[Monitor] {
        &self.monitors
    }
}

/// Windows callback for monitor enumeration
unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<Monitor>);

    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(hmonitor, &mut monitor_info.monitorInfo as *mut _ as *mut _).as_bool() {
        let rect = monitor_info.monitorInfo.rcMonitor;
        let is_primary = monitor_info.monitorInfo.dwFlags & 0x00000001 != 0; // MONITORINFOF_PRIMARY

        // Extract monitor name from wide string
        let name = String::from_utf16_lossy(
            &monitor_info
                .szDevice
                .iter()
                .take_while(|&&c| c != 0)
                .copied()
                .collect::<Vec<u16>>(),
        );

        let monitor = Monitor {
            id: if name.is_empty() {
                format!("{:?}", hmonitor)
            } else {
                name.clone()
            },
            name,
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            is_primary,
        };

        monitors.push(monitor);
    }

    BOOL(1) // Continue enumeration
}
