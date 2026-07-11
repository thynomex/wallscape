use super::window_snapshot::window_class_name;
use crate::windows_util::encode_wide;
use anyhow::{anyhow, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{SetLastError, ERROR_SUCCESS, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindow, GetWindowLongPtrW, GWL_EXSTYLE, GW_HWNDNEXT,
};

pub(super) fn has_shell_defview(hwnd: HWND) -> bool {
    find_child_window(hwnd, HWND(std::ptr::null_mut()), "SHELLDLL_DefView")
        .ok()
        .flatten()
        .is_some()
}

pub(super) fn has_extended_style(hwnd: HWND, style: isize) -> bool {
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    ex_style & style == style
}

pub(super) fn find_top_level_window(class_name: &str) -> Result<Option<HWND>> {
    let class_name = encode_wide(class_name);

    unsafe {
        SetLastError(ERROR_SUCCESS);
        hwnd_result_or_none(FindWindowW(
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::null(),
        ))
    }
}

pub(super) fn find_child_window(
    parent: HWND,
    child_after: HWND,
    class_name: &str,
) -> Result<Option<HWND>> {
    let class_name = encode_wide(class_name);

    unsafe {
        SetLastError(ERROR_SUCCESS);
        hwnd_result_or_none(FindWindowExW(
            parent,
            child_after,
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::null(),
        ))
    }
}

pub(super) fn find_workerw_behind(hwnd: HWND) -> Result<Option<HWND>> {
    let mut current = hwnd;

    for _ in 0..64 {
        let Some(next) = get_next_window(current)? else {
            return Ok(None);
        };

        if window_class_name(next) == "WorkerW" {
            return Ok(Some(next));
        }

        current = next;
    }

    Ok(None)
}

fn get_next_window(hwnd: HWND) -> Result<Option<HWND>> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        hwnd_result_or_none(GetWindow(hwnd, GW_HWNDNEXT))
    }
}

fn hwnd_result_or_none(result: windows::core::Result<HWND>) -> Result<Option<HWND>> {
    match result {
        Ok(hwnd) if hwnd.0.is_null() => Ok(None),
        Ok(hwnd) => Ok(Some(hwnd)),
        Err(error) if error.code().0 == 0 => Ok(None),
        Err(error) => Err(anyhow!("{}", error)),
    }
}
