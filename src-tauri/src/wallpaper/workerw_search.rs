use super::desktop_bounds::DesktopBounds;
use super::window_snapshot::WindowSnapshot;

pub(super) struct WorkerWSearch {
    pub(super) desktop_bounds: DesktopBounds,
    pub(super) icon_parent: Option<isize>,
    pub(super) icon_parent_class: Option<String>,
    pub(super) icon_view: Option<isize>,
    pub(super) target_workerw: Option<isize>,
    pub(super) fallback_workerw: Option<isize>,
    pub(super) workerw_count: usize,
    pub(super) stopped_after_match: bool,
    pub(super) hosts: Vec<WindowSnapshot>,
}

impl WorkerWSearch {
    pub(super) fn new(desktop_bounds: DesktopBounds) -> Self {
        Self {
            desktop_bounds,
            icon_parent: None,
            icon_parent_class: None,
            icon_view: None,
            target_workerw: None,
            fallback_workerw: None,
            workerw_count: 0,
            stopped_after_match: false,
            hosts: Vec::new(),
        }
    }

    pub(super) fn summary(&self) -> String {
        let icon_parent = match (self.icon_parent, self.icon_parent_class.as_deref()) {
            (Some(hwnd), Some(class_name)) => format!("Some({hwnd}, class='{class_name}')"),
            (Some(hwnd), None) => format!("Some({hwnd})"),
            (None, _) => "None".to_string(),
        };
        let icon_view = self
            .icon_view
            .map(|hwnd| hwnd.to_string())
            .unwrap_or_else(|| "None".to_string());
        let target_workerw = self
            .target_workerw
            .map(|hwnd| hwnd.to_string())
            .unwrap_or_else(|| "None".to_string());
        let fallback_workerw = self
            .fallback_workerw
            .map(|hwnd| hwnd.to_string())
            .unwrap_or_else(|| "None".to_string());
        let hosts = self
            .hosts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" | ");

        format!(
            "icon_parent={icon_parent}; icon_view={icon_view}; target_workerw={target_workerw}; fallback_workerw={fallback_workerw}; workerw_count={}; hosts=[{}]",
            self.workerw_count, hosts
        )
    }
}
