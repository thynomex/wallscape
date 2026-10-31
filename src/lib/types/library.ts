/** Library grid filter: everything, favorites, or one collection. */
export type LibraryFilter = "all" | "favorites" | `collection:${number}`;

/**
 * How many cards a long grid has revealed through "Show more", tied to the
 * list it was revealed for. Views are remounted on every switch, so the
 * owner keeps this above them to preserve it.
 */
export interface RenderWindow {
  key: string;
  count: number;
}
