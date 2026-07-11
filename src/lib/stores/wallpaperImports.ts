import {
  detectBrokenWallpapers,
  importWallpaper as importWallpaperApi,
  importWallpapers as importWallpapersApi,
  regenerateWallpaperThumbnail,
  scanImportPaths,
} from "$lib/api/wallpaperApi";
import { formatBatchImportMessage } from "$lib/utils/importSummary";
import { runStoreTask, storeActionFailure } from "$lib/utils/storeTask";
import type {
  BatchImportItem,
  BatchImportResult,
  BrokenWallpaper,
  ImportProbe,
  ImportScanResult,
  StoreActionResult,
  ThumbnailRegenerationResult,
  Wallpaper,
} from "$lib/types/wallpaper";

export interface WallpaperImportState {
  wallpapers: Wallpaper[];
  featured: Wallpaper | null;
  loading: boolean;
  importing: boolean;
  importStatus: string | null;
  importWarnings: string[];
  error: string | null;
}

export interface WallpaperReloadAction {
  reloadWallpapers: () => Promise<void>;
}

export interface WallpaperReplaceAction {
  replaceWallpaper: (wallpaper: Wallpaper) => void;
}

export async function importWallpaperIntoState(
  state: WallpaperImportState,
  videoPath: string,
  probe: ImportProbe | undefined,
  actions: WallpaperReloadAction,
): Promise<StoreActionResult<Wallpaper>> {
  state.importing = true;
  state.importStatus = "Saving video to library...";
  state.importWarnings = [];
  try {
    const result = await runStoreTask(state, "Failed to import wallpaper", async () => {
      const imported = await importWallpaperApi(videoPath, probe);

      await actions.reloadWallpapers();
      state.importWarnings = imported.warnings;
      const importedFromList =
        state.wallpapers.find((wallpaper) => wallpaper.id === imported.wallpaper.id) ??
        imported.wallpaper;

      state.featured = importedFromList;
      state.importStatus = imported.warnings.length
        ? imported.duplicate
          ? "Skipped duplicate import."
          : "Imported with notes."
        : "Imported successfully.";
      return importedFromList;
    });

    if (!result.ok) state.importStatus = null;
    return result;
  } finally {
    state.importing = false;
  }
}

export function scanImportPathsForImport(paths: string[]): Promise<ImportScanResult> {
  return scanImportPaths(paths);
}

export async function importWallpapersIntoState(
  state: WallpaperImportState,
  items: BatchImportItem[],
  actions: WallpaperReloadAction,
): Promise<StoreActionResult<BatchImportResult>> {
  state.importing = true;
  state.importStatus = `Importing ${items.length} wallpaper${items.length === 1 ? "" : "s"}...`;
  state.importWarnings = [];
  try {
    const result = await runStoreTask(state, "Failed to import wallpapers", async () => {
      const imported = await importWallpapersApi(items);
      await actions.reloadWallpapers();

      state.importWarnings = [
        ...imported.warnings,
        ...imported.failed.map((failure) => `${failure.path}: ${failure.reason}`),
      ];
      state.featured =
        imported.imported[0] ??
        imported.duplicates[0] ??
        state.featured ??
        state.wallpapers[0] ??
        null;
      state.importStatus = formatBatchImportMessage(imported, { suffix: "." });
      return imported;
    });

    if (!result.ok) state.importStatus = null;
    return result;
  } finally {
    state.importing = false;
  }
}

export async function detectBrokenWallpapersInState(
  state: WallpaperImportState,
): Promise<StoreActionResult<BrokenWallpaper[]>> {
  state.importStatus = "Checking library files...";
  state.importWarnings = [];

  const result = await runStoreTask(state, "Failed to check library files", async () => {
    const broken = await detectBrokenWallpapers();
    state.importStatus = broken.length
      ? `${broken.length} broken library entr${broken.length === 1 ? "y" : "ies"} found.`
      : "No broken library files found.";
    state.importWarnings = broken
      .slice(0, 8)
      .map((item) => `${item.wallpaper.title}: ${item.reason}`);
    return broken;
  });

  if (!result.ok) {
    state.importStatus = null;
  }

  return result;
}

export async function regenerateThumbnailInState(
  state: WallpaperImportState,
  wallpaper: Wallpaper,
  probe: ImportProbe,
  actions: WallpaperReplaceAction,
): Promise<StoreActionResult<ThumbnailRegenerationResult>> {
  if (wallpaper.id <= 0 || wallpaper.media_type !== "video") {
    return {
      ok: false,
      error: "Thumbnail regeneration is only available for imported videos.",
    };
  }

  state.importStatus = `Regenerating "${wallpaper.title}" thumbnail...`;
  state.importWarnings = [];

  try {
    const result = await regenerateWallpaperThumbnail(wallpaper.id, probe);
    actions.replaceWallpaper(result.wallpaper);
    state.importWarnings = result.warnings;
    state.importStatus = `"${result.wallpaper.title}" thumbnail refreshed.`;
    return { ok: true, value: result };
  } catch (cause) {
    state.importStatus = null;
    return storeActionFailure(state, "Failed to regenerate thumbnail", cause);
  }
}
