// Core wallpaper engine module
mod desktop_bounds;
mod desktop_host;
mod desktop_parent;
pub mod engine;
pub mod monitor;
mod placement;
pub mod runtime;
pub mod system;
pub mod video;
mod window_lookup;
mod window_snapshot;
mod workerw_search;

pub use engine::{FitMode, PlaybackOptions, WallpaperEngine};
pub use monitor::MonitorManager;
pub use runtime::{WallpaperRuntime, WallpaperTarget};
pub use system::{get_desktop_wallpaper, set_desktop_wallpaper, set_desktop_wallpaper_for_monitor};
pub use video::VideoPlayer;
