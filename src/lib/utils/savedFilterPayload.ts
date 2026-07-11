import type { SavedFilterPayload } from "$lib/types/wallpaper";

export interface WallhavenFilterFields {
  query: string;
  categories: string;
  purity: string;
  sorting: string;
  atleast: string;
  ratios: string;
}

export function localFilterPayload(query: string): SavedFilterPayload {
  return {
    query: query.trim(),
  };
}

export function localFilterQueryFromPayload(payload: SavedFilterPayload) {
  return payloadString(payload, "query", "");
}

export function wallhavenFilterPayload(
  fields: WallhavenFilterFields,
): SavedFilterPayload {
  return {
    query: fields.query.trim(),
    categories: fields.categories,
    purity: fields.purity,
    sorting: fields.sorting,
    atleast: fields.atleast,
    ratios: fields.ratios,
  };
}

export function wallhavenFilterFieldsFromPayload(
  payload: SavedFilterPayload,
): WallhavenFilterFields {
  return {
    query: payloadString(payload, "query", ""),
    categories: payloadString(payload, "categories", "111"),
    purity: payloadString(payload, "purity", "100"),
    sorting: payloadString(payload, "sorting", "date_added"),
    atleast: payloadString(payload, "atleast", ""),
    ratios: payloadString(payload, "ratios", ""),
  };
}

function payloadString(
  payload: SavedFilterPayload,
  key: string,
  fallback: string,
) {
  const value = payload[key];
  return typeof value === "string" ? value : fallback;
}
