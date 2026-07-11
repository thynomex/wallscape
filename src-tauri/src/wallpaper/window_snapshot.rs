use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetClientRect, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible,
};

pub(super) struct WindowSnapshot {
    hwnd: isize,
    class_name: String,
    visible: bool,
    rect: Option<RECT>,
    client_rect: Option<RECT>,
    pid: u32,
    process_path: Option<String>,
    has_shell_defview: bool,
}

pub(super) fn window_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);

        if len > 0 {
            String::from_utf16_lossy(&class_name[..len as usize])
        } else {
            "<unknown>".to_string()
        }
    }
}

pub(super) fn window_snapshot(
    hwnd: HWND,
    class_name: String,
    has_shell_defview: bool,
) -> WindowSnapshot {
    let (pid, process_path) = window_process(hwnd);

    WindowSnapshot {
        hwnd: hwnd.0 as isize,
        class_name,
        visible: unsafe { IsWindowVisible(hwnd).as_bool() },
        rect: window_rect(hwnd),
        client_rect: raw_client_rect(hwnd),
        pid,
        process_path,
        has_shell_defview,
    }
}

fn window_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        Some(rect)
    }
}

fn raw_client_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).ok()?;
        Some(rect)
    }
}

fn format_rect(rect: RECT) -> String {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    format!(
        "({}, {})-({}, {}) {}x{}",
        rect.left, rect.top, rect.right, rect.bottom, width, height
    )
}

fn window_process(hwnd: HWND) -> (u32, Option<String>) {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    (pid, process_path(pid))
}

fn process_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL(0), pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);

        result
            .ok()
            .map(|_| String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

impl std::fmt::Display for WindowSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rect = self
            .rect
            .map(format_rect)
            .unwrap_or_else(|| "<unavailable>".to_string());
        let client = self
            .client_rect
            .map(format_rect)
            .unwrap_or_else(|| "<unavailable>".to_string());
        let process = self.process_path.as_deref().unwrap_or("<unknown>");

        write!(
            formatter,
            "hwnd={} class='{}' visible={} rect={} client={} pid={} process='{}' has_shell_defview={}",
            self.hwnd,
            self.class_name,
            self.visible,
            rect,
            client,
            self.pid,
            process,
            self.has_shell_defview
        )
    }
}
