import type {
  MotionBgsWallpaper,
  StoreActionResult,
  Wallpaper,
} from "$lib/types/wallpaper";
import { motionBgsResultForDetailWallpaper } from "$lib/utils/motionbgsDetail";

interface MotionBgsDownloadActions {
  downloadMotionBgs(wallpaper: MotionBgsWallpaper): Promise<StoreActionResult<Wallpaper>>;
}

export interface DownloadedMotionBgsWallpaper {
  wallpaper: Wallpaper;
  fallbackThumbnailSrc?: string;
}

export async function downloadMotionBgsWallpaper(
  wallpaper: MotionBgsWallpaper,
  actions: MotionBgsDownloadActions,
): Promise<StoreActionResult<DownloadedMotionBgsWallpaper>> {
  const result = await actions.downloadMotionBgs(wallpaper);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      wallpaper: result.value,
      fallbackThumbnailSrc: wallpaper.thumbnailUrl,
    },
  };
}

export function downloadMotionBgsDetailWallpaper(
  wallpaper: Wallpaper,
  results: MotionBgsWallpaper[],
  actions: MotionBgsDownloadActions,
): Promise<StoreActionResult<Wallpaper>> {
  const result = motionBgsResultForDetailWallpaper(results, wallpaper);

  if (!result) {
    return Promise.resolve({
      ok: false,
      error: "MotionBGS result is no longer available",
    });
  }

  return actions.downloadMotionBgs(result);
}
