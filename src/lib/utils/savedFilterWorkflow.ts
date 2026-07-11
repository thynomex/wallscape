import type {
  SavedFilter,
  SavedFilterPayload,
  SavedFilterType,
  StoreActionResult,
} from "$lib/types/wallpaper";
import {
  localFilterPayload,
  localFilterQueryFromPayload,
  wallhavenFilterFieldsFromPayload,
  wallhavenFilterPayload,
  type WallhavenFilterFields,
} from "$lib/utils/savedFilterPayload";

interface SavedFilterActions {
  saveFilter(
    name: string,
    filterType: SavedFilterType,
    payload: SavedFilterPayload,
  ): Promise<StoreActionResult<SavedFilter>>;
  deleteSavedFilter(filter: SavedFilter): Promise<StoreActionResult>;
}

export function saveLocalSavedFilter(
  name: string,
  query: string,
  actions: Pick<SavedFilterActions, "saveFilter">,
): Promise<StoreActionResult> {
  return withoutValue(actions.saveFilter(name, "local", localFilterPayload(query)));
}

export function applyLocalSavedFilter(
  filter: SavedFilter,
  applyQuery: (query: string) => void | Promise<void>,
): Promise<void> {
  return Promise.resolve(applyQuery(localFilterQueryFromPayload(filter.payload)));
}

export function saveWallhavenSavedFilter(
  name: string,
  fields: WallhavenFilterFields,
  actions: Pick<SavedFilterActions, "saveFilter">,
): Promise<StoreActionResult> {
  return withoutValue(
    actions.saveFilter(name, "wallhaven", wallhavenFilterPayload(fields)),
  );
}

export function applyWallhavenSavedFilter(
  filter: SavedFilter,
  applyFields: (fields: WallhavenFilterFields) => void | Promise<void>,
): Promise<void> {
  return Promise.resolve(
    applyFields(wallhavenFilterFieldsFromPayload(filter.payload)),
  );
}

export function deleteSavedFilter(
  filter: SavedFilter,
  actions: Pick<SavedFilterActions, "deleteSavedFilter">,
): Promise<StoreActionResult> {
  return actions.deleteSavedFilter(filter);
}

async function withoutValue<T>(
  resultPromise: Promise<StoreActionResult<T>>,
): Promise<StoreActionResult> {
  const result = await resultPromise;
  return result.ok ? { ok: true, value: undefined } : result;
}
