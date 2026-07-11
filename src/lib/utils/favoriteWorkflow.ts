import {
  normalizeFavoriteRotationInterval,
  type Settings,
} from "$lib/stores/settings.svelte";
import type { StoreActionResult, Wallpaper } from "$lib/types/wallpaper";
import {
  favoriteChangeMessage,
  favoriteRotationResultMessage,
  favoriteRotationToggleMessage,
  type FavoriteRotationReason,
} from "$lib/utils/toastMessages";

export type { FavoriteRotationReason };

interface FavoriteActions {
  rotateRandomFavorite(): Promise<StoreActionResult<Wallpaper>>;
  setFavorite(
    wallpaper: Wallpaper,
    isFavorite: boolean,
  ): Promise<StoreActionResult<Wallpaper>>;
}

interface SettingsActions {
  updateSettings(patch: Partial<Settings>): Promise<void>;
  settingsError(): string | null;
}

export type FavoriteRotationOutcome =
  | {
      ok: true;
      rotated: false;
    }
  | {
      ok: true;
      rotated: true;
      wallpaper: Wallpaper;
      message: string;
      shouldToast: boolean;
    }
  | {
      ok: false;
      error: string;
      shouldToast: boolean;
    };

export interface FavoriteUpdateOutcome {
  wallpaper: Wallpaper;
  message: string;
}

export type SettingsUpdateOutcome =
  | {
      ok: true;
      message: string | null;
    }
  | {
      ok: false;
      error: string;
    };

export async function runFavoriteRotation(
  reason: FavoriteRotationReason,
  wallpapers: Wallpaper[],
  actions: Pick<FavoriteActions, "rotateRandomFavorite">,
): Promise<FavoriteRotationOutcome> {
  if (!wallpapers.some((wallpaper) => wallpaper.is_favorite)) {
    return reason === "manual"
      ? { ok: false, error: "No favorites to shuffle yet", shouldToast: true }
      : { ok: true, rotated: false };
  }

  const result = await actions.rotateRandomFavorite();
  if (!result.ok) {
    return {
      ok: false,
      error: result.error,
      shouldToast: reason !== "scheduled",
    };
  }

  return {
    ok: true,
    rotated: true,
    wallpaper: result.value,
    message: favoriteRotationResultMessage(reason),
    shouldToast: reason !== "scheduled",
  };
}

export async function updateWallpaperFavorite(
  wallpaper: Wallpaper,
  isFavorite: boolean,
  actions: Pick<FavoriteActions, "setFavorite">,
): Promise<StoreActionResult<FavoriteUpdateOutcome>> {
  const result = await actions.setFavorite(wallpaper, isFavorite);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      wallpaper: result.value,
      message: favoriteChangeMessage(isFavorite),
    },
  };
}

export async function updateFavoriteRotationEnabled(
  enabled: boolean,
  hasFavorites: boolean,
  previous: boolean,
  actions: SettingsActions,
): Promise<SettingsUpdateOutcome> {
  if (enabled && !hasFavorites) {
    return { ok: false, error: "Add a favorite before starting rotation" };
  }

  await actions.updateSettings({ favoriteRotationEnabled: enabled });

  const error = actions.settingsError();
  if (error) {
    return { ok: false, error };
  }

  return {
    ok: true,
    message: previous !== enabled ? favoriteRotationToggleMessage(enabled) : null,
  };
}

export async function updateFavoriteRotationInterval(
  minutes: number,
  previous: number,
  actions: SettingsActions,
): Promise<SettingsUpdateOutcome> {
  const next = normalizeFavoriteRotationInterval(minutes);
  await actions.updateSettings({ favoriteRotationIntervalMinutes: next });

  const error = actions.settingsError();
  if (error) {
    return { ok: false, error };
  }

  return {
    ok: true,
    message: previous !== next ? "Rotation interval saved" : null,
  };
}
