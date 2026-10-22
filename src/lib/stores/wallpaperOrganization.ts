import {
  createCollection as createCollectionApi,
  deleteCollection as deleteCollectionApi,
  deleteSavedFilter as deleteSavedFilterApi,
  listCollectionMemberships,
  listCollections,
  listSavedFilters,
  saveFilter as saveFilterApi,
  setCollectionMembership as setCollectionMembershipApi,
} from "$lib/api/wallpaperApi";
import type {
  Collection,
  CollectionMembership,
  SavedFilter,
  SavedFilterPayload,
  SavedFilterType,
  StoreActionResult,
  Wallpaper,
} from "$lib/types/wallpaper";

export interface WallpaperOrganizationState {
  collections: Collection[];
  collectionMemberships: CollectionMembership[];
  savedFilters: SavedFilter[];
  error: string | null;
}

export async function loadOrganizationState(state: WallpaperOrganizationState) {
  try {
    const [collections, memberships, savedFilters] = await Promise.all([
      listCollections(),
      listCollectionMemberships(),
      listSavedFilters(),
    ]);

    state.collections = collections;
    state.collectionMemberships = memberships;
    state.savedFilters = savedFilters;
  } catch (e) {
    console.warn("Failed to load organization data:", e);
    state.collections = [];
    state.collectionMemberships = [];
    state.savedFilters = [];
  }
}

export async function createCollectionInState(
  state: WallpaperOrganizationState,
  name: string,
): Promise<StoreActionResult<Collection>> {
  try {
    const collection = await createCollectionApi(name);
    state.collections = upsertCollection(state.collections, collection);
    return { ok: true, value: collection };
  } catch (e) {
    const error = `Failed to save collection: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function deleteCollectionFromState(
  state: WallpaperOrganizationState,
  collection: Collection,
): Promise<StoreActionResult> {
  try {
    await deleteCollectionApi(collection.id);
    state.collections = state.collections.filter((item) => item.id !== collection.id);
    state.collectionMemberships = state.collectionMemberships.filter(
      (item) => item.collection_id !== collection.id,
    );
    return { ok: true, value: undefined };
  } catch (e) {
    const error = `Failed to delete collection: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function setCollectionMembershipInState(
  state: WallpaperOrganizationState,
  collection: Collection,
  wallpaper: Wallpaper,
  inCollection: boolean,
): Promise<StoreActionResult<Collection>> {
  if (wallpaper.id <= 0) {
    return {
      ok: false,
      error: "Only saved library wallpapers can be added to collections.",
    };
  }

  try {
    const updated = await setCollectionMembershipApi(
      collection.id,
      wallpaper.id,
      inCollection,
    );
    state.collections = upsertCollection(state.collections, updated);
    state.collectionMemberships = updateMemberships(
      state.collectionMemberships,
      collection.id,
      wallpaper.id,
      inCollection,
    );
    return { ok: true, value: updated };
  } catch (e) {
    const error = `Failed to update collection: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function saveFilterInState(
  state: WallpaperOrganizationState,
  name: string,
  filterType: SavedFilterType,
  payload: SavedFilterPayload,
): Promise<StoreActionResult<SavedFilter>> {
  try {
    const saved = await saveFilterApi(name, filterType, payload);
    state.savedFilters = upsertSavedFilter(state.savedFilters, saved);
    return { ok: true, value: saved };
  } catch (e) {
    const error = `Failed to save filter: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export async function deleteSavedFilterFromState(
  state: WallpaperOrganizationState,
  filter: SavedFilter,
): Promise<StoreActionResult> {
  try {
    await deleteSavedFilterApi(filter.id);
    state.savedFilters = state.savedFilters.filter((item) => item.id !== filter.id);
    return { ok: true, value: undefined };
  } catch (e) {
    const error = `Failed to delete filter: ${e}`;
    state.error = error;
    console.error(e);
    return { ok: false, error };
  }
}

export function decrementCollectionCounts(
  collections: Collection[],
  collectionIds: Set<number>,
) {
  return collections.map((collection) => {
    if (!collectionIds.has(collection.id)) return collection;

    return {
      ...collection,
      wallpaper_count: Math.max(0, collection.wallpaper_count - 1),
    };
  });
}

function upsertCollection(collections: Collection[], updated: Collection) {
  return sortCollections([
    ...collections.filter((collection) => collection.id !== updated.id),
    updated,
  ]);
}

function upsertSavedFilter(filters: SavedFilter[], updated: SavedFilter) {
  return sortSavedFilters([
    ...filters.filter((filter) => filter.id !== updated.id),
    updated,
  ]);
}

function sortCollections(collections: Collection[]) {
  return [...collections].sort((a, b) => a.name.localeCompare(b.name));
}

function sortSavedFilters(filters: SavedFilter[]) {
  return [...filters].sort((a, b) => {
    const typeSort = a.filter_type.localeCompare(b.filter_type);
    return typeSort || a.name.localeCompare(b.name);
  });
}

function updateMemberships(
  memberships: CollectionMembership[],
  collectionId: number,
  wallpaperId: number,
  inCollection: boolean,
) {
  const without = memberships.filter(
    (item) => item.collection_id !== collectionId || item.wallpaper_id !== wallpaperId,
  );

  if (!inCollection) return without;

  return [
    ...without,
    {
      collection_id: collectionId,
      wallpaper_id: wallpaperId,
    },
  ];
}
