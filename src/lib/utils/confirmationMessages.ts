import type { Collection, Wallpaper } from "$lib/types/wallpaper";

export function removeWallpaperConfirmationMessage(wallpaper: Wallpaper) {
  return `Remove "${wallpaper.title}" from the Wallscape library?\n\nThis only removes it from Wallscape. The original video file stays on disk.`;
}

export function deleteCollectionConfirmationMessage(collection: Collection) {
  return `Delete "${collection.name}"?\n\nWallpapers stay in your library.`;
}
