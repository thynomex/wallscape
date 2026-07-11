import type {
  BatchImportResult as GeneratedBatchImportResult,
  BrokenWallpaper as GeneratedBrokenWallpaper,
  DownloadMotionBgsResult as GeneratedDownloadMotionBgsResult,
  Collection,
  CollectionMembership,
  DownloadWallhavenResult as GeneratedDownloadWallhavenResult,
  ImportProbe as GeneratedImportProbe,
  ImportScanResult,
  ImportWallpaperResult as GeneratedImportWallpaperResult,
  Monitor,
  MotionBgsMeta,
  MotionBgsSearchArgs as GeneratedMotionBgsSearchArgs,
  MotionBgsSearchResponse,
  MotionBgsWallpaper,
  RemoveWallpaperResult as GeneratedRemoveWallpaperResult,
  SavedFilter as GeneratedSavedFilter,
  StorageCleanupResult,
  StorageStats,
  ThumbnailRegenerationResult as GeneratedThumbnailRegenerationResult,
  WallhavenMeta,
  WallhavenSearchArgs as GeneratedWallhavenSearchArgs,
  WallhavenSearchResponse,
  WallhavenWallpaper,
  Wallpaper as GeneratedWallpaper,
  WallpaperBackupStatus,
  WallpaperHistoryEntry,
  WallpaperRestoreStatus,
} from "./generated";

export type MediaType = "video" | "image";
type OptionalCommandInput<T> = {
  [K in keyof T]?: T[K] | undefined;
};

export type {
  Monitor,
  MotionBgsMeta,
  MotionBgsSearchResponse,
  MotionBgsWallpaper,
  WallhavenMeta,
  WallhavenSearchResponse,
  WallhavenWallpaper,
  WallpaperBackupStatus,
  WallpaperHistoryEntry,
  WallpaperRestoreStatus,
  StorageCleanupResult,
  StorageStats,
  Collection,
  CollectionMembership,
};

/** Normalized wallpaper shape used by the UI after API-boundary cleanup. */
export interface Wallpaper
  extends Omit<
    GeneratedWallpaper,
    "media_type" | "source" | "source_id" | "file_size_bytes" | "created_at"
  > {
  media_type: MediaType;
  source: string | null;
  source_id: string | null;
  file_size_bytes?: number;
  created_at?: number;
  file_size_mb?: number;
  /** UI-only: optional badge shown on the card. */
  badge?: "new" | "featured" | "exclusive";
}

export interface WallpaperDto
  extends Omit<GeneratedWallpaper, "media_type" | "source" | "source_id" | "is_favorite"> {
  media_type?: string | null;
  source?: string | null;
  source_id?: string | null;
  is_favorite?: boolean | null;
}

export type ImportProbe = OptionalCommandInput<GeneratedImportProbe>;
export interface BatchImportItem {
  videoPath: string;
  probe?: ImportProbe | null;
}
export type { ImportScanResult };
export type MotionBgsSearchArgs = OptionalCommandInput<GeneratedMotionBgsSearchArgs>;
export type WallhavenSearchArgs = OptionalCommandInput<GeneratedWallhavenSearchArgs>;
export type SavedFilterType = "local" | "wallhaven";
export type SavedFilterPayload = Record<string, unknown>;

export interface SavedFilter
  extends Omit<GeneratedSavedFilter, "filter_type" | "payload"> {
  filter_type: SavedFilterType;
  payload: SavedFilterPayload;
}

export interface SavedFilterDto extends Omit<GeneratedSavedFilter, "filter_type"> {
  filter_type?: string | null;
}

export interface ImportWallpaperResult
  extends Omit<GeneratedImportWallpaperResult, "wallpaper"> {
  wallpaper: Wallpaper;
}

export interface RemoveWallpaperResult
  extends Omit<GeneratedRemoveWallpaperResult, "wallpaper"> {
  wallpaper: Wallpaper;
}

export interface DownloadWallhavenResult
  extends Omit<GeneratedDownloadWallhavenResult, "wallpaper"> {
  wallpaper: Wallpaper;
}

export interface DownloadMotionBgsResult
  extends Omit<GeneratedDownloadMotionBgsResult, "wallpaper"> {
  wallpaper: Wallpaper;
}

export interface BatchImportResult
  extends Omit<GeneratedBatchImportResult, "imported" | "duplicates"> {
  imported: Wallpaper[];
  duplicates: Wallpaper[];
}

export interface BrokenWallpaper extends Omit<GeneratedBrokenWallpaper, "wallpaper"> {
  wallpaper: Wallpaper;
}

export interface ThumbnailRegenerationResult
  extends Omit<GeneratedThumbnailRegenerationResult, "wallpaper"> {
  wallpaper: Wallpaper;
}

export type StoreActionResult<T = void> =
  | { ok: true; value: T }
  | { ok: false; error: string };

export function normalizeWallpaper(dto: WallpaperDto): Wallpaper {
  return {
    ...dto,
    media_type: dto.media_type === "image" ? "image" : "video",
    source: dto.source ?? null,
    source_id: dto.source_id ?? null,
    is_favorite: Boolean(dto.is_favorite),
  };
}

export function normalizeSavedFilter(dto: SavedFilterDto): SavedFilter {
  return {
    ...dto,
    filter_type: dto.filter_type === "wallhaven" ? "wallhaven" : "local",
    payload: parseFilterPayload(dto.payload),
  };
}

function parseFilterPayload(payload: string): SavedFilterPayload {
  try {
    const parsed: unknown = JSON.parse(payload);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as SavedFilterPayload;
    }
  } catch {
    // Invalid saved payloads are ignored at the UI boundary.
  }

  return {};
}
