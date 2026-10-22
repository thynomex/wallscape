import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { WallpaperStore } from "./wallpapers.svelte";
import { runFavoriteRotation } from "$lib/utils/favoriteWorkflow";
import { deferred, wallpaper } from "./testHelpers";
import type {
  MotionBgsSearchResponse,
  WallhavenSearchResponse,
  Wallpaper,
} from "$lib/types/wallpaper";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), convertFileSrc: vi.fn() }));

const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  invokeMock.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => {});
});

function library() {
  const store = new WallpaperStore();
  store.wallpapers = [wallpaper(1, true), wallpaper(2)];
  store.featured = store.wallpapers[0];
  return store;
}

describe("local searches", () => {
  it("keeps the full library and featured item available for favorite rotation after an empty search", async () => {
    const store = library();
    invokeMock.mockResolvedValueOnce([]);
    await store.search("no match");
    expect(store.searchResults).toEqual([]);
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2]);
    expect(store.featured?.id).toBe(1);
    const rotate = vi.fn().mockResolvedValue({ ok: true, value: wallpaper(1, true) });
    expect(await runFavoriteRotation("scheduled", store.wallpapers, {
      rotateRandomFavorite: rotate,
    })).toMatchObject({ ok: true, rotated: true });
    expect(rotate).toHaveBeenCalledOnce();
  });

  it("ignores stale successes and failures without ending a newer loading state", async () => {
    const store = library();
    const old = deferred<Wallpaper[]>();
    const current = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(old.promise).mockReturnValueOnce(current.promise);
    const a = store.search("old");
    const b = store.search("current");
    old.resolve([wallpaper(1)]);
    await a;
    expect(store.searchLoading).toBe(true);
    expect(store.searchResults).toBeNull();
    current.resolve([wallpaper(2)]);
    await b;
    expect(store.searchResults?.map((item) => item.id)).toEqual([2]);

    const lateFailure = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(lateFailure.promise).mockResolvedValueOnce([wallpaper(1)]);
    const c = store.search("slow failure");
    await store.search("success");
    lateFailure.reject("Network unavailable");
    await c;
    expect(store.searchResults?.map((item) => item.id)).toEqual([1]);
    expect(store.searchError).toBeNull();
  });

  it("clearing a search invalidates pending results without replacing the library", async () => {
    const store = library();
    const response = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(response.promise);
    const pending = store.search("old");
    await store.search("  ");
    expect(store.searchResults).toBeNull();
    expect(store.searchLoading).toBe(false);
    response.resolve([wallpaper(2)]);
    await pending;
    expect(store.searchResults).toBeNull();
    expect(invokeMock).toHaveBeenCalledOnce();
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2]);
  });

  it("reports a current search failure without clearing favorites or the hero", async () => {
    const store = library();
    invokeMock.mockRejectedValueOnce("Database busy");
    await store.search("forest");
    expect(store.searchResults).toEqual([]);
    expect(store.searchError).toContain("Database busy");
    expect(store.wallpapers[0].is_favorite).toBe(true);
    expect(store.featured?.id).toBe(1);
  });

  it("preserves a successful newer library refresh when an older read finishes", async () => {
    const store = library();
    const old = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(old.promise).mockResolvedValueOnce([wallpaper(3)]);
    const pending = store.loadWallpapers();
    await store.loadWallpapers();
    old.resolve([wallpaper(1)]);
    await pending;
    expect(store.wallpapers.map((item) => item.id)).toEqual([3]);
    invokeMock.mockRejectedValueOnce("Database busy");
    await store.loadWallpapers();
    expect(store.wallpapers.map((item) => item.id)).toEqual([3]);
  });

  it("does not discard or tombstone a row added while a list read was pending", async () => {
    const store = library();
    const read = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(read.promise);
    const pending = store.loadWallpapers();
    store.wallpapers = [...store.wallpapers, wallpaper(3)];
    read.resolve([wallpaper(1), wallpaper(2)]);
    await pending;
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2, 3]);
    invokeMock.mockResolvedValueOnce([wallpaper(1), wallpaper(2), wallpaper(3)]);
    await store.loadWallpapers();
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2, 3]);
  });

  it("does not infer permanent deletion from an omitted row in a list response", async () => {
    const store = library();
    invokeMock.mockResolvedValueOnce([wallpaper(1)]);
    await store.loadWallpapers();
    invokeMock.mockResolvedValueOnce([wallpaper(1), wallpaper(2)]);
    await store.loadWallpapers();
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2]);
  });

  it("keeps a concurrent favorite change and newly imported entries from a pending refresh", async () => {
    const store = library();
    const read = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(read.promise).mockResolvedValueOnce(wallpaper(2, true));
    const pending = store.loadWallpapers();
    await store.setFavorite(store.wallpapers[1], true);
    read.resolve([wallpaper(1, true), wallpaper(2), wallpaper(3)]);
    await pending;
    expect(store.wallpapers[1].is_favorite).toBe(true);
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2, 3]);
  });

  it("merges a pending refresh after rotation and keeps the selected and featured records current", async () => {
    const store = library();
    const read = deferred<Wallpaper[]>();
    const rotated = { ...wallpaper(1, true), title: "Rotated favorite" };
    invokeMock.mockReturnValueOnce(read.promise)
      .mockResolvedValueOnce(rotated)
      .mockResolvedValueOnce({ path: null, can_restore: false })
      .mockResolvedValueOnce({ path: null, can_restore: false })
      .mockResolvedValueOnce([]);
    const pending = store.loadWallpapers();
    await store.rotateRandomFavorite();
    read.resolve([wallpaper(1, true), wallpaper(2), wallpaper(3)]);
    await pending;
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 2, 3]);
    expect(store.selected?.title).toBe("Rotated favorite");
    expect(store.featured?.title).toBe("Rotated favorite");
  });

  it("accepts new entries without resurrecting a wallpaper removed during the read", async () => {
    const store = library();
    const read = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(read.promise)
      .mockResolvedValueOnce({ wallpaper: wallpaper(2) });
    const pending = store.loadWallpapers();
    await store.removeWallpaper(store.wallpapers[1]);
    read.resolve([wallpaper(1, true), wallpaper(2), wallpaper(3)]);
    await pending;
    expect(store.wallpapers.map((item) => item.id)).toEqual([1, 3]);
  });

  it("updates favorites in both canonical and filtered results", async () => {
    const store = library();
    invokeMock.mockResolvedValueOnce([wallpaper(2)]);
    await store.search("second");
    invokeMock.mockResolvedValueOnce(wallpaper(2, true));
    await store.setFavorite(store.searchResults![0], true);
    expect(store.wallpapers.find((item) => item.id === 2)?.is_favorite).toBe(true);
    expect(store.searchResults?.[0].is_favorite).toBe(true);
  });

  it("keeps a favorite change when a search started before it returns older data", async () => {
    const store = library();
    const search = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(search.promise).mockResolvedValueOnce(wallpaper(2, true));
    const pending = store.search("second");
    await store.setFavorite(store.wallpapers[1], true);
    search.resolve([wallpaper(2)]);
    await pending;
    expect(store.searchResults?.[0].is_favorite).toBe(true);
    expect(store.wallpapers[1].is_favorite).toBe(true);
  });
});

