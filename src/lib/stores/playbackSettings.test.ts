import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { setWallpaperFitMode, setWallpaperSpeed } from "$lib/api/wallpaperApi";
import { DEFAULT_SETTINGS, SettingsStore, type Settings } from "./settings.svelte";
import { deferred } from "./testHelpers";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  invokeMock.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => {});
});

async function loadedStore() {
  invokeMock.mockResolvedValueOnce({ ...DEFAULT_SETTINGS });
  const store = new SettingsStore();
  await store.load();
  invokeMock.mockClear();
  return store;
}

describe("detail playback settings", () => {
  it("preserves speed and fit defaults when an unrelated setting is saved later", async () => {
    const store = await loadedStore();
    let persisted = { ...DEFAULT_SETTINGS };
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "get_settings") return { ...persisted };
      if (command === "update_settings") {
        persisted = { ...(args as { settings: Settings }).settings };
        return { ...persisted };
      }
      // The existing runtime commands persist defaults as well as applying them.
      if (command === "set_wallpaper_speed") {
        persisted.defaultPlaybackSpeed = (args as { speed: number }).speed;
        return;
      }
      if (command === "set_wallpaper_fit_mode") {
        persisted.defaultFitMode = (args as { mode: string }).mode;
        return;
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    await store.update({ defaultPlaybackSpeed: 2 }, (saved) =>
      setWallpaperSpeed(saved.defaultPlaybackSpeed));
    await store.update({ defaultFitMode: "fill" }, (saved) =>
      setWallpaperFitMode(saved.defaultFitMode));
    await store.update({ closeToTray: false });

    expect(persisted).toMatchObject({
      defaultPlaybackSpeed: 2,
      defaultFitMode: "fill",
      closeToTray: false,
    });
    expect(store.settings).toEqual(persisted);
    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual([
      "set_wallpaper_speed", "get_settings",
      "set_wallpaper_fit_mode", "get_settings", "update_settings",
    ]);
  });

  it("holds newer writes until the previous runtime command finishes", async () => {
    const store = await loadedStore();
    const applying = deferred<void>();
    const started = deferred<void>();
    const savedFirst = { ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 1.5 };
    const savedSecond = { ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 2 };
    invokeMock.mockResolvedValueOnce(savedFirst).mockResolvedValueOnce(savedSecond);
    const applyFirst = vi.fn(async () => {
      started.resolve();
      await applying.promise;
    });
    const applySecond = vi.fn(async () => {});

    const first = store.update({ defaultPlaybackSpeed: 1.5 }, applyFirst);
    const second = store.update({ defaultPlaybackSpeed: 2 }, applySecond);
    await started.promise;

    expect(invokeMock).not.toHaveBeenCalled();
    expect(applySecond).not.toHaveBeenCalled();
    expect(store.settings.defaultPlaybackSpeed).toBe(2);
    expect(store.saving).toBe(true);

    applying.resolve();
    expect((await first).ok).toBe(true);
    expect(store.settings.defaultPlaybackSpeed).toBe(2);
    expect((await second).ok).toBe(true);
    expect(applyFirst).toHaveBeenCalledExactlyOnceWith(savedFirst);
    expect(applySecond).toHaveBeenCalledExactlyOnceWith(savedSecond);
    expect(store.saving).toBe(false);
  });

  it("retains previous defaults when the playback command cannot persist them", async () => {
    const store = await loadedStore();
    invokeMock.mockResolvedValueOnce({ ...DEFAULT_SETTINGS });
    const apply = vi.fn(async () => { throw "Disk full"; });

    expect(await store.update({ defaultPlaybackSpeed: 2 }, apply)).toEqual({
      ok: false,
      error: "Failed to save settings: Disk full",
    });
    expect(apply).toHaveBeenCalledOnce();
    expect(invokeMock).toHaveBeenCalledExactlyOnceWith("get_settings");
    expect(store.settings.defaultPlaybackSpeed).toBe(1);
    expect(store.saving).toBe(false);
  });

  it("keeps previous defaults after runtime failure and continues the queue", async () => {
    const store = await loadedStore();
    invokeMock
      .mockRejectedValueOnce(new Error("Player unavailable"))
      .mockResolvedValueOnce({ ...DEFAULT_SETTINGS })
      .mockResolvedValueOnce({ ...DEFAULT_SETTINGS, closeToTray: false });

    const first = store.update({ defaultPlaybackSpeed: 2 }, (settings) =>
      setWallpaperSpeed(settings.defaultPlaybackSpeed));
    const second = store.update({ closeToTray: false });

    expect(await first).toEqual({
      ok: false,
      error: "Failed to save settings: Error: Player unavailable",
    });
    expect((await second).ok).toBe(true);
    expect(invokeMock).toHaveBeenNthCalledWith(3, "update_settings", {
      settings: { ...DEFAULT_SETTINGS, closeToTray: false },
    });
    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual([
      "set_wallpaper_speed", "get_settings", "update_settings",
    ]);
    expect(store.settings.defaultPlaybackSpeed).toBe(1);
    expect(store.settings.closeToTray).toBe(false);
    expect(store.error).toBeNull();
    expect(store.saving).toBe(false);
  });

  it("retains previous defaults if a failed command cannot be reconciled", async () => {
    const store = await loadedStore();
    invokeMock
      .mockRejectedValueOnce("Player unavailable")
      .mockRejectedValueOnce("IPC unavailable");

    expect(await store.update({ defaultPlaybackSpeed: 2 }, (settings) =>
      setWallpaperSpeed(settings.defaultPlaybackSpeed))).toEqual({
      ok: false,
      error: "Failed to save settings: Player unavailable. Failed to reload settings: IPC unavailable",
    });
    expect(store.settings.defaultPlaybackSpeed).toBe(1);
    expect(store.saving).toBe(false);
  });

  it("retains a successfully persisted patch if refreshing settings fails", async () => {
    const store = await loadedStore();
    const saved = { ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 2 };
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce("IPC unavailable")
      .mockResolvedValueOnce({ ...saved, closeToTray: false });

    const first = store.update({ defaultPlaybackSpeed: 2 }, (settings) =>
      setWallpaperSpeed(settings.defaultPlaybackSpeed));
    expect(await first).toEqual({
      ok: false,
      error: "Settings saved, but failed to reload settings: IPC unavailable",
    });
    expect(store.settings.defaultPlaybackSpeed).toBe(2);
    expect(store.error).toBe("Settings saved, but failed to reload settings: IPC unavailable");

    expect((await store.update({ closeToTray: false })).ok).toBe(true);
    expect(invokeMock).toHaveBeenNthCalledWith(3, "update_settings", {
      settings: { ...saved, closeToTray: false },
    });
    expect(store.settings.defaultPlaybackSpeed).toBe(2);
    expect(store.error).toBeNull();
    expect(store.saving).toBe(false);
  });
});
