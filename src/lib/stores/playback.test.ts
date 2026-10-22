import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { PlaybackStore } from "./playback.svelte";
import { DEFAULT_SETTINGS, SettingsStore } from "./settings.svelte";
import { setWallpaperSpeed } from "$lib/api/wallpaperApi";
import { deferred } from "./testHelpers";
import type { RuntimeStatus } from "$lib/types/generated";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);
const playbackValues = { playbackSpeed: 1, fitMode: "fit" };
beforeEach(() => invokeMock.mockReset());

it("keeps the control paused when automatic rules reject a manual resume", async () => {
  const store = new PlaybackStore();
  const automatic = { active: true, paused: true, autoPaused: true, ...playbackValues };
  invokeMock.mockResolvedValueOnce(automatic).mockResolvedValueOnce(automatic);
  await store.refresh();
  expect((await store.toggle()).ok).toBe(true);
  expect(invokeMock).toHaveBeenLastCalledWith("set_wallpaper_paused", { paused: false });
  expect(store.status).toEqual(automatic);
});

it("ignores a stale status poll after a successful pause and prevents overlapping toggles", async () => {
  const store = new PlaybackStore();
  const poll = deferred<RuntimeStatus>();
  const pause = deferred<RuntimeStatus>();
  invokeMock.mockReturnValueOnce(poll.promise).mockReturnValueOnce(pause.promise);
  const refresh = store.refresh();
  const toggle = store.toggle();
  await store.toggle();
  expect(invokeMock).toHaveBeenCalledTimes(2);
  expect(store.busy).toBe(true);
  pause.resolve({ active: true, paused: true, autoPaused: false, ...playbackValues });
  await toggle;
  poll.resolve({ active: true, paused: false, autoPaused: false, ...playbackValues });
  await refresh;
  expect(store.status.paused).toBe(true);
  expect(store.busy).toBe(false);
});

it("preserves the confirmed pause state and reports errors when a change fails", async () => {
  const store = new PlaybackStore();
  store.status = { active: true, paused: true, autoPaused: false, ...playbackValues };
  invokeMock.mockRejectedValueOnce("Player unavailable");
  expect(await store.toggle()).toEqual({
    ok: false, error: "Failed to change playback pause: Player unavailable",
  });
  expect(store.status.paused).toBe(true);
  expect(store.busy).toBe(false);
});

it("keeps current playback values when Settings saves different defaults", async () => {
  const playback = new PlaybackStore();
  const settings = new SettingsStore();
  const active = { active: true, paused: false, autoPaused: false, ...playbackValues };
  invokeMock.mockResolvedValueOnce(active).mockResolvedValueOnce({ ...DEFAULT_SETTINGS });
  await playback.refresh();
  await settings.load();
  invokeMock.mockResolvedValueOnce({
    ...DEFAULT_SETTINGS, defaultPlaybackSpeed: 2, defaultFitMode: "fill",
  }).mockResolvedValueOnce(active);

  await settings.update({ defaultPlaybackSpeed: 2, defaultFitMode: "fill" });
  await playback.refresh();

  expect(settings.settings.defaultPlaybackSpeed).toBe(2);
  expect(settings.settings.defaultFitMode).toBe("fill");
  expect(playback.status.playbackSpeed).toBe(1);
  expect(playback.status.fitMode).toBe("fit");
});

it("uses confirmed speed changes immediately and ignores an older status poll", async () => {
  const store = new PlaybackStore();
  const poll = deferred<RuntimeStatus>();
  const active = { active: true, paused: false, autoPaused: false, ...playbackValues };
  invokeMock.mockReturnValueOnce(poll.promise).mockResolvedValueOnce({ ...active, playbackSpeed: 2 });
  const refresh = store.refresh();
  await store.update(() => setWallpaperSpeed(2));
  poll.resolve(active);
  await refresh;
  expect(store.status.playbackSpeed).toBe(2);
  expect(store.busy).toBe(false);
});

it("reloads actual playback after a command applies but cannot save its default", async () => {
  const store = new PlaybackStore();
  const active = { active: true, paused: false, autoPaused: false, ...playbackValues, playbackSpeed: 2 };
  invokeMock.mockRejectedValueOnce("Default could not be saved").mockResolvedValueOnce(active);
  await expect(store.update(() => setWallpaperSpeed(2))).rejects.toBe("Default could not be saved");
  expect(store.status.playbackSpeed).toBe(2);
  expect(store.busy).toBe(false);
});
