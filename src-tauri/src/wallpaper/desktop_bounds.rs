use super::monitor::MonitorManager;
use anyhow::{anyhow, Result};

#[derive(Clone, Copy, Debug)]
pub(super) struct DesktopBounds {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl DesktopBounds {
    pub(super) fn right(self) -> i32 {
        self.x + self.width
    }

    pub(super) fn bottom(self) -> i32 {
        self.y + self.height
    }
}

pub(super) fn virtual_desktop_bounds() -> Result<DesktopBounds> {
    let manager = MonitorManager::new()?;
    let monitors = manager.get_monitors();

    if monitors.is_empty() {
        return Err(anyhow!("No monitors were detected"));
    }

    let left = monitors.iter().map(|monitor| monitor.x).min().unwrap_or(0);
    let top = monitors.iter().map(|monitor| monitor.y).min().unwrap_or(0);
    let right = monitors
        .iter()
        .map(|monitor| monitor.x + monitor.width)
        .max()
        .unwrap_or(0);
    let bottom = monitors
        .iter()
        .map(|monitor| monitor.y + monitor.height)
        .max()
        .unwrap_or(0);

    let width = right - left;
    let height = bottom - top;

    if width <= 0 || height <= 0 {
        return Err(anyhow!("Detected monitors have invalid bounds"));
    }

    Ok(DesktopBounds {
        x: left,
        y: top,
        width,
        height,
    })
}
