import { listMonitors } from "$lib/api/wallpaperApi";
import {
  rotateRandomFavoriteInState,
  setFavoriteInState,
} from "$lib/stores/wallpaperFavorites";
import {
  createCollectionInState,
  deleteCollectionFromState,
  deleteSavedFilterFromState,
  loadOrganizationState,
  saveFilterInState,
  setCollectionMembershipInState,
} from "$lib/stores/wallpaperOrganization";
import {
  detectBrokenWallpapersInState,
  importWallpaperIntoState,
  importWallpapersIntoState,
  regenerateThumbnailInState,
  scanImportPathsForImport,
} from "$lib/stores/wallpaperImports";
import {
  applyWallpaperInState,
  cancelAppliedWallpaperInState,
  loadBackupStatusIntoState,
  loadWallpaperHistoryIntoState,
  loadPreviousWallpaperStatusIntoState,
  restoreOriginalWallpaperInState,
  restorePreviousWallpaperInState,
  undoWallpaperHistoryInState,
} from "$lib/stores/wallpaperLifecycle";
import {
  loadWallpapersIntoState,
  removeWallpaperFromState,
  replaceWallpaperInState,
  revealWallpaperInExplorerFromState,
  searchWallpapersInState,
} from "$lib/stores/wallpaperLibrary";
import {
  downloadMotionBgsIntoState,
  searchMotionBgsIntoState,
} from "$lib/stores/wallpaperMotionBgs";
import {
  downloadWallhavenIntoState,
  searchWallhavenIntoState,
} from "$lib/stores/wallpaperWallhaven";
import type {
  BatchImportItem,
  BatchImportResult,
  BrokenWallpaper,
  Collection,
  CollectionMembership,
  ImportProbe,
  ImportScanResult,
  Monitor,
  MotionBgsMeta,
  MotionBgsSearchArgs,
  MotionBgsWallpaper,
  SavedFilter,
  SavedFilterPayload,
  SavedFilterType,
  StoreActionResult,
  ThumbnailRegenerationResult,
  WallhavenMeta,
  WallhavenSearchArgs,
  WallhavenWallpaper,
  Wallpaper,
  WallpaperBackupStatus,
  WallpaperHistoryEntry,
  WallpaperRestoreStatus,
} from "$lib/types/wallpaper";
import type { RemoteDownloadStates } from "$lib/types/downloads";

/** Reactive app state — Svelte 5 runes in a module (`.svelte.ts`). */
class WallpaperStore {
  wallpapers = $state<Wallpaper[]>([]);
  monitors = $state<Monitor[]>([]);
  collections = $state<Collection[]>([]);
  collectionMemberships = $state<CollectionMembership[]>([]);
  savedFilters = $state<SavedFilter[]>([]);
  selected = $state<Wallpaper | null>(null);
  featured = $state<Wallpaper | null>(null);
  loading = $state(false);
  importing = $state(false);
  importStatus = $state<string | null>(null);
  importWarnings = $state<string[]>([]);
  error = $state<string | null>(null);
  wallhavenResults = $state<WallhavenWallpaper[]>([]);
  wallhavenMeta = $state<WallhavenMeta | null>(null);
  wallhavenLoading = $state(false);
  remoteDownloads = $state<RemoteDownloadStates>({});
  wallhavenError = $state<string | null>(null);
  motionBgsResults = $state<MotionBgsWallpaper[]>([]);
  motionBgsMeta = $state<MotionBgsMeta | null>(null);
  motionBgsLoading = $state(false);
  motionBgsError = $state<string | null>(null);
  backup = $state<WallpaperBackupStatus>({
    path: null,
    can_restore: false,
  });
  history = $state<WallpaperHistoryEntry[]>([]);
  previous = $state<WallpaperRestoreStatus>({
    path: null,
    can_restore: false,
  });

  async loadMonitors() {
    try {
      this.monitors = await listMonitors();
    } catch (e) {
      console.error("Failed to load monitors:", e);
    }
  }

  async loadWallpapers() {
    await loadWallpapersIntoState(this);
  }

  async loadOrganization() {
    await loadOrganizationState(this);
  }

  async loadBackupStatus() {
    await loadBackupStatusIntoState(this);
  }

  async loadPreviousWallpaperStatus() {
    await loadPreviousWallpaperStatusIntoState(this);
  }

  async loadWallpaperHistory() {
    await loadWallpaperHistoryIntoState(this);
  }

  async search(query: string) {
    await searchWallpapersInState(this, query);
  }

  async importWallpaper(
    videoPath: string,
    probe?: ImportProbe,
  ): Promise<StoreActionResult<Wallpaper>> {
    return importWallpaperIntoState(this, videoPath, probe, {
      reloadWallpapers: () => this.loadWallpapers(),
    });
  }

