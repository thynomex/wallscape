import type { MotionBgsWallpaper, Wallpaper } from "$lib/types/wallpaper";
import { motionBgsBaseSourceId } from "$lib/utils/motionbgsDownload";

export function motionBgsResultForDetailWallpaper(
  results: MotionBgsWallpaper[],
  wallpaper: Wallpaper,
) {
  const sourceId = motionBgsBaseSourceId(wallpaper.source_id);
  return sourceId
    ? results.find((item) => item.id === sourceId) ?? null
    : null;
}
