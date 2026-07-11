import {
  listWallpapers,
  removeWallpaper as removeWallpaperApi,
  revealWallpaperInExplorer,
  searchWallpapers,
} from "$lib/api/wallpaperApi";
import { decrementCollectionCounts } from "$lib/stores/wallpaperOrganization";
import { runStoreTask, storeActionFailure } from "$lib/utils/storeTask";
import type {
  Collection,
  CollectionMembership,
  StoreActionResult,
  Wallpaper,
} from "$lib/types/wallpaper";

export interface WallpaperLibraryState {
  wallpapers: Wallpaper[];
  collections: Collection[];
  collectionMemberships: CollectionMembership[];
  selected: Wallpaper | null;
  featured: Wallpaper | null;
  loading: boolean;
  importStatus: string | null;
  importWarnings: string[];
  error: string | null;
}

export async function loadWallpapersIntoState(state: WallpaperLibraryState) {
  const result = await runStoreTask(state, "Failed to load wallpapers", listWallpapers);
  setWallpapersInState(state, result.ok ? result.value : []);
}

export async function searchWallpapersInState(
  state: WallpaperLibraryState,
  query: string,
) {
  if (!query.trim()) {
    await loadWallpapersIntoState(state);
    return;
  }

  const result = await runStoreTask(
    state,
    "Failed to search wallpapers",
    () => searchWallpapers(query),
  );
  setWallpapersInState(state, result.ok ? result.value : []);
}

export async function revealWallpaperInExplorerFromState(
  state: Pick<WallpaperLibraryState, "error">,
  wallpaper: Wallpaper,
): Promise<StoreActionResult> {
  if (wallpaper.id <= 0) {
    return { ok: false, error: "Only saved library wallpapers can be revealed in Explorer." };
  }

  try {
    await revealWallpaperInExplorer(wallpaper.id);
    return { ok: true, value: undefined };
  } catch (cause) {
    return storeActionFailure(state, "Failed to reveal wallpaper", cause);
  }
}

export async function removeWallpaperFromState(
  state: WallpaperLibraryState,
  wallpaper: Wallpaper,
): Promise<StoreActionResult<Wallpaper>> {
  if (wallpaper.id <= 0) {
    const error = "Only saved library wallpapers can be removed from the library.";
    state.error = error;
    return { ok: false, error };
  }

  state.importStatus = `Removing "${wallpaper.title}" from your library...`;
  state.importWarnings = [];

  const result = await runStoreTask(state, "Failed to remove wallpaper", async () => {
    const result = await removeWallpaperApi(wallpaper.id);
    state.wallpapers = state.wallpapers.filter((item) => item.id !== wallpaper.id);
    const removedCollectionIds = new Set(
      state.collectionMemberships
        .filter((item) => item.wallpaper_id === wallpaper.id)
        .map((item) => item.collection_id),
    );
    state.collectionMemberships = state.collectionMemberships.filter(
      (item) => item.wallpaper_id !== wallpaper.id,
    );
    state.collections = decrementCollectionCounts(
      state.collections,
      removedCollectionIds,
    );

    if (state.selected?.id === wallpaper.id) {
      state.selected = null;
    }

    if (state.featured?.id === wallpaper.id) {
      state.featured = state.wallpapers[0] ?? null;
    }

    state.importStatus = `"${wallpaper.title}" removed from the library.`;
    return result.wallpaper;
  });

  if (!result.ok) {
    state.importStatus = null;
  }

  return result;
}

export function replaceWallpaperInState(
  state: Pick<WallpaperLibraryState, "wallpapers" | "selected" | "featured">,
  updated: Wallpaper,
) {
  state.wallpapers = state.wallpapers.map((wallpaper) =>
    wallpaper.id === updated.id ? updated : wallpaper,
  );

  if (state.selected?.id === updated.id) {
    state.selected = updated;
  }

  if (state.featured?.id === updated.id) {
    state.featured = updated;
  }
}

function setWallpapersInState(
  state: Pick<WallpaperLibraryState, "wallpapers" | "featured">,
  wallpapers: Wallpaper[],
) {
  const featuredId = state.featured?.id;
  state.wallpapers = wallpapers;
  state.featured =
    wallpapers.find((wallpaper) => wallpaper.id === featuredId) ??
    wallpapers[0] ??
    null;
}
