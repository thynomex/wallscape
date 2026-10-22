import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "$lib/types/generated";
import type { StoreActionResult } from "$lib/types/wallpaper";

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
  diagnosticWebhookUrl: null,
  diagnosticAutoSend: false,
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

interface PendingUpdate {
  id: number;
  patch: Partial<Settings>;
  persist?: (settings: Settings) => Promise<unknown>;
  resolve: (result: StoreActionResult<Settings>) => void;
}

/** Settings shown in the UI include pending patches over the last saved response. */
export class SettingsStore {
  settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  loaded = $state(false);
  saving = $state(false);
  error = $state<string | null>(null);
  private confirmed: Settings = { ...DEFAULT_SETTINGS };
  private pending: PendingUpdate[] = [];
  private latestUpdate = 0;
  private draining = false;
  private loadPromise: Promise<void> | null = null;

  load(): Promise<void> {
    if (this.loaded) return Promise.resolve();
    this.loadPromise ??= this.loadInitialSettings();
    return this.loadPromise;
  }

  private async loadInitialSettings() {
    try {
      this.confirmed = await invoke<Settings>("get_settings");
    } catch (e) {
      // Backend unavailable (e.g. browser preview) — keep defaults.
      console.warn("Failed to load settings, using defaults:", e);
      this.confirmed = { ...DEFAULT_SETTINGS };
    } finally {
      this.rebuildOptimisticSettings();
      this.loaded = true;
    }
  }

  /**
   * Serialize writes and rebase each patch on the last confirmed response.
   * Older replies cannot roll back newer edits, including when a save fails.
   * A custom persist callback owns both runtime application and persistence;
   * it runs instead of update_settings, with get_settings confirming its result.
   */
  update(
    patch: Partial<Settings>,
    persist?: (settings: Settings) => Promise<unknown>,
  ): Promise<StoreActionResult<Settings>> {
    const id = ++this.latestUpdate;
    const result = new Promise<StoreActionResult<Settings>>((resolve) => {
      this.pending.push({ id, patch: { ...patch }, persist, resolve });
    });
    this.rebuildOptimisticSettings();
    this.saving = true;
    this.error = null;
    void this.drainUpdates();
    return result;
  }

  private rebuildOptimisticSettings() {
    this.settings = this.pending.reduce(
      (settings, update) => ({ ...settings, ...update.patch }),
      { ...this.confirmed },
    );
  }

  private async drainUpdates() {
    if (this.draining) return;
    this.draining = true;
    await this.load();

    while (this.pending.length) {
      const update = this.pending[0];
      let result: StoreActionResult<Settings>;
      let persistedByCallback = false;
      try {
        const next = { ...this.confirmed, ...update.patch };
        if (update.persist) {
          // Playback commands apply to the runtime and persist their own defaults.
          await update.persist({ ...next });
          persistedByCallback = true;
          // A successful command committed this valid patch. Retain it even if
          // the authoritative refresh fails, so the next write cannot undo it.
          this.confirmed = next;
          this.confirmed = await invoke<Settings>("get_settings");
        } else {
          this.confirmed = await invoke<Settings>("update_settings", { settings: next });
        }
        result = { ok: true, value: { ...this.confirmed } };
      } catch (e) {
        let error = persistedByCallback
          ? `Settings saved, but failed to reload settings: ${e}`
          : `Failed to save settings: ${e}`;
        if (update.persist && !persistedByCallback) {
          // A rejected IPC reply can be ambiguous. Prefer authoritative defaults;
          // if they are unavailable, retain the previously confirmed settings.
          try {
            this.confirmed = await invoke<Settings>("get_settings");
          } catch (refreshError) {
            error += `. Failed to reload settings: ${refreshError}`;
          }
        }
        result = { ok: false, error };
        console.error(e);
      }

      this.pending.shift();
      this.rebuildOptimisticSettings();
      if (update.id === this.latestUpdate) {
        this.error = result.ok ? null : result.error;
      }
      update.resolve(result);
    }

    this.saving = false;
    this.draining = false;
  }
}

export const settingsStore = new SettingsStore();
