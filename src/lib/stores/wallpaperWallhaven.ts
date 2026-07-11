import {
  downloadWallhavenWallpaper,
  searchWallhaven as searchWallhavenApi,
} from "$lib/api/wallpaperApi";
import type {
  StoreActionResult,
  WallhavenMeta,
  WallhavenSearchArgs,
  WallhavenWallpaper,
  Wallpaper,
} from "$lib/types/wallpaper";
import type { RemoteDownloadStates } from "$lib/types/downloads";
import {
  applyRemoteDownloadProgress,
  failRemoteDownload,
  formatRemoteDownloadStatus,
  initialRemoteDownloadState,
  isRemoteDownloadActive,
  listenForRemoteDownloadProgress,
  remoteDownloadKey,
} from "$lib/utils/downloadProgress";

const WALLHAVEN_DOWNLOAD_SOURCE = "wallhaven";

export interface WallpaperWallhavenState {
  wallpapers: Wallpaper[];
  featured: Wallpaper | null;
  importStatus: string | null;
  importWarnings: string[];
  wallhavenResults: WallhavenWallpaper[];
  wallhavenMeta: WallhavenMeta | null;
  wallhavenLoading: boolean;
  remoteDownloads: RemoteDownloadStates;
  wallhavenError: string | null;
}

export interface WallpaperWallhavenActions {
  reloadWallpapers: () => Promise<void>;
}

export async function searchWallhavenIntoState(
  state: WallpaperWallhavenState,
  args: WallhavenSearchArgs,
) {
  state.wallhavenLoading = true;
  state.wallhavenError = null;
  try {
    const response = await searchWallhavenApi(args);
    state.wallhavenResults = response.data;
    state.wallhavenMeta = response.meta;
  } catch (e) {
    state.wallhavenError = `Failed to search Wallhaven: ${e}`;
    console.error(e);
  } finally {
    state.wallhavenLoading = false;
  }
}

export async function downloadWallhavenIntoState(
  state: WallpaperWallhavenState,
  wallpaper: WallhavenWallpaper,
  actions: WallpaperWallhavenActions,
): Promise<StoreActionResult<Wallpaper>> {
  const downloadKey = remoteDownloadKey(WALLHAVEN_DOWNLOAD_SOURCE, wallpaper.id);
  const currentDownload = state.remoteDownloads[downloadKey];

  if (isRemoteDownloadActive(currentDownload)) {
    return {
      ok: false,
      error: `Wallhaven ${wallpaper.id} is already downloading.`,
    };
  }

  state.remoteDownloads = {
    ...state.remoteDownloads,
    [downloadKey]: initialRemoteDownloadState(
      WALLHAVEN_DOWNLOAD_SOURCE,
      wallpaper.id,
      wallpaper.file_size > 0 ? wallpaper.file_size : null,
    ),
  };
  state.wallhavenError = null;
  state.importStatus = `Downloading Wallhaven ${wallpaper.id}...`;
  state.importWarnings = [];

  let unlistenProgress: (() => void) | null = null;

  try {
    try {
      unlistenProgress = await listenForRemoteDownloadProgress(
        downloadKey,
        (progress) => {
          state.remoteDownloads = applyRemoteDownloadProgress(
            state.remoteDownloads,
            progress,
          );
          state.importStatus = formatRemoteDownloadStatus(
            state.remoteDownloads[downloadKey],
            `Downloading Wallhaven ${wallpaper.id}...`,
          );
        },
      );
    } catch (error) {
      console.warn("Download progress listener unavailable:", error);
    }

    const result = await downloadWallhavenWallpaper(wallpaper);
    const completedDownload = state.remoteDownloads[downloadKey];
    const fallbackBytes = Math.max(0, wallpaper.file_size);

    state.remoteDownloads = applyRemoteDownloadProgress(state.remoteDownloads, {
      source: WALLHAVEN_DOWNLOAD_SOURCE,
      sourceId: wallpaper.id,
      downloadKey,
      status: "complete",
      attempt: completedDownload?.attempt ?? 1,
      receivedBytes: completedDownload?.receivedBytes ?? fallbackBytes,
      totalBytes: completedDownload?.totalBytes ?? (fallbackBytes || null),
      progress: 1,
      message: "Download complete",
    });

    await actions.reloadWallpapers();
    const importedFromList =
      state.wallpapers.find((item) => item.id === result.wallpaper.id) ??
      result.wallpaper;
    state.featured = importedFromList;
    state.importStatus = `"${importedFromList.title}" added to your library.`;
    return { ok: true, value: importedFromList };
  } catch (e) {
    const error = `Failed to download Wallhaven wallpaper: ${e}`;
    state.remoteDownloads = failRemoteDownload(
      state.remoteDownloads,
      WALLHAVEN_DOWNLOAD_SOURCE,
      wallpaper.id,
      error,
    );
    state.wallhavenError = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  } finally {
    unlistenProgress?.();
  }
}
