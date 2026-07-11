import type { Wallpaper } from "$lib/types/wallpaper";

export type FavoriteRotationReason = "manual" | "scheduled" | "startup";
export type ImportSource = "selected" | "folder" | "dropped";

export function favoriteChangeMessage(isFavorite: boolean) {
  return isFavorite ? "Added to favorites" : "Removed from favorites";
}

export function favoriteRotationToggleMessage(enabled: boolean) {
  return enabled ? "Favorite rotation started" : "Favorite rotation stopped";
}

export function favoriteRotationResultMessage(reason: FavoriteRotationReason) {
  return reason === "startup" ? "Startup favorite applied" : "Favorite rotated";
}

export function removedWallpaperMessage(wallpaper: Wallpaper) {
  return `"${wallpaper.title}" removed`;
}

export function importFailureMessage(source: ImportSource, error: unknown) {
  return `Failed to import ${source === "folder" ? "folder" : "wallpapers"}: ${error}`;
}

export function brokenLibraryEntriesMessage(count: number) {
  return `${count} broken library entries found`;
}

export function collectionMembershipMessage(inCollection: boolean) {
  return inCollection ? "Added to collection" : "Removed from collection";
}
