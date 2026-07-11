import { invoke } from "@tauri-apps/api/core";
import type {
  BatchImportItem,
  BatchImportResult,
  BrokenWallpaper,
  Collection,
  CollectionMembership,
  DownloadMotionBgsResult,
  DownloadWallhavenResult,
  ImportProbe,
  ImportScanResult,
  ImportWallpaperResult,
  Monitor,
  MotionBgsSearchArgs,
  MotionBgsSearchResponse,
  MotionBgsWallpaper,
  RemoveWallpaperResult,
  SavedFilter,
  SavedFilterDto,
  SavedFilterPayload,
  SavedFilterType,
  StorageCleanupResult,
  StorageStats,
  ThumbnailRegenerationResult,
  WallhavenSearchArgs,
  WallhavenSearchResponse,
  WallhavenWallpaper,
  Wallpaper,
  WallpaperBackupStatus,
  WallpaperDto,
  WallpaperHistoryEntry,
  WallpaperRestoreStatus,
} from "$lib/types/wallpaper";
import { normalizeSavedFilter, normalizeWallpaper } from "$lib/types/wallpaper";

interface ImportWallpaperResultDto {
  wallpaper: WallpaperDto;
  warnings: string[];
  duplicate: boolean;
}

interface RemoveWallpaperResultDto {
  wallpaper: WallpaperDto;
}

interface DownloadWallhavenResultDto {
  wallpaper: WallpaperDto;
  sourceId: string;
}

interface DownloadMotionBgsResultDto {
  wallpaper: WallpaperDto;
  sourceId: string;
}

interface BatchImportResultDto {
  imported: WallpaperDto[];
  duplicates: WallpaperDto[];
  failed: BatchImportResult["failed"];
  warnings: string[];
}

interface BrokenWallpaperDto {
  wallpaper: WallpaperDto;
  reason: string;
}

interface ThumbnailRegenerationResultDto {
  wallpaper: WallpaperDto;
  warnings: string[];
}

function normalizeWallpapers(wallpapers: WallpaperDto[]): Wallpaper[] {
  return wallpapers.map(normalizeWallpaper);
}

export function listMonitors(): Promise<Monitor[]> {
  return invoke<Monitor[]>("get_monitors");
}

export async function listWallpapers(): Promise<Wallpaper[]> {
  return normalizeWallpapers(await invoke<WallpaperDto[]>("list_wallpapers"));
}

export function listCollections(): Promise<Collection[]> {
  return invoke<Collection[]>("list_collections");
}

export function createCollection(name: string): Promise<Collection> {
  return invoke<Collection>("create_collection", { name });
}

export function deleteCollection(id: number): Promise<void> {
  return invoke("delete_collection", { id });
}

export function listCollectionMemberships(): Promise<CollectionMembership[]> {
  return invoke<CollectionMembership[]>("list_collection_memberships");
}

export function setCollectionMembership(
  collectionId: number,
  wallpaperId: number,
  inCollection: boolean,
): Promise<Collection> {
  return invoke<Collection>("set_collection_membership", {
    collectionId,
    wallpaperId,
    inCollection,
  });
}

export async function listSavedFilters(
  filterType?: SavedFilterType,
): Promise<SavedFilter[]> {
  const filters = await invoke<SavedFilterDto[]>("list_saved_filters", {
    filterType: filterType ?? null,
  });
  return filters.map(normalizeSavedFilter);
}

export async function saveFilter(
  name: string,
  filterType: SavedFilterType,
  payload: SavedFilterPayload,
): Promise<SavedFilter> {
  const filter = await invoke<SavedFilterDto>("save_filter", {
    name,
    filterType,
    payload: JSON.stringify(payload),
  });
  return normalizeSavedFilter(filter);
}

export function deleteSavedFilter(id: number): Promise<void> {
  return invoke("delete_saved_filter", { id });
}

export async function searchWallpapers(query: string): Promise<Wallpaper[]> {
  return normalizeWallpapers(await invoke<WallpaperDto[]>("search_wallpapers", { query }));
}

export function getOriginalWallpaperBackup(): Promise<WallpaperBackupStatus> {
  return invoke<WallpaperBackupStatus>("get_original_wallpaper_backup");
}

export function getPreviousWallpaper(): Promise<WallpaperRestoreStatus> {
  return invoke<WallpaperRestoreStatus>("get_previous_wallpaper");
}

export async function importWallpaper(
  videoPath: string,
  probe?: ImportProbe,
): Promise<ImportWallpaperResult> {
  const result = await invoke<ImportWallpaperResultDto>("import_wallpaper", {
    videoPath,
    probe,
  });

  return {
    ...result,
    wallpaper: normalizeWallpaper(result.wallpaper),
  };
}

export function scanImportPaths(paths: string[]): Promise<ImportScanResult> {
  return invoke<ImportScanResult>("scan_import_paths", { paths });
}

