use std::path::Path;

use crate::windows_util::encode_wide;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub(crate) fn reveal_path_in_explorer(path: &Path) -> Result<(), String> {
    let explorer = encode_wide("explorer.exe");
    let parameters = encode_wide(&format!("/select,\"{}\"", path.display()));
    shell_execute(
        &explorer,
        Some(&parameters),
        "Windows could not open Explorer",
    )
}

pub(crate) fn open_url(url: &str) -> Result<(), String> {
    let url = encode_wide(url);
    shell_execute(&url, None, "Windows could not open the URL")
}

fn shell_execute(
    target: &[u16],
    parameters: Option<&[u16]>,
    failure_message: &str,
) -> Result<(), String> {
    let open = encode_wide("open");
    let parameters = parameters
        .map(|parameters| PCWSTR::from_raw(parameters.as_ptr()))
        .unwrap_or_else(PCWSTR::null);

    let result = unsafe {
        ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR::from_raw(open.as_ptr()),
            PCWSTR::from_raw(target.as_ptr()),
            parameters,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(failure_message.to_string());
    }

    Ok(())
}