function wallhavenResponse(page: number): WallhavenSearchResponse {
  return {
    data: [],
    meta: { current_page: page, last_page: 4, per_page: 24, total: 80, seed: null },
  };
}

function motionBgsResponse(page: number): MotionBgsSearchResponse {
  return { data: [], meta: { currentPage: page, hasNextPage: page < 4 } };
}

describe.each([
  {
    name: "Wallhaven",
    search: (store: WallpaperStore, page: number) => store.searchWallhaven({ page }),
    response: wallhavenResponse,
    loading: (store: WallpaperStore) => store.wallhavenLoading,
    page: (store: WallpaperStore) => store.wallhavenMeta?.current_page,
    error: (store: WallpaperStore) => store.wallhavenError,
  },
  {
    name: "MotionBGS",
    search: (store: WallpaperStore, page: number) => store.searchMotionBgs({ page }),
    response: motionBgsResponse,
    loading: (store: WallpaperStore) => store.motionBgsLoading,
    page: (store: WallpaperStore) => store.motionBgsMeta?.currentPage,
    error: (store: WallpaperStore) => store.motionBgsError,
  },
])("$name search races", ({ search, response, loading, page, error }) => {
  it("keeps the latest page when a previous request resolves last", async () => {
    const store = library();
    const old = deferred<unknown>();
    invokeMock.mockReturnValueOnce(old.promise).mockResolvedValueOnce(response(2));
    const a = search(store, 1);
    await search(store, 2);
    old.resolve(response(1));
    await a;
    expect(page(store)).toBe(2);
    expect(loading(store)).toBe(false);
  });

  it("ignores earlier errors and leaves loading owned by the latest request", async () => {
    const store = library();
    const old = deferred<unknown>();
    const current = deferred<unknown>();
    invokeMock.mockReturnValueOnce(old.promise).mockReturnValueOnce(current.promise);
    const a = search(store, 1);
    const b = search(store, 2);
    old.reject("Request failed");
    await a;
    expect(error(store)).toBeNull();
    expect(loading(store)).toBe(true);
    current.resolve(response(2));
    await b;
    expect(page(store)).toBe(2);
  });
});

