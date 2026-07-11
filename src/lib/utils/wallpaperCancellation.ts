import type {
  Wallpaper,
  WallpaperRestoreStatus,
} from "$lib/types/wallpaper";

export function canCancelAppliedWallpaper(
  selected: Wallpaper | null,
  previous: WallpaperRestoreStatus,
  candidate: Wallpaper | null,
) {
  if (!candidate || !previous.can_restore || !selected) return false;

  if (selected.id === candidate.id) return true;
  if (selected.file_path && selected.file_path === candidate.file_path) return true;

  return Boolean(
    selected.source &&
      candidate.source &&
      selected.source === candidate.source &&
      selected.source_id &&
      selected.source_id === candidate.source_id,
  );
}
