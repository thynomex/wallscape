import {
  rotateRandomFavoriteWallpaper,
  setWallpaperFavorite,
} from "$lib/api/wallpaperApi";
import type { StoreActionResult, Wallpaper } from "$lib/types/wallpaper";

export interface WallpaperFavoriteState {
  wallpapers: Wallpaper[];
  selected: Wallpaper | null;
  featured: Wallpaper | null;
  error: string | null;
}

export interface WallpaperFavoriteActions {
  replaceWallpaper: (wallpaper: Wallpaper) => void;
  loadBackupStatus: () => Promise<void>;
  loadPreviousWallpaperStatus: () => Promise<void>;
  loadWallpaperHistory: () => Promise<void>;
}

export async function setFavoriteInState(
  state: WallpaperFavoriteState,
  wallpaper: Wallpaper,
  isFavorite: boolean,
  actions: Pick<WallpaperFavoriteActions, "replaceWallpaper">,
): Promise<StoreActionResult<Wallpaper>> {
  const previous = wallpaper;
  const optimistic = { ...wallpaper, is_favorite: isFavorite };
  actions.replaceWallpaper(optimistic);

  if (wallpaper.id <= 0) {
    return { ok: true, value: optimistic };
  }

  try {
    const updated = await setWallpaperFavorite(wallpaper.id, isFavorite);
    actions.replaceWallpaper(updated);
    return { ok: true, value: updated };
  } catch (e) {
    actions.replaceWallpaper(previous);
    const error = `Failed to update favorite: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function rotateRandomFavoriteInState(
  state: WallpaperFavoriteState,
  actions: WallpaperFavoriteActions,
): Promise<StoreActionResult<Wallpaper>> {
  state.error = null;
  try {
    const wallpaper = await rotateRandomFavoriteWallpaper();
    actions.replaceWallpaper(wallpaper);
    state.selected = wallpaper;
    state.featured = wallpaper;
    await actions.loadBackupStatus();
    await actions.loadPreviousWallpaperStatus();
    await actions.loadWallpaperHistory();
    return { ok: true, value: wallpaper };
  } catch (e) {
    const error = `Failed to rotate favorite wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}
