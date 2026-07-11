import type { WallhavenSearchArgs } from "$lib/types/wallpaper";

export interface WallhavenSearchFields {
  query: string;
  categories: string;
  purity: string;
  sorting: string;
  atleast: string;
  ratios: string;
}

export function wallhavenSearchArgs(
  fields: WallhavenSearchFields,
  page: number,
): WallhavenSearchArgs {
  return {
    query: fields.query,
    categories: fields.categories,
    purity: fields.purity,
    sorting: fields.sorting,
    order: "desc",
    atleast: fields.atleast || undefined,
    ratios: fields.ratios || undefined,
    page,
  };
}

export function effectiveWallhavenSorting(query: string, sorting: string) {
  return sorting === "relevance" && !query.trim() ? "date_added" : sorting;
}
