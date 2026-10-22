import {
  detectBrokenWallpapers,
  importWallpaper as importWallpaperApi,
  importWallpapers as importWallpapersApi,
  regenerateWallpaperThumbnail,
  scanImportPaths,
} from "$lib/api/wallpaperApi";
import { formatBatchImportMessage } from "$lib/utils/importSummary";
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
  state.loading = true;
  state.importing = true;
  state.importStatus = "Saving video to library...";
  state.importWarnings = [];
  state.error = null;

  try {
    const result = await importWallpaperApi(videoPath, probe);

    await actions.reloadWallpapers();
    state.importWarnings = result.warnings;
    const importedFromList =
      state.wallpapers.find((wallpaper) => wallpaper.id === result.wallpaper.id) ??
      result.wallpaper;

    state.featured = importedFromList;
    state.importStatus = result.warnings.length
      ? result.duplicate
        ? "Skipped duplicate import."
        : "Imported with notes."
      : "Imported successfully.";
    return { ok: true, value: importedFromList };
  } catch (e) {
    const error = `Failed to import wallpaper: ${e}`;
    state.error = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  } finally {
    state.loading = false;
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
  state.loading = true;
  state.importing = true;
  state.importStatus = `Importing ${items.length} wallpaper${items.length === 1 ? "" : "s"}...`;
  state.importWarnings = [];
  state.error = null;

  try {
    const result = await importWallpapersApi(items);
    await actions.reloadWallpapers();

    state.importWarnings = [
      ...result.warnings,
      ...result.failed.map((failure) => `${failure.path}: ${failure.reason}`),
    ];
    state.featured =
      result.imported[0] ??
      result.duplicates[0] ??
      state.featured ??
      state.wallpapers[0] ??
      null;
    state.importStatus = formatBatchImportMessage(result, { suffix: "." });

    return { ok: true, value: result };
  } catch (e) {
    const error = `Failed to import wallpapers: ${e}`;
    state.error = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  } finally {
    state.loading = false;
    state.importing = false;
  }
}

export async function detectBrokenWallpapersInState(
  state: WallpaperImportState,
): Promise<StoreActionResult<BrokenWallpaper[]>> {
  state.loading = true;
  state.error = null;
  state.importStatus = "Checking library files...";
  state.importWarnings = [];

  try {
    const broken = await detectBrokenWallpapers();
    state.importStatus = broken.length
      ? `${broken.length} broken library entr${broken.length === 1 ? "y" : "ies"} found.`
      : "No broken library files found.";
    state.importWarnings = broken
      .slice(0, 8)
      .map((item) => `${item.wallpaper.title}: ${item.reason}`);
    return { ok: true, value: broken };
  } catch (e) {
    const error = `Failed to check library files: ${e}`;
    state.error = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  } finally {
    state.loading = false;
  }
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
  } catch (e) {
    const error = `Failed to regenerate thumbnail: ${e}`;
    state.error = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  }
}