it("keeps request ownership independent across sources and store instances", async () => {
  const store = library();
  const other = library();
  const local = deferred<Wallpaper[]>();
  const wallhaven = deferred<WallhavenSearchResponse>();
  const motionBgs = deferred<MotionBgsSearchResponse>();
  invokeMock
    .mockReturnValueOnce(local.promise)
    .mockReturnValueOnce(wallhaven.promise)
    .mockReturnValueOnce(motionBgs.promise)
    .mockResolvedValueOnce([wallpaper(2)]);
  const requests = [store.search("one"), store.searchWallhaven({}), store.searchMotionBgs({})];
  await other.search("two");
  local.resolve([wallpaper(1)]);
  wallhaven.resolve(wallhavenResponse(1));
  motionBgs.resolve(motionBgsResponse(1));
  await Promise.all(requests);
  expect(store.searchResults?.[0].id).toBe(1);
  expect(other.searchResults?.[0].id).toBe(2);
  expect(store.wallhavenMeta?.current_page).toBe(1);
  expect(store.motionBgsMeta?.currentPage).toBe(1);
});

function organizedLibrary() {
  const store = library();
  store.searchResults = [...store.wallpapers];
  store.collections = [{ id: 10, name: "Favorites", created_at: 0, wallpaper_count: 2 }];
  store.collectionMemberships = [
    { collection_id: 10, wallpaper_id: 1 },
    { collection_id: 10, wallpaper_id: 2 },
  ];
  store.selected = store.wallpapers[0];
  return store;
}

function expectRemoved(store: WallpaperStore) {
  expect(store.wallpapers.map((item) => item.id)).toEqual([2]);
  expect(store.searchResults?.map((item) => item.id)).toEqual([2]);
  expect(store.collectionMemberships).toEqual([{ collection_id: 10, wallpaper_id: 2 }]);
  expect(store.collections[0].wallpaper_count).toBe(1);
  expect(store.featured?.id).toBe(2);
}

