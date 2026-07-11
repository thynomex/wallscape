use super::desktop_bounds::{virtual_desktop_bounds, DesktopBounds};
use super::desktop_parent::DesktopParent;
use super::placement::client_size;
use super::window_lookup::{
    find_child_window, find_top_level_window, find_workerw_behind, has_extended_style,
    has_shell_defview,
};
use super::window_snapshot::{window_class_name, window_snapshot};
use super::workerw_search::WorkerWSearch;
use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{
    GetLastError, SetLastError, BOOL, ERROR_SUCCESS, HWND, LPARAM, WPARAM,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, IsWindowVisible, SendMessageTimeoutW, SMTO_NORMAL, WS_EX_NOREDIRECTIONBITMAP,
};

const WORKERW_DISCOVERY_ATTEMPTS: u32 = 40;

pub(super) fn find_or_create_desktop_parent() -> Result<DesktopParent> {
    find_or_create_desktop_parent_candidate()?.ok_or_else(workerw_initialization_error)
}

/// Find or create the WorkerW window for wallpaper injection.
fn find_or_create_desktop_parent_candidate() -> Result<Option<DesktopParent>> {
    use std::time::Instant;
    let start_discovery = Instant::now();
    tracing::info!("Finding WorkerW window for desktop injection");
    let desktop_bounds = virtual_desktop_bounds()?;
    tracing::info!(
        "Virtual desktop bounds for wallpaper placement: ({}, {})-({}, {}) {}x{}",
        desktop_bounds.x,
        desktop_bounds.y,
        desktop_bounds.right(),
        desktop_bounds.bottom(),
        desktop_bounds.width,
        desktop_bounds.height
    );

    unsafe {
        // Step 1: Find the Progman window.
        let progman = find_top_level_window("Progman")?
            .ok_or_else(|| anyhow!("Failed to find Progman window"))?;

        tracing::debug!("Found Progman window: {}", describe_window(progman));
        let raised_desktop = has_extended_style(progman, WS_EX_NOREDIRECTIONBITMAP.0 as isize);
        if raised_desktop {
            tracing::info!("Detected Windows raised desktop with layered SHELLDLL_DefView");
        }

        // Step 2: Send 0x052C message to spawn WorkerW window.
        // This is an undocumented Windows message that creates the WorkerW window.
        for (wparam, lparam) in [(0, 0), (0xD, 0), (0xD, 1)] {
            let mut result = 0;
            SetLastError(ERROR_SUCCESS);
            let returned = SendMessageTimeoutW(
                progman,
                0x052C, // Undocumented message to spawn WorkerW
                WPARAM(wparam),
                LPARAM(lparam),
                SMTO_NORMAL,
                1000, // 1 second timeout
                Some(&mut result),
            );
            if returned.0 == 0 {
                tracing::warn!(
                    "SendMessageTimeoutW 0x052C wparam={} lparam={} failed: last_error={:?}",
                    wparam,
                    lparam,
                    GetLastError()
                );
            } else {
                tracing::debug!(
                    "SendMessageTimeoutW 0x052C wparam={} lparam={} returned={} result={}",
                    wparam,
                    lparam,
                    returned.0,
                    result
                );
            }
        }

        tracing::debug!("Sent 0x052C messages to Progman");

        if raised_desktop {
            if let Some(parent) = raised_desktop_parent(progman, desktop_bounds)? {
                tracing::info!(
                    "Found raised desktop parent for injection: progman={:?}, shell_defview={:?}, workerw={:?}",
                    HWND(parent.hwnd as *mut _),
                    parent.shell_defview.map(|hwnd| HWND(hwnd as *mut _)),
                    parent.workerw.map(|hwnd| HWND(hwnd as *mut _))
                );
                return Ok(Some(parent));
            }

            tracing::warn!(
                "Raised desktop was detected, but the expected Progman child windows were not available; falling back to standard WorkerW discovery"
            );
        }

        // Step 3: Enumerate windows to find the correct WorkerW.
        // Try multiple times with small delays, as WorkerW may take a moment to appear.
        for attempt in 1..=WORKERW_DISCOVERY_ATTEMPTS {
            let mut search = WorkerWSearch::new(desktop_bounds);
            let search_ptr = &mut search as *mut WorkerWSearch;

            enumerate_desktop_hosts(search_ptr)?;

            if let Some(hwnd) = search.target_workerw {
                tracing::info!(
                    "Found WorkerW window for injection: {} (attempt {}, elapsed {:?})",
                    hwnd,
                    attempt,
                    start_discovery.elapsed()
                );
                if let Some(hwnd) =
                    select_wallpaper_parent("exact WorkerW", attempt, hwnd, desktop_bounds)
                {
                    return Ok(Some(DesktopParent::workerw(hwnd)));
                }
            }

            if let Some(hwnd) = find_workerw_after(search.icon_parent, desktop_bounds)? {
                tracing::info!(
                    "Found fallback WorkerW window for injection: {} (attempt {}, elapsed {:?})",
                    hwnd,
                    attempt,
                    start_discovery.elapsed()
                );
                if let Some(hwnd) = select_wallpaper_parent(
                    "FindWindowEx WorkerW fallback",
                    attempt,
                    hwnd,
                    desktop_bounds,
                ) {
                    return Ok(Some(DesktopParent::workerw(hwnd)));
                }
            }

            if let Some(hwnd) = search.fallback_workerw {
                tracing::info!(
                    "Found WorkerW fallback without desktop icons: {} (attempt {}, elapsed {:?})",
                    hwnd,
                    attempt,
                    start_discovery.elapsed()
                );
                if let Some(hwnd) = select_wallpaper_parent(
                    "WorkerW without SHELLDLL_DefView",
                    attempt,
                    hwnd,
                    desktop_bounds,
                ) {
                    return Ok(Some(DesktopParent::workerw(hwnd)));
                }
            }

            tracing::debug!(
                "WorkerW discovery attempt {} summary: {}",
                attempt,
                search.summary()
            );

            if attempt < WORKERW_DISCOVERY_ATTEMPTS {
                tracing::debug!(
                    "WorkerW not found on attempt {} (WorkerW count: {}), retrying...",
                    attempt,
                    search.workerw_count
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        if is_usable_wallpaper_parent(progman, desktop_bounds) {
            tracing::warn!(
                "WorkerW discovery failed after {} attempts; falling back to Progman desktop parent: {}",
                WORKERW_DISCOVERY_ATTEMPTS,
                describe_window(progman)
            );
            return Ok(Some(DesktopParent::progman_fallback(progman.0 as isize)));
        }

        tracing::warn!(
            "Could not find suitable WorkerW window after {} attempts",
            WORKERW_DISCOVERY_ATTEMPTS
        );
        Ok(None)
    }
}

fn describe_window(hwnd: HWND) -> String {
    window_snapshot(hwnd, window_class_name(hwnd), has_shell_defview(hwnd)).to_string()
}

fn workerw_initialization_error() -> anyhow::Error {
    anyhow!(
        "Could not initialize the Windows desktop wallpaper layer. Explorer did not expose a usable WorkerW host, and Progman was not usable as a fallback. Collect the wallpaper diagnostic log; this usually means Explorer's desktop window tree is in an unsupported state or another desktop customization process is still attached."
    )
}

/// Callback for EnumWindows to find the correct WorkerW window.
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let search = &mut *(lparam.0 as *mut WorkerWSearch);

    // Get the window class name.
    let class_name = window_class_name(hwnd);

    if class_name != "Progman" && class_name != "WorkerW" {
        return BOOL(1); // Continue enumeration
    }

    if class_name == "WorkerW" {
        search.workerw_count += 1;
    }

    let shell_view = find_child_window(hwnd, HWND(std::ptr::null_mut()), "SHELLDLL_DefView");
    let shell_view_hwnd = match &shell_view {
        Ok(Some(shell_view)) => Some(*shell_view),
        _ => None,
    };
    search.hosts.push(window_snapshot(
        hwnd,
        class_name.clone(),
        shell_view_hwnd.is_some(),
    ));

    tracing::debug!("Checking desktop host window: {}", describe_window(hwnd));

    // Check if this top-level window hosts the desktop icon view.
    // Depending on the Windows shell state, SHELLDLL_DefView can live under
    // either Progman or a WorkerW.
    match shell_view {
        Ok(Some(shell_view)) => {
            tracing::debug!(
                "Found SHELLDLL_DefView under class='{}', hwnd={:?}",
                class_name,
                hwnd
            );
            search.icon_parent = Some(hwnd.0 as isize);
            search.icon_parent_class = Some(class_name.clone());
            search.icon_view = Some(shell_view.0 as isize);

            // Get the next WorkerW window in z-order (the one behind the desktop icons).
            match find_workerw_behind(hwnd) {
                Ok(Some(target_workerw)) => {
                    let target_class = window_class_name(target_workerw);
                    tracing::debug!(
                        "Window behind DefView WorkerW: class='{}', hwnd={:?}",
                        target_class,
                        target_workerw
                    );
                    if target_class == "WorkerW" {
                        if !is_usable_wallpaper_parent(target_workerw, search.desktop_bounds) {
                            tracing::debug!(
                                "Rejected target WorkerW behind desktop icons because it is not usable: {:?}",
                                target_workerw
                            );
                            return BOOL(1);
                        }
                        tracing::info!("Found target WorkerW for injection: {:?}", target_workerw);
                        search.target_workerw = Some(target_workerw.0 as isize);
                        search.stopped_after_match = true;
                        SetLastError(ERROR_SUCCESS);
                        return BOOL(0); // Stop enumeration
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!("Failed to inspect window behind desktop icons: {}", error);
                }
            }
        }
        Ok(None) => {
            if class_name == "WorkerW" && search.fallback_workerw.is_none() {
                if is_usable_wallpaper_parent(hwnd, search.desktop_bounds) {
                    tracing::debug!("Found usable WorkerW without SHELLDLL_DefView: {:?}", hwnd);
                    search.fallback_workerw = Some(hwnd.0 as isize);
                } else {
                    tracing::debug!(
                        "Ignoring WorkerW without SHELLDLL_DefView because it is not usable: {:?}",
                        hwnd
                    );
                }
            }
        }
        Err(error) => {
            tracing::debug!("Failed to search for SHELLDLL_DefView: {}", error);
        }
    }

    BOOL(1) // Continue enumeration
}

fn find_workerw_after(
    icon_parent: Option<isize>,
    desktop_bounds: DesktopBounds,
) -> Result<Option<isize>> {
    let Some(icon_parent) = icon_parent else {
        return Ok(None);
    };

    let candidate = find_child_window(
        HWND(std::ptr::null_mut()),
        HWND(icon_parent as *mut _),
        "WorkerW",
    )?;

    match candidate {
        Some(hwnd) if is_usable_wallpaper_parent(hwnd, desktop_bounds) => Ok(Some(hwnd.0 as isize)),
        Some(hwnd) => {
            tracing::debug!(
                "Rejected FindWindowEx WorkerW fallback because it is not usable: {:?}",
                hwnd
            );
            Ok(None)
        }
        None => Ok(None),
    }
}

fn raised_desktop_parent(
    progman: HWND,
    desktop_bounds: DesktopBounds,
) -> Result<Option<DesktopParent>> {
    if !is_usable_wallpaper_parent(progman, desktop_bounds) {
        tracing::debug!(
            "Rejected raised desktop Progman parent because it is not usable: {:?}",
            progman
        );
        return Ok(None);
    }

    let shell_defview =
        match find_child_window(progman, HWND(std::ptr::null_mut()), "SHELLDLL_DefView")? {
            Some(hwnd) => hwnd,
            None => {
                tracing::debug!("Raised desktop Progman has no SHELLDLL_DefView child");
                return Ok(None);
            }
        };

    let workerw = match find_child_window(progman, HWND(std::ptr::null_mut()), "WorkerW")? {
        Some(hwnd) => hwnd,
        None => {
            tracing::debug!("Raised desktop Progman has no child WorkerW");
            return Ok(None);
        }
    };

    Ok(Some(DesktopParent::raised_desktop(
        progman.0 as isize,
        shell_defview.0 as isize,
        workerw.0 as isize,
    )))
}

fn select_wallpaper_parent(
    label: &str,
    attempt: u32,
    hwnd: isize,
    desktop_bounds: DesktopBounds,
) -> Option<isize> {
    let hwnd_handle = HWND(hwnd as *mut _);

    if is_usable_wallpaper_parent(hwnd_handle, desktop_bounds) {
        tracing::debug!("Selected {label} on attempt {attempt}: {:?}", hwnd_handle);
        Some(hwnd)
    } else {
        tracing::debug!(
            "Rejected {label} on attempt {attempt} because it is not usable: {:?}",
            hwnd_handle
        );
        None
    }
}

fn is_usable_wallpaper_parent(hwnd: HWND, desktop_bounds: DesktopBounds) -> bool {
    if !unsafe { IsWindowVisible(hwnd).as_bool() } {
        return false;
    }

    let Some((width, height)) = client_size(hwnd) else {
        return false;
    };

    if width < desktop_bounds.width || height < desktop_bounds.height {
        tracing::debug!(
            "Rejected wallpaper parent {:?} because its client size {}x{} is smaller than virtual desktop {}x{}",
            hwnd,
            width,
            height,
            desktop_bounds.width,
            desktop_bounds.height
        );
        return false;
    }

    true
}

fn enumerate_desktop_hosts(search_ptr: *mut WorkerWSearch) -> Result<()> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        match EnumWindows(Some(enum_windows_callback), LPARAM(search_ptr as isize)) {
            Ok(()) => Ok(()),
            Err(_) if (*search_ptr).stopped_after_match => Ok(()),
            Err(error) if error.code().0 == 0 => Ok(()),
            Err(error) => Err(anyhow!("{}", error)),
        }
    }
}
