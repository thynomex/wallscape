import type {
  Monitor,
  StoreActionResult,
  Wallpaper,
  WallpaperRestoreStatus,
} from "$lib/types/wallpaper";
import { monitorLabel, monitorTargetForWallpaper } from "$lib/utils/monitorDisplay";
import { canCancelAppliedWallpaper } from "$lib/utils/wallpaperCancellation";

interface WallpaperLifecycleActions {
  apply(wallpaper: Wallpaper, monitorId: string | null): Promise<StoreActionResult>;
  restoreOriginalWallpaper(): Promise<StoreActionResult>;
  cancelAppliedWallpaper(wallpaper: Wallpaper): Promise<StoreActionResult>;
}

export interface WallpaperApplyOutcome {
  wallpaper: Wallpaper;
  message: string;
}

export interface WallpaperLifecycleMessageOutcome {
  message: string;
}

export function canCancelWallpaper(
  selected: Wallpaper | null,
  previous: WallpaperRestoreStatus,
  wallpaper: Wallpaper | null,
) {
  return canCancelAppliedWallpaper(selected, previous, wallpaper);
}

export async function applyWallpaper(
  wallpaper: Wallpaper,
  monitorId: string | null,
  monitors: Monitor[],
  actions: Pick<WallpaperLifecycleActions, "apply">,
): Promise<StoreActionResult<WallpaperApplyOutcome>> {
  const targetMonitorId = monitorTargetForWallpaper(wallpaper, monitorId);
  const result = await actions.apply(wallpaper, targetMonitorId);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      wallpaper,
      message: targetMonitorId
        ? `Wallpaper applied to ${monitorLabel(monitors, targetMonitorId)}`
        : "Wallpaper applied",
    },
  };
}

export async function restoreOriginalWallpaper(
  actions: Pick<WallpaperLifecycleActions, "restoreOriginalWallpaper">,
): Promise<StoreActionResult<WallpaperLifecycleMessageOutcome>> {
  const result = await actions.restoreOriginalWallpaper();
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      message: "Original wallpaper restored",
    },
  };
}

export async function cancelAppliedWallpaper(
  wallpaper: Wallpaper,
  actions: Pick<WallpaperLifecycleActions, "cancelAppliedWallpaper">,
): Promise<StoreActionResult<WallpaperLifecycleMessageOutcome>> {
  const result = await actions.cancelAppliedWallpaper(wallpaper);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      message: "Wallpaper canceled",
    },
  };
}
