import {
  getOriginalWallpaperBackup,
  getPreviousWallpaper,
  listWallpaperHistory,
  removeWallpaper as removeWallpaperApi,
  restoreOriginalWallpaper,
  restorePreviousWallpaper,
  setWallpaper,
  undoWallpaperHistory,
} from "$lib/api/wallpaperApi";
import type {
  StoreActionResult,
  Wallpaper,
  WallpaperBackupStatus,
  WallpaperHistoryEntry,
  WallpaperRestoreStatus,
} from "$lib/types/wallpaper";

export interface WallpaperLifecycleState {
  wallpapers: Wallpaper[];
  selected: Wallpaper | null;
  featured: Wallpaper | null;
  backup: WallpaperBackupStatus;
  previous: WallpaperRestoreStatus;
  history: WallpaperHistoryEntry[];
  importStatus: string | null;
  error: string | null;
}

export async function loadBackupStatusIntoState(state: WallpaperLifecycleState) {
  try {
    state.backup = await getOriginalWallpaperBackup();
  } catch (e) {
    console.warn("Failed to load wallpaper backup status:", e);
    state.backup = { path: null, can_restore: false };
  }
}

export async function loadPreviousWallpaperStatusIntoState(
  state: WallpaperLifecycleState,
) {
  try {
    state.previous = await getPreviousWallpaper();
  } catch (e) {
    console.warn("Failed to load previous wallpaper status:", e);
    state.previous = { path: null, can_restore: false };
  }
}

export async function loadWallpaperHistoryIntoState(state: WallpaperLifecycleState) {
  try {
    state.history = await listWallpaperHistory(12);
  } catch (e) {
    console.warn("Failed to load wallpaper history:", e);
    state.history = [];
  }
}

export async function applyWallpaperInState(
  state: WallpaperLifecycleState,
  wallpaper: Wallpaper,
  monitorId: string | null = null,
): Promise<StoreActionResult> {
  state.error = null;
  if (!wallpaper.file_path) {
    return { ok: true, value: undefined };
  }

  try {
    await setWallpaper(wallpaper.file_path, monitorId);
    state.selected = wallpaper;
    await loadBackupStatusIntoState(state);
    await loadPreviousWallpaperStatusIntoState(state);
    await loadWallpaperHistoryIntoState(state);
    return { ok: true, value: undefined };
  } catch (e) {
    const error = `Failed to set wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function restorePreviousWallpaperInState(
  state: WallpaperLifecycleState,
): Promise<StoreActionResult> {
  state.error = null;
  try {
    const previousPath = state.previous.path;
    await restorePreviousWallpaper();
    await loadPreviousWallpaperStatusIntoState(state);
    await loadWallpaperHistoryIntoState(state);
    state.selected = previousPath
      ? state.wallpapers.find((wallpaper) => wallpaper.file_path === previousPath) ?? null
      : null;
    return { ok: true, value: undefined };
  } catch (e) {
    const error = `Failed to restore previous wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function cancelAppliedWallpaperInState(
  state: WallpaperLifecycleState,
  fallbackWallpaper?: Wallpaper,
): Promise<StoreActionResult> {
  state.error = null;

  const canceledWallpaper =
    state.selected && state.selected.id > 0
      ? state.selected
      : fallbackWallpaper && fallbackWallpaper.id > 0
        ? fallbackWallpaper
        : null;
  const previousPath = state.previous.path;

  try {
    await restorePreviousWallpaper();

    if (canceledWallpaper && canceledWallpaper.file_path !== previousPath) {
      await removeWallpaperApi(canceledWallpaper.id);
      state.wallpapers = state.wallpapers.filter(
        (wallpaper) => wallpaper.id !== canceledWallpaper.id,
      );
    }

    await loadPreviousWallpaperStatusIntoState(state);
    await loadWallpaperHistoryIntoState(state);

    const restoredWallpaper = previousPath
      ? state.wallpapers.find((wallpaper) => wallpaper.file_path === previousPath) ?? null
      : null;
    state.selected = restoredWallpaper;

    if (state.featured?.id === canceledWallpaper?.id || !state.featured) {
      state.featured = restoredWallpaper ?? state.wallpapers[0] ?? null;
    }

    state.importStatus = canceledWallpaper
      ? `"${canceledWallpaper.title}" canceled and removed from your library.`
      : "Previous wallpaper restored.";
    return { ok: true, value: undefined };
  } catch (e) {
    const error = `Failed to cancel wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function restoreOriginalWallpaperInState(
  state: WallpaperLifecycleState,
): Promise<StoreActionResult> {
  state.error = null;
  try {
    await restoreOriginalWallpaper();
    await loadBackupStatusIntoState(state);
    await loadWallpaperHistoryIntoState(state);
    state.selected = null;
    return { ok: true, value: undefined };
  } catch (e) {
    const error = `Failed to restore original wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function undoWallpaperHistoryInState(
  state: WallpaperLifecycleState,
): Promise<StoreActionResult<WallpaperHistoryEntry>> {
  state.error = null;

  try {
    const restored = await undoWallpaperHistory();
    await loadPreviousWallpaperStatusIntoState(state);
    await loadWallpaperHistoryIntoState(state);
    state.selected =
      state.wallpapers.find((wallpaper) => wallpaper.file_path === restored.file_path) ??
      null;
    return { ok: true, value: restored };
  } catch (e) {
    const error = `Failed to undo wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}
