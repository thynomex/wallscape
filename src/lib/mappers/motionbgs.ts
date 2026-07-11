import type { MotionBgsWallpaper, Wallpaper } from "$lib/types/wallpaper";

export function motionBgsToWallpaper(wallpaper: MotionBgsWallpaper | null): Wallpaper | null {
  if (!wallpaper) return null;

  return {
    id: -Number.parseInt(wallpaper.id, 10) || -1,
    title: wallpaper.title,
    file_path: wallpaper.previewVideoUrl ?? wallpaper.url,
    thumbnail_path: wallpaper.thumbnailUrl,
    tags: ["motionbgs", wallpaper.quality, ...wallpaper.tags],
    width: wallpaper.width,
    height: wallpaper.height,
    fps: 60,
    duration_ms: 0,
    file_size_bytes: wallpaper.fileSize,
    media_type: "video",
    source: "motionbgs",
    source_id: wallpaper.id,
    is_favorite: false,
  };
}
