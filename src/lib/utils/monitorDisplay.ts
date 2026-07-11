import type { Monitor, Wallpaper } from "$lib/types/wallpaper";

export function monitorTargetForWallpaper(
  wallpaper: Wallpaper,
  monitorId: string | null,
) {
  return wallpaper.file_path ? monitorId : null;
}

export function monitorLabel(monitors: Monitor[], monitorId: string) {
  const monitor = monitors.find((item) => item.id === monitorId);
  if (!monitor) return "display";

  return monitor.is_primary
    ? `primary display (${monitor.width}×${monitor.height})`
    : `${monitor.name || "display"} (${monitor.width}×${monitor.height})`;
}
