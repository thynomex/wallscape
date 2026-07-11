import type { WallhavenWallpaper, Wallpaper } from "$lib/types/wallpaper";

export function wallhavenToWallpaper(wallpaper: WallhavenWallpaper | null): Wallpaper | null {
  if (!wallpaper) return null;

  return {
    id: -Number.parseInt(wallpaper.id, 36) || -1,
    title: `Wallhaven ${wallpaper.category} ${wallpaper.id}`,
    file_path: wallpaper.path,
    thumbnail_path: wallpaper.thumbs.large,
    tags: ["wallhaven", wallpaper.category, wallpaper.purity],
    width: wallpaper.dimension_x,
    height: wallpaper.dimension_y,
    fps: 0,
    duration_ms: 0,
    file_size_bytes: wallpaper.file_size,
    media_type: "image",
    source: "wallhaven",
    source_id: wallpaper.id,
    is_favorite: false,
  };
}
