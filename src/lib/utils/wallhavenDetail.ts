import type { WallhavenWallpaper, Wallpaper } from "$lib/types/wallpaper";

export function wallhavenResultForDetailWallpaper(
  results: WallhavenWallpaper[],
  wallpaper: Wallpaper,
) {
  return wallpaper.source_id
    ? results.find((item) => item.id === wallpaper.source_id) ?? null
    : null;
}
