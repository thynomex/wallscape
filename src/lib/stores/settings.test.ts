import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_SETTINGS, SettingsStore, type Settings } from "./settings.svelte";
import { updateFavoriteRotationInterval } from "$lib/utils/favoriteWorkflow";
import { deferred } from "./testHelpers";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  invokeMock.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => {});
});

async function loadedStore(settings: Settings = { ...DEFAULT_SETTINGS }) {
  invokeMock.mockResolvedValueOnce(settings);
  const store = new SettingsStore();
  await store.load();
  invokeMock.mockClear();
  return store;
}

describe("settings persistence", () => {
  it("preserves a disabled desktop occlusion preference when other settings change", async () => {
    const initial = { ...DEFAULT_SETTINGS, autoPauseOccluded: false };
    const store = await loadedStore(initial);
    invokeMock.mockResolvedValueOnce({ ...initial, closeToTray: false });

    expect((await store.update({ closeToTray: false })).ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledExactlyOnceWith("update_settings", {
      settings: { ...initial, closeToTray: false },
    });
    expect(store.settings.autoPauseOccluded).toBe(false);
  });

  it("serializes writes and preserves a newer optimistic value when an old reply arrives", async () => {
    const store = await loadedStore();
    const first = deferred<Settings>();
    const second = deferred<Settings>();
    invokeMock.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);

    const a = store.update({ defaultPlaybackSpeed: 1.5 });
    const b = store.update({ defaultPlaybackSpeed: 2, closeToTray: false });
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(store.settings.defaultPlaybackSpeed).toBe(2);
    expect(store.saving).toBe(true);

    const savedFirst = { ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 1.5 };
    first.resolve(savedFirst);
    expect(await a).toEqual({ ok: true, value: savedFirst });
    expect(store.settings.defaultPlaybackSpeed).toBe(2);
    expect(store.saving).toBe(true);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "update_settings", {
      settings: { ...savedFirst, defaultPlaybackSpeed: 2, closeToTray: false },
    });

    const savedSecond = { ...savedFirst, defaultPlaybackSpeed: 2, closeToTray: false };
    second.resolve(savedSecond);
    expect(await b).toEqual({ ok: true, value: savedSecond });
    expect(store.settings).toEqual(savedSecond);
    expect(store.saving).toBe(false);
  });

  it("rebases later patches after failure without retrying a rejected setting", async () => {
    const store = await loadedStore();
    const first = deferred<Settings>();
    const second = deferred<Settings>();
    invokeMock.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const a = store.update({ launchAtStartup: true });
    const b = store.update({ startMinimized: true });
    await Promise.resolve();

    first.reject("Access denied");
    expect(await a).toEqual({ ok: false, error: "Failed to save settings: Access denied" });
    expect(store.settings.launchAtStartup).toBe(false);
    expect(store.settings.startMinimized).toBe(true);
    expect(store.error).toBeNull();
    expect(store.saving).toBe(true);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "update_settings", {
      settings: { ...DEFAULT_SETTINGS, startMinimized: true },
    });

    second.resolve({ ...DEFAULT_SETTINGS, startMinimized: true });
    expect((await b).ok).toBe(true);
    expect(store.settings.startMinimized).toBe(true);
    expect(store.settings.launchAtStartup).toBe(false);
  });

  it("rolls back the latest failure to the last confirmed value and returns each operation's result", async () => {
    const store = await loadedStore();
    const saved = { ...DEFAULT_SETTINGS, favoriteRotationIntervalMinutes: 45 };
    invokeMock.mockResolvedValueOnce(saved).mockRejectedValueOnce("Disk full");
    const actions = { updateSettings: (patch: Partial<Settings>) => store.update(patch) };

    const a = updateFavoriteRotationInterval(45, 30, actions);
    const b = updateFavoriteRotationInterval(60, 45, actions);
    expect(await a).toEqual({ ok: true, message: "Rotation interval saved" });
    expect(await b).toEqual({ ok: false, error: "Failed to save settings: Disk full" });
    expect(store.settings.favoriteRotationIntervalMinutes).toBe(45);
    expect(store.error).toBe("Failed to save settings: Disk full");
    expect(store.saving).toBe(false);
  });

  it("retains unrelated loaded values and pending edits when initial loading overlaps an update", async () => {
    const initial = deferred<Settings>();
    const saved = deferred<Settings>();
    invokeMock.mockReturnValueOnce(initial.promise).mockReturnValueOnce(saved.promise);
    const store = new SettingsStore();
    const load = store.load();
    const update = store.update({ closeToTray: false });
    expect(store.settings.closeToTray).toBe(false);
    initial.resolve({ ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 1.5 });
    await load;
    expect(store.settings.closeToTray).toBe(false);
    expect(invokeMock).toHaveBeenNthCalledWith(2, "update_settings", {
      settings: { ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 1.5, closeToTray: false },
    });
    saved.resolve({ ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 1.5, closeToTray: false });
    await update;
    expect(store.settings.defaultPlaybackSpeed).toBe(1.5);
  });
});