export async function importWallpapers(
  items: BatchImportItem[],
): Promise<BatchImportResult> {
  const result = await invoke<BatchImportResultDto>("import_wallpapers", { items });
  return {
    ...result,
    imported: normalizeWallpapers(result.imported),
    duplicates: normalizeWallpapers(result.duplicates),
  };
}

export async function detectBrokenWallpapers(): Promise<BrokenWallpaper[]> {
  const result = await invoke<BrokenWallpaperDto[]>("detect_broken_wallpapers");
  return result.map((item) => ({
    ...item,
    wallpaper: normalizeWallpaper(item.wallpaper),
  }));
}

export async function regenerateWallpaperThumbnail(
  id: number,
  probe: ImportProbe,
): Promise<ThumbnailRegenerationResult> {
  const result = await invoke<ThumbnailRegenerationResultDto>(
    "regenerate_wallpaper_thumbnail",
    { id, probe },
  );
  return {
    ...result,
    wallpaper: normalizeWallpaper(result.wallpaper),
  };
}

export function revealWallpaperInExplorer(id: number): Promise<void> {
  return invoke("reveal_wallpaper_in_explorer", { id });
}

export function searchWallhaven(
  args: WallhavenSearchArgs,
): Promise<WallhavenSearchResponse> {
  return invoke<WallhavenSearchResponse>("search_wallhaven", { args });
}

export function searchMotionBgs(
  args: MotionBgsSearchArgs,
): Promise<MotionBgsSearchResponse> {
  return invoke<MotionBgsSearchResponse>("search_motionbgs", { args });
}

export async function downloadWallhavenWallpaper(
  wallpaper: WallhavenWallpaper,
): Promise<DownloadWallhavenResult> {
  const result = await invoke<DownloadWallhavenResultDto>(
    "download_wallhaven_wallpaper",
    { wallpaper },
  );

  return {
    ...result,
    wallpaper: normalizeWallpaper(result.wallpaper),
  };
}

export async function downloadMotionBgsWallpaper(
  wallpaper: MotionBgsWallpaper,
): Promise<DownloadMotionBgsResult> {
  const result = await invoke<DownloadMotionBgsResultDto>(
    "download_motionbgs_wallpaper",
    { wallpaper },
  );

  return {
    ...result,
    wallpaper: normalizeWallpaper(result.wallpaper),
  };
}

export async function removeWallpaper(id: number): Promise<RemoveWallpaperResult> {
  const result = await invoke<RemoveWallpaperResultDto>("remove_wallpaper", { id });
  return {
    wallpaper: normalizeWallpaper(result.wallpaper),
  };
}

export async function setWallpaperFavorite(
  id: number,
  isFavorite: boolean,
): Promise<Wallpaper> {
  return normalizeWallpaper(
    await invoke<WallpaperDto>("set_wallpaper_favorite", {
      id,
      isFavorite,
    }),
  );
}

export async function rotateRandomFavoriteWallpaper(): Promise<Wallpaper> {
  return normalizeWallpaper(
    await invoke<WallpaperDto>("rotate_random_favorite_wallpaper"),
  );
}

export function setWallpaper(
  videoPath: string,
  monitorId?: string | null,
): Promise<string> {
  return invoke<string>("set_wallpaper", { videoPath, monitorId });
}

export function restorePreviousWallpaper(): Promise<string> {
  return invoke<string>("restore_previous_wallpaper");
}

export function listWallpaperHistory(
  limit?: number,
): Promise<WallpaperHistoryEntry[]> {
  return invoke<WallpaperHistoryEntry[]>("list_wallpaper_history", {
    limit: limit ?? null,
  });
}

export function undoWallpaperHistory(): Promise<WallpaperHistoryEntry> {
  return invoke<WallpaperHistoryEntry>("undo_wallpaper_history");
}

export function restoreOriginalWallpaper(): Promise<string> {
  return invoke<string>("restore_original_wallpaper");
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke("open_external_url", { url });
}

export function getStorageStats(): Promise<StorageStats> {
  return invoke<StorageStats>("get_storage_stats");
}

export function clearWallhavenCache(): Promise<StorageCleanupResult> {
  return invoke<StorageCleanupResult>("clear_wallhaven_cache");
}

export function cleanupUnusedThumbnails(): Promise<StorageCleanupResult> {
  return invoke<StorageCleanupResult>("cleanup_unused_thumbnails");
}

export function cleanupMissingLibraryEntries(): Promise<StorageCleanupResult> {
  return invoke<StorageCleanupResult>("cleanup_missing_library_entries");
}

export function setWallpaperPaused(paused: boolean): Promise<void> {
  return invoke("set_wallpaper_paused", { paused });
}

export function setWallpaperSpeed(
  speed: number,
  monitorId?: string | null,
): Promise<void> {
  return invoke("set_wallpaper_speed", { speed, monitorId });
}

export function setWallpaperFitMode(
  mode: string,
  monitorId?: string | null,
): Promise<void> {
  return invoke("set_wallpaper_fit_mode", { mode, monitorId });
}
