import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "$lib/types/generated";

export type { Settings } from "$lib/types/generated";

export const DEFAULT_FAVORITE_ROTATION_INTERVAL_MINUTES = 30;
export const MIN_FAVORITE_ROTATION_INTERVAL_MINUTES = 1;
export const MAX_FAVORITE_ROTATION_INTERVAL_MINUTES = 1440;

export const DEFAULT_SETTINGS: Settings = {
  launchAtStartup: false,
  startMinimized: false,
  closeToTray: true,
  minimizeToTray: false,
  restoreLastWallpaper: true,
  favoriteRotationEnabled: false,
  favoriteRotationIntervalMinutes: DEFAULT_FAVORITE_ROTATION_INTERVAL_MINUTES,
  favoriteRotationOnStartup: false,
  autoPauseEnabled: true,
  autoPauseOnBattery: true,
  autoPauseFullscreenApps: true,
  autoPauseOccluded: true,
  autoPauseRemoteSession: true,
  autoPauseDisplaySleep: true,
  defaultPlaybackSpeed: 1.0,
  defaultFitMode: "fit",
  defaultFpsCap: null,
};

export function normalizeFavoriteRotationInterval(value: number) {
  if (!Number.isFinite(value)) return DEFAULT_FAVORITE_ROTATION_INTERVAL_MINUTES;

  return Math.min(
    MAX_FAVORITE_ROTATION_INTERVAL_MINUTES,
    Math.max(MIN_FAVORITE_ROTATION_INTERVAL_MINUTES, Math.round(value)),
  );
}

/** Reactive settings state — Svelte 5 runes in a module (`.svelte.ts`). */
class SettingsStore {
  settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  loaded = $state(false);
  saving = $state(false);
  error = $state<string | null>(null);

  async load() {
    try {
      this.settings = await invoke<Settings>("get_settings");
    } catch (e) {
      // Backend unavailable (e.g. browser preview) — keep defaults.
      console.warn("Failed to load settings, using defaults:", e);
      this.settings = { ...DEFAULT_SETTINGS };
    } finally {
      this.loaded = true;
    }
  }

  /**
   * Optimistically apply a partial change and persist it. On failure the
   * previous values are restored so the UI mirrors the real saved state
   * (important for the launch-at-startup toggle, which can fail at the OS level).
   */
  async update(patch: Partial<Settings>) {
    const previous = { ...this.settings };
    const next = { ...this.settings, ...patch };
    this.settings = next;
    this.saving = true;
    this.error = null;

    try {
      this.settings = await invoke<Settings>("update_settings", { settings: next });
    } catch (e) {
      this.settings = previous;
      this.error = `Failed to save settings: ${e}`;
      console.error(e);
    } finally {
      this.saving = false;
    }
  }
}

export const settingsStore = new SettingsStore();
