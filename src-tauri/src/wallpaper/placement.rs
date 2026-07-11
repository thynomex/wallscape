use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

pub(super) const ALL_MONITORS_SLOT: &str = "__all__";

#[derive(Clone, Debug)]
pub(super) struct PlayerPlacement {
    pub(super) slot: String,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl PlayerPlacement {
    pub(super) fn relative_to_parent(mut self, parent: HWND) -> Result<Self> {
        if self.slot == ALL_MONITORS_SLOT {
            if let Some((width, height)) = client_size(parent) {
                self.x = 0;
                self.y = 0;
                self.width = width;
                self.height = height;
                return Ok(self);
            }
        }

        let point = screen_to_client(parent, self.x, self.y)?;
        self.x = point.x;
        self.y = point.y;
        self.ensure_fits_parent(parent)?;
        Ok(self)
    }

    fn ensure_fits_parent(&self, parent: HWND) -> Result<()> {
        let Some((parent_width, parent_height)) = client_size(parent) else {
            return Err(anyhow!(
                "Selected desktop parent has no usable client area for wallpaper placement"
            ));
        };

        if self.x < 0
            || self.y < 0
            || self.x + self.width > parent_width
            || self.y + self.height > parent_height
        {
            return Err(anyhow!(
                "Selected desktop parent cannot cover wallpaper target '{}' at ({}, {}) {}x{} within parent client {}x{}",
                self.slot,
                self.x,
                self.y,
                self.width,
                self.height,
                parent_width,
                parent_height
            ));
        }

        Ok(())
    }
}

pub(super) fn client_size(hwnd: HWND) -> Option<(i32, i32)> {
    unsafe {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).ok()?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return None;
        }

        Some((width, height))
    }
}

fn screen_to_client(parent: HWND, x: i32, y: i32) -> Result<POINT> {
    let mut point = POINT { x, y };
    unsafe {
        if !ScreenToClient(parent, &mut point).as_bool() {
            return Err(anyhow!(
                "Failed to map screen point ({}, {}) to wallpaper parent client coordinates",
                x,
                y
            ));
        }
    }
    Ok(point)
}
