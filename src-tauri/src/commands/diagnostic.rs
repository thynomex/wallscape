use serde::Serialize;
use std::process::Command;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};

#[derive(Serialize)]
pub struct DiagnosticInfo {
    pub os_version: String,
    pub build_number: String,
    pub desktop_windows: Vec<WindowInfo>,
    pub running_wallpaper_apps: Vec<String>,
    pub antivirus_info: Option<String>,
    pub workerw_count: usize,
    pub progman_found: bool,
}

#[derive(Serialize)]
pub struct WindowInfo {
    pub hwnd: String,
    pub class_name: String,
    pub title: String,
    pub visible: bool,
}

#[tauri::command]
pub async fn collect_diagnostic_info() -> Result<DiagnosticInfo, String> {
    tracing::info!("Collecting diagnostic information for troubleshooting");

    let os_version = get_os_version();
    let build_number = get_build_number();
    let desktop_windows = enumerate_desktop_windows()?;
    let running_wallpaper_apps = detect_wallpaper_apps();
    let antivirus_info = detect_antivirus();

    let workerw_count = desktop_windows
        .iter()
        .filter(|w| w.class_name == "WorkerW")
        .count();
    let progman_found = desktop_windows.iter().any(|w| w.class_name == "Progman");

    let info = DiagnosticInfo {
        os_version,
        build_number,
        desktop_windows,
        running_wallpaper_apps,
        antivirus_info,
        workerw_count,
        progman_found,
    };

    tracing::info!(
        "Diagnostic info collected: OS={}, Build={}, WorkerW count={}, Progman found={}",
        info.os_version,
        info.build_number,
        info.workerw_count,
        info.progman_found
    );

    if !info.running_wallpaper_apps.is_empty() {
        tracing::warn!(
            "Detected potentially conflicting wallpaper applications: {}",
            info.running_wallpaper_apps.join(", ")
        );
    }

    Ok(info)
}

fn get_os_version() -> String {
    match Command::new("cmd").args(["/C", "ver"]).output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

fn get_build_number() -> String {
    match Command::new("cmd")
        .args(["/C", "wmic os get BuildNumber /value"])
        .output()
    {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            output_str
                .lines()
                .find(|line| line.starts_with("BuildNumber="))
                .and_then(|line| line.split('=').nth(1))
                .unwrap_or("Unknown")
                .trim()
                .to_string()
        }
        Err(_) => "Unknown".to_string(),
    }
}

fn enumerate_desktop_windows() -> Result<Vec<WindowInfo>, String> {
    let mut windows = Vec::new();

    unsafe {
        let windows_ptr = &mut windows as *mut Vec<WindowInfo>;
        EnumWindows(
            Some(enum_windows_callback),
            windows::Win32::Foundation::LPARAM(windows_ptr as isize),
        )
        .map_err(|e| format!("Failed to enumerate windows: {}", e))?;
    }

    Ok(windows)
}

unsafe extern "system" fn enum_windows_callback(
    hwnd: HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    let class_name = get_window_class_name(hwnd);

    // Only collect desktop-related windows
    if !class_name.contains("Progman")
        && !class_name.contains("WorkerW")
        && !class_name.contains("SHELLDLL_DefView")
    {
        return windows::Win32::Foundation::BOOL(1);
    }

    let title = get_window_title(hwnd);
    let visible = IsWindowVisible(hwnd).as_bool();

    windows.push(WindowInfo {
        hwnd: format!("{:?}", hwnd),
        class_name,
        title,
        visible,
    });

    windows::Win32::Foundation::BOOL(1)
}

unsafe fn get_window_class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut buffer);
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        String::new()
    }
}

unsafe fn get_window_title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = GetWindowTextW(hwnd, &mut buffer);
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        String::new()
    }
}

fn detect_wallpaper_apps() -> Vec<String> {
    let mut apps = Vec::new();

    // Check for known wallpaper application processes
    let known_apps = [
        "wallpaper32.exe",     // Wallpaper Engine (32-bit)
        "wallpaper64.exe",     // Wallpaper Engine (64-bit)
        "lively.exe",          // Lively Wallpaper
        "LivelyWallpaper.exe", // Lively Wallpaper (old name)
        "deskscapes.exe",      // DeskScapes
        "RainWallpaper.exe",   // RainWallpaper
    ];

    match Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for app in known_apps {
                if output_str.to_lowercase().contains(&app.to_lowercase()) {
                    apps.push(app.to_string());
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to check for running wallpaper apps: {}", e);
        }
    }

    apps
}

fn detect_antivirus() -> Option<String> {
    // Try to detect antivirus using Windows Security Center
    match Command::new("powershell")
        .args([
            "-Command",
            "Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntivirusProduct | Select-Object -ExpandProperty displayName",
        ])
        .output()
    {
        Ok(output) => {
            let av_list = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !av_list.is_empty() && av_list != "Windows Defender" {
                Some(av_list)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