  scanImportPaths(paths: string[]): Promise<ImportScanResult> {
    return scanImportPathsForImport(paths);
  }

  async importWallpapers(
    items: BatchImportItem[],
  ): Promise<StoreActionResult<BatchImportResult>> {
    return importWallpapersIntoState(this, items, {
      reloadWallpapers: () => this.loadWallpapers(),
    });
  }

  async detectBrokenWallpapers(): Promise<StoreActionResult<BrokenWallpaper[]>> {
    return detectBrokenWallpapersInState(this);
  }

  async revealInExplorer(wallpaper: Wallpaper): Promise<StoreActionResult> {
    return revealWallpaperInExplorerFromState(this, wallpaper);
  }

  async regenerateThumbnail(
    wallpaper: Wallpaper,
    probe: ImportProbe,
  ): Promise<StoreActionResult<ThumbnailRegenerationResult>> {
    return regenerateThumbnailInState(this, wallpaper, probe, {
      replaceWallpaper: (updated) => this.replaceWallpaper(updated),
    });
  }

  async searchWallhaven(args: WallhavenSearchArgs) {
    await searchWallhavenIntoState(this, args);
  }

  async downloadWallhaven(
    wallpaper: WallhavenWallpaper,
  ): Promise<StoreActionResult<Wallpaper>> {
    return downloadWallhavenIntoState(this, wallpaper, {
      reloadWallpapers: () => this.loadWallpapers(),
    });
  }

  async searchMotionBgs(args: MotionBgsSearchArgs) {
    await searchMotionBgsIntoState(this, args);
  }

  async downloadMotionBgs(
    wallpaper: MotionBgsWallpaper,
  ): Promise<StoreActionResult<Wallpaper>> {
    return downloadMotionBgsIntoState(this, wallpaper, {
      reloadWallpapers: () => this.loadWallpapers(),
    });
  }

  async removeWallpaper(wallpaper: Wallpaper): Promise<StoreActionResult<Wallpaper>> {
    return removeWallpaperFromState(this, wallpaper);
  }

  async setFavorite(
    wallpaper: Wallpaper,
    isFavorite: boolean,
  ): Promise<StoreActionResult<Wallpaper>> {
    return setFavoriteInState(this, wallpaper, isFavorite, {
      replaceWallpaper: (updated) => this.replaceWallpaper(updated),
    });
  }

  async createCollection(name: string): Promise<StoreActionResult<Collection>> {
    return createCollectionInState(this, name);
  }

  async deleteCollection(collection: Collection): Promise<StoreActionResult> {
    return deleteCollectionFromState(this, collection);
  }

  async setCollectionMembership(
    collection: Collection,
    wallpaper: Wallpaper,
    inCollection: boolean,
  ): Promise<StoreActionResult<Collection>> {
    return setCollectionMembershipInState(this, collection, wallpaper, inCollection);
  }

  async saveFilter(
    name: string,
    filterType: SavedFilterType,
    payload: SavedFilterPayload,
  ): Promise<StoreActionResult<SavedFilter>> {
    return saveFilterInState(this, name, filterType, payload);
  }

  async deleteSavedFilter(filter: SavedFilter): Promise<StoreActionResult> {
    return deleteSavedFilterFromState(this, filter);
  }

  async rotateRandomFavorite(): Promise<StoreActionResult<Wallpaper>> {
    return rotateRandomFavoriteInState(this, {
      replaceWallpaper: (updated) => this.replaceWallpaper(updated),
      loadBackupStatus: () => this.loadBackupStatus(),
      loadPreviousWallpaperStatus: () => this.loadPreviousWallpaperStatus(),
      loadWallpaperHistory: () => this.loadWallpaperHistory(),
    });
  }

  async apply(
    wallpaper: Wallpaper,
    monitorId: string | null = null,
  ): Promise<StoreActionResult> {
    return applyWallpaperInState(this, wallpaper, monitorId);
  }

  async restorePreviousWallpaper(): Promise<StoreActionResult> {
    return restorePreviousWallpaperInState(this);
  }

  async undoWallpaperHistory(): Promise<StoreActionResult<WallpaperHistoryEntry>> {
    return undoWallpaperHistoryInState(this);
  }

  async cancelAppliedWallpaper(fallbackWallpaper?: Wallpaper): Promise<StoreActionResult> {
    return cancelAppliedWallpaperInState(this, fallbackWallpaper);
  }

  async restoreOriginalWallpaper(): Promise<StoreActionResult> {
    return restoreOriginalWallpaperInState(this);
  }

  private replaceWallpaper(updated: Wallpaper) {
    replaceWallpaperInState(this, updated);
  }
}

export const store = new WallpaperStore();
