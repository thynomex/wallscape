import {
  listWallpapers,
  removeWallpaper as removeWallpaperApi,
  revealWallpaperInExplorer,
  searchWallpapers,
} from "$lib/api/wallpaperApi";
import { decrementCollectionCounts } from "$lib/stores/wallpaperOrganization";
import { latestRequest } from "$lib/stores/latestRequest";
import type {
  Collection,
  CollectionMembership,
  StoreActionResult,
  Wallpaper,
} from "$lib/types/wallpaper";

export interface WallpaperLibraryState {
  wallpapers: Wallpaper[];
  searchResults: Wallpaper[] | null;
  searchLoading: boolean;
  searchError: string | null;
  collections: Collection[];
  collectionMemberships: CollectionMembership[];
  selected: Wallpaper | null;
  featured: Wallpaper | null;
  loading: boolean;
  importStatus: string | null;
  importWarnings: string[];
  error: string | null;
}

const beginLoad = latestRequest();
const beginSearch = latestRequest();
// Keep late reads from reintroducing an entry deleted during their request.
const removedIds = new WeakMap<object, Set<number>>();

export async function loadWallpapersIntoState(state: WallpaperLibraryState) {
  const isCurrent = beginLoad(state);
  const libraryAtStart = new Map(state.wallpapers.map((wallpaper) => [wallpaper.id, wallpaper]));
  state.loading = true;
  state.error = null;
  try {
    const list = await listWallpapers();
    if (!isCurrent()) return;
    // Retain records changed or added locally since the read began, while still
    // accepting newly imported/downloaded records from the response.
    const changed = new Map(state.wallpapers
      .filter((wallpaper) => libraryAtStart.get(wallpaper.id) !== wallpaper)
      .map((wallpaper) => [wallpaper.id, wallpaper]));
    const merged = list.map((wallpaper) => {
      const current = changed.get(wallpaper.id) ?? wallpaper;
      changed.delete(wallpaper.id);
      return current;
    });
    setWallpapersInState(state, withoutRemovedWallpapers(state, [...merged, ...changed.values()]));
  } catch (e) {
    if (!isCurrent()) return;
    const error = `Failed to load wallpapers: ${e}`;
    state.error = error;
    console.error(e);
  } finally {
    if (isCurrent()) state.loading = false;
  }
}

export async function searchWallpapersInState(
  state: WallpaperLibraryState,
  query: string,
) {
  const isCurrent = beginSearch(state);
  const libraryAtStart = state.wallpapers;
  state.searchError = null;
  if (!query.trim()) {
    state.searchResults = null;
    state.searchLoading = false;
    return;
  }

  state.searchLoading = true;
  try {
    const list = await searchWallpapers(query);
    if (!isCurrent()) return;
    const current = new Map(state.wallpapers.map((wallpaper) => [wallpaper.id, wallpaper]));
    state.searchResults = withoutRemovedWallpapers(state, list)
      // A refresh may have removed entries since this search began. Only
      // reconcile this response; absence in a read is not a permanent deletion.
      .filter((wallpaper) => state.wallpapers === libraryAtStart || current.has(wallpaper.id))
      .map((wallpaper) => current.get(wallpaper.id) ?? wallpaper);
  } catch (e) {
    if (!isCurrent()) return;
    const error = `Failed to search wallpapers: ${e}`;
    state.searchError = error;
    console.error(e);
    state.searchResults = [];
  } finally {
    if (isCurrent()) state.searchLoading = false;
  }
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
  } catch (e) {
    const error = `Failed to reveal wallpaper: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
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

  state.loading = true;
  state.error = null;
  state.importStatus = `Removing "${wallpaper.title}" from your library...`;
  state.importWarnings = [];

  try {
    const result = await removeWallpaperApi(wallpaper.id);
    forgetWallpaperInState(state, wallpaper.id);

    state.importStatus = `"${wallpaper.title}" removed from the library.`;
    return { ok: true, value: result.wallpaper };
  } catch (e) {
    const error = `Failed to remove wallpaper: ${e}`;
    state.error = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  } finally {
    state.loading = false;
  }
}

export function replaceWallpaperInState(
  state: Pick<WallpaperLibraryState, "wallpapers" | "searchResults" | "selected" | "featured">,
  updated: Wallpaper,
) {
  state.wallpapers = state.wallpapers.map((wallpaper) =>
    wallpaper.id === updated.id ? updated : wallpaper,
  );
  state.searchResults = state.searchResults?.map((wallpaper) =>
    wallpaper.id === updated.id ? updated : wallpaper,
  ) ?? null;

  if (state.selected?.id === updated.id) {
    state.selected = updated;
  }

  if (state.featured?.id === updated.id) {
    state.featured = updated;
  }
}

export type WallpaperRemovalState = Pick<
  WallpaperLibraryState,
  "wallpapers" | "searchResults" | "collections" | "collectionMemberships" | "selected" | "featured"
>;

/** Shared by library removal and cancellation after the backend confirms deletion. */
export function forgetWallpaperInState(state: WallpaperRemovalState, id: number) {
  const removed = removedIds.get(state) ?? new Set<number>();
  removed.add(id);
  removedIds.set(state, removed);
  state.wallpapers = state.wallpapers.filter((item) => item.id !== id);
  state.searchResults = state.searchResults?.filter((item) => item.id !== id) ?? null;
  const collectionIds = new Set(
    state.collectionMemberships
      .filter((item) => item.wallpaper_id === id)
      .map((item) => item.collection_id),
  );
  state.collectionMemberships = state.collectionMemberships.filter(
    (item) => item.wallpaper_id !== id,
  );
  state.collections = decrementCollectionCounts(state.collections, collectionIds);
  if (state.selected?.id === id) state.selected = null;
  if (state.featured?.id === id) state.featured = state.wallpapers[0] ?? null;
}

function withoutRemovedWallpapers(state: object, wallpapers: Wallpaper[]) {
  const removed = removedIds.get(state);
  return removed ? wallpapers.filter((wallpaper) => !removed.has(wallpaper.id)) : wallpapers;
}

function setWallpapersInState(
  state: Pick<WallpaperLibraryState, "wallpapers" | "searchResults" | "selected" | "featured">,
  wallpapers: Wallpaper[],
) {
  const featuredId = state.featured?.id;
  const byId = new Map(wallpapers.map((wallpaper) => [wallpaper.id, wallpaper]));
  state.wallpapers = wallpapers;
  state.searchResults = state.searchResults?.flatMap((wallpaper) => {
    const current = byId.get(wallpaper.id);
    return current ? [current] : [];
  }) ?? null;
  if (state.selected && state.selected.id > 0) {
    state.selected = byId.get(state.selected.id) ?? null;
  }
  state.featured =
    wallpapers.find((wallpaper) => wallpaper.id === featuredId) ??
    wallpapers[0] ??
    null;
}