describe("library deletion", () => {
  it("removes the wallpaper from searches, selection, and collection counts", async () => {
    const store = organizedLibrary();
    invokeMock.mockResolvedValueOnce({ wallpaper: wallpaper(1) });
    expect((await store.removeWallpaper(store.wallpapers[0])).ok).toBe(true);
    expectRemoved(store);
    expect(store.selected).toBeNull();
  });

  it("uses the same cleanup when canceling an applied wallpaper", async () => {
    const store = organizedLibrary();
    store.previous = { path: store.wallpapers[1].file_path, can_restore: true };
    invokeMock.mockImplementation(async (command) => {
      if (command === "restore_previous_wallpaper") return undefined;
      if (command === "remove_wallpaper") return { wallpaper: wallpaper(1) };
      if (command === "get_previous_wallpaper") return { path: null, can_restore: false };
      if (command === "list_wallpaper_history") return [];
      throw new Error(`Unexpected command: ${command}`);
    });
    expect((await store.cancelAppliedWallpaper()).ok).toBe(true);
    expectRemoved(store);
    expect(store.selected?.id).toBe(2);
  });

  it("does not reintroduce a removed entry from in-flight search or list responses", async () => {
    const store = organizedLibrary();
    const search = deferred<Wallpaper[]>();
    const list = deferred<Wallpaper[]>();
    invokeMock
      .mockReturnValueOnce(search.promise)
      .mockReturnValueOnce(list.promise)
      .mockResolvedValueOnce({ wallpaper: wallpaper(1) });
    const requests = [store.search("all"), store.loadWallpapers()];
    await store.removeWallpaper(store.wallpapers[0]);
    search.resolve([wallpaper(1), wallpaper(2)]);
    list.resolve([wallpaper(1), wallpaper(2)]);
    await Promise.all(requests);
    expectRemoved(store);
  });

  it("filters stale search responses after maintenance removes missing entries", async () => {
    const store = organizedLibrary();
    const search = deferred<Wallpaper[]>();
    invokeMock.mockReturnValueOnce(search.promise).mockResolvedValueOnce([wallpaper(2)]);
    const pending = store.search("all");
    await store.loadWallpapers();
    search.resolve([wallpaper(1), wallpaper(2)]);
    await pending;
    expect(store.searchResults?.map((item) => item.id)).toEqual([2]);
    expect(store.selected).toBeNull();
  });

  it("keeps restored playback selection if canceling succeeds but removal fails", async () => {
    const store = organizedLibrary();
    store.previous = { path: store.wallpapers[1].file_path, can_restore: true };
    invokeMock.mockImplementation(async (command) => {
      if (command === "restore_previous_wallpaper") return undefined;
      if (command === "remove_wallpaper") throw new Error("Library is locked");
      if (command === "get_previous_wallpaper") return { path: null, can_restore: false };
      if (command === "list_wallpaper_history") return [];
      throw new Error(`Unexpected command: ${command}`);
    });
    expect((await store.cancelAppliedWallpaper()).ok).toBe(false);
    expect(store.selected?.id).toBe(2);
    expect(store.wallpapers).toHaveLength(2);
    expect(store.collections[0].wallpaper_count).toBe(2);
    expect(store.previous.can_restore).toBe(false);
  });

  it("does not report removal when restoring the same wallpaper", async () => {
    const store = organizedLibrary();
    store.previous = { path: store.wallpapers[0].file_path, can_restore: true };
    invokeMock.mockImplementation(async (command) => {
      if (command === "restore_previous_wallpaper") return undefined;
      if (command === "get_previous_wallpaper") return { path: null, can_restore: false };
      if (command === "list_wallpaper_history") return [];
      throw new Error(`Unexpected command: ${command}`);
    });
    expect((await store.cancelAppliedWallpaper()).ok).toBe(true);
    expect(store.wallpapers).toHaveLength(2);
    expect(store.importStatus).toBe("Previous wallpaper restored.");
  });

  it("preserves all state when backend removal fails", async () => {
    const store = organizedLibrary();
    invokeMock.mockRejectedValueOnce("File is in use");
    expect((await store.removeWallpaper(store.wallpapers[0])).ok).toBe(false);
    expect(store.wallpapers).toHaveLength(2);
    expect(store.searchResults).toHaveLength(2);
    expect(store.collections[0].wallpaper_count).toBe(2);
    expect(store.collectionMemberships).toHaveLength(2);
    expect(store.selected?.id).toBe(1);
  });
});
