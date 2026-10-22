import { getRuntimeStatus, setWallpaperPaused } from "$lib/api/wallpaperApi";
import type { RuntimeStatus } from "$lib/types/generated";
import type { StoreActionResult } from "$lib/types/wallpaper";
import { latestRequest } from "./latestRequest";

const beginRequest = latestRequest();

export class PlaybackStore {
  status = $state<RuntimeStatus>({
    active: false, paused: false, autoPaused: false, playbackSpeed: null, fitMode: null,
  });
  busy = $state(false);

  async refresh() {
    if (this.busy) return;
    const isCurrent = beginRequest(this);
    try {
      const status = await getRuntimeStatus();
      if (isCurrent()) this.status = status;
    } catch (error) {
      console.warn("Failed to refresh playback status:", error);
    }
  }

  async toggle(): Promise<StoreActionResult<RuntimeStatus>> {
    if (this.busy) return { ok: true, value: this.status };
    const isCurrent = beginRequest(this);
    this.busy = true;
    try {
      const status = await setWallpaperPaused(!this.status.paused);
      if (isCurrent()) this.status = status;
      return { ok: true, value: status };
    } catch (error) {
      return { ok: false, error: `Failed to change playback pause: ${error}` };
    } finally {
      this.busy = false;
    }
  }

  async update(operation: () => Promise<RuntimeStatus>): Promise<RuntimeStatus> {
    if (this.busy) throw new Error("A playback change is already in progress");
    const isCurrent = beginRequest(this);
    this.busy = true;
    try {
      const status = await operation();
      if (isCurrent()) this.status = status;
      return status;
    } catch (error) {
      // Playback may have changed even if saving its default failed.
      try {
        const status = await getRuntimeStatus();
        if (isCurrent()) this.status = status;
      } catch (refreshError) {
        console.warn("Failed to refresh playback status:", refreshError);
      }
      throw error;
    } finally {
      this.busy = false;
    }
  }
}
