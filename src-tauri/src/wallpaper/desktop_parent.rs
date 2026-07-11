use windows::Win32::Foundation::HWND;

#[derive(Clone, Copy, Debug)]
pub(super) enum DesktopParentKind {
    WorkerW,
    RaisedDesktop,
    ProgmanFallback,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DesktopParent {
    pub(super) hwnd: isize,
    pub(super) kind: DesktopParentKind,
    pub(super) shell_defview: Option<isize>,
    pub(super) workerw: Option<isize>,
}

impl DesktopParent {
    pub(super) fn workerw(hwnd: isize) -> Self {
        Self {
            hwnd,
            kind: DesktopParentKind::WorkerW,
            shell_defview: None,
            workerw: Some(hwnd),
        }
    }

    pub(super) fn raised_desktop(progman: isize, shell_defview: isize, workerw: isize) -> Self {
        Self {
            hwnd: progman,
            kind: DesktopParentKind::RaisedDesktop,
            shell_defview: Some(shell_defview),
            workerw: Some(workerw),
        }
    }

    pub(super) fn progman_fallback(hwnd: isize) -> Self {
        Self {
            hwnd,
            kind: DesktopParentKind::ProgmanFallback,
            shell_defview: None,
            workerw: None,
        }
    }

    pub(super) fn hwnd(self) -> HWND {
        HWND(self.hwnd as *mut _)
    }

    pub(super) fn requires_layered_child(self) -> bool {
        matches!(self.kind, DesktopParentKind::RaisedDesktop)
    }
}
