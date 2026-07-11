import type {
  BrokenWallpaper,
  ImportProbe,
  StoreActionResult,
  ThumbnailRegenerationResult,
  Wallpaper,
} from "$lib/types/wallpaper";
import { prepareThumbnailRegenerationProbe } from "$lib/utils/importPreparation";
import {
  brokenLibraryEntriesMessage,
  removedWallpaperMessage,
} from "$lib/utils/toastMessages";

interface LibraryMaintenanceActions {
  removeWallpaper(wallpaper: Wallpaper): Promise<StoreActionResult<Wallpaper>>;
  revealInExplorer(wallpaper: Wallpaper): Promise<StoreActionResult>;
  regenerateThumbnail(
    wallpaper: Wallpaper,
    probe: ImportProbe,
  ): Promise<StoreActionResult<ThumbnailRegenerationResult>>;
  detectBrokenWallpapers(): Promise<StoreActionResult<BrokenWallpaper[]>>;
}

export interface ThumbnailRegenerationOutcome {
  wallpaper: Wallpaper;
  message: string;
}

export interface BrokenLibraryCheckOutcome {
  message: string;
  severity: "success" | "warning";
}

export interface RemovedWallpaperOutcome {
  wallpaper: Wallpaper;
  message: string;
}

export async function removeWallpaperFromLibrary(
  wallpaper: Wallpaper,
  actions: Pick<LibraryMaintenanceActions, "removeWallpaper">,
): Promise<StoreActionResult<RemovedWallpaperOutcome>> {
  const result = await actions.removeWallpaper(wallpaper);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      wallpaper: result.value,
      message: removedWallpaperMessage(wallpaper),
    },
  };
}

export function revealWallpaperInExplorer(
  wallpaper: Wallpaper,
  actions: Pick<LibraryMaintenanceActions, "revealInExplorer">,
): Promise<StoreActionResult> {
  return actions.revealInExplorer(wallpaper);
}

export async function regenerateWallpaperThumbnail(
  wallpaper: Wallpaper,
  actions: Pick<LibraryMaintenanceActions, "regenerateThumbnail">,
  options: { setStatus?: (status: string) => void } = {},
): Promise<StoreActionResult<ThumbnailRegenerationOutcome>> {
  const probe = await prepareThumbnailRegenerationProbe(wallpaper, {
    onStatus: options.setStatus,
  });
  const result = await actions.regenerateThumbnail(wallpaper, probe);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      wallpaper: result.value.wallpaper,
      message: "Thumbnail refreshed",
    },
  };
}

export async function checkLibraryFiles(
  actions: Pick<LibraryMaintenanceActions, "detectBrokenWallpapers">,
): Promise<StoreActionResult<BrokenLibraryCheckOutcome>> {
  const result = await actions.detectBrokenWallpapers();
  if (!result.ok) return result;

  return result.value.length
    ? {
        ok: true,
        value: {
          message: brokenLibraryEntriesMessage(result.value.length),
          severity: "warning",
        },
      }
    : {
        ok: true,
        value: {
          message: "No broken library files found",
          severity: "success",
        },
      };
}
