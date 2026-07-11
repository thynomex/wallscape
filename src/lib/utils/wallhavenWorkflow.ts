import type {
  StoreActionResult,
  WallhavenWallpaper,
  Wallpaper,
} from "$lib/types/wallpaper";
import { wallhavenResultForDetailWallpaper } from "$lib/utils/wallhavenDetail";

interface WallhavenDownloadActions {
  downloadWallhaven(wallpaper: WallhavenWallpaper): Promise<StoreActionResult<Wallpaper>>;
}

export interface DownloadedWallhavenWallpaper {
  wallpaper: Wallpaper;
  fallbackThumbnailSrc?: string;
}

export async function downloadWallhavenWallpaper(
  wallpaper: WallhavenWallpaper,
  actions: WallhavenDownloadActions,
): Promise<StoreActionResult<DownloadedWallhavenWallpaper>> {
  const result = await actions.downloadWallhaven(wallpaper);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      wallpaper: result.value,
      fallbackThumbnailSrc: wallpaper.thumbs.large,
    },
  };
}

export function downloadWallhavenDetailWallpaper(
  wallpaper: Wallpaper,
  results: WallhavenWallpaper[],
  actions: WallhavenDownloadActions,
): Promise<StoreActionResult<Wallpaper>> {
  const result = wallhavenResultForDetailWallpaper(results, wallpaper);

  if (!result) {
    return Promise.resolve({
      ok: false,
      error: "Wallhaven result is no longer available",
    });
  }

  return actions.downloadWallhaven(result);
}
