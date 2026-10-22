import {
  downloadMotionBgsWallpaper,
  searchMotionBgs as searchMotionBgsApi,
} from "$lib/api/wallpaperApi";
import type {
  MotionBgsMeta,
  MotionBgsSearchArgs,
  MotionBgsWallpaper,
  StoreActionResult,
  Wallpaper,
} from "$lib/types/wallpaper";
import type { RemoteDownloadStates } from "$lib/types/downloads";
import { latestRequest } from "$lib/stores/latestRequest";
import {
  failedRemoteDownloadState,
  formatRemoteDownloadStatus,
  initialRemoteDownloadState,
  isRemoteDownloadActive,
  listenForRemoteDownloadProgress,
  normalizeDownloadProgress,
  remoteDownloadKey,
} from "$lib/utils/downloadProgress";
import { motionBgsDownloadSourceId } from "$lib/utils/motionbgsDownload";

const MOTIONBGS_DOWNLOAD_SOURCE = "motionbgs";
const beginSearch = latestRequest();

export interface WallpaperMotionBgsState {
  wallpapers: Wallpaper[];
  featured: Wallpaper | null;
  importStatus: string | null;
  importWarnings: string[];
  motionBgsResults: MotionBgsWallpaper[];
  motionBgsMeta: MotionBgsMeta | null;
  motionBgsLoading: boolean;
  remoteDownloads: RemoteDownloadStates;
  motionBgsError: string | null;
}

export interface WallpaperMotionBgsActions {
  reloadWallpapers: () => Promise<void>;
}

export async function searchMotionBgsIntoState(
  state: WallpaperMotionBgsState,
  args: MotionBgsSearchArgs,
) {
  const isCurrent = beginSearch(state);
  state.motionBgsLoading = true;
  state.motionBgsError = null;
  try {
    const response = await searchMotionBgsApi(args);
    if (!isCurrent()) return;
    state.motionBgsResults = response.data;
    state.motionBgsMeta = response.meta;
  } catch (e) {
    if (!isCurrent()) return;
    state.motionBgsError = `Failed to search MotionBGS: ${e}`;
    console.error(e);
  } finally {
    if (isCurrent()) state.motionBgsLoading = false;
  }
}

export async function downloadMotionBgsIntoState(
  state: WallpaperMotionBgsState,
  wallpaper: MotionBgsWallpaper,
  actions: WallpaperMotionBgsActions,
): Promise<StoreActionResult<Wallpaper>> {
  const downloadSourceId = motionBgsDownloadSourceId(wallpaper);
  const downloadKey = remoteDownloadKey(MOTIONBGS_DOWNLOAD_SOURCE, downloadSourceId);
  const currentDownload = state.remoteDownloads[downloadKey];

  if (isRemoteDownloadActive(currentDownload)) {
    return {
      ok: false,
      error: `${wallpaper.title} is already downloading.`,
    };
  }

  state.remoteDownloads[downloadKey] = initialRemoteDownloadState(
    MOTIONBGS_DOWNLOAD_SOURCE,
    downloadSourceId,
    wallpaper.fileSize > 0 ? wallpaper.fileSize : null,
  );
  state.motionBgsError = null;
  state.importStatus = `Downloading MotionBGS ${wallpaper.title}...`;
  state.importWarnings = [];

  let unlistenProgress: (() => void) | null = null;

  try {
    try {
      unlistenProgress = await listenForRemoteDownloadProgress(
        downloadKey,
        (progress) => {
          state.remoteDownloads[downloadKey] = normalizeDownloadProgress(progress);
          state.importStatus = formatRemoteDownloadStatus(
            state.remoteDownloads[downloadKey],
            `Downloading MotionBGS ${wallpaper.title}...`,
          );
        },
      );
    } catch (error) {
      console.warn("Download progress listener unavailable:", error);
    }

    const result = await downloadMotionBgsWallpaper(wallpaper);
    const completedDownload = state.remoteDownloads[downloadKey];
    const fallbackBytes = Math.max(0, wallpaper.fileSize);

    state.remoteDownloads[downloadKey] = normalizeDownloadProgress({
      source: MOTIONBGS_DOWNLOAD_SOURCE,
      sourceId: downloadSourceId,
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
    const error = `Failed to download MotionBGS wallpaper: ${e}`;
    state.remoteDownloads[downloadKey] = failedRemoteDownloadState(
      state.remoteDownloads[downloadKey],
      MOTIONBGS_DOWNLOAD_SOURCE,
      downloadSourceId,
      error,
    );
    state.motionBgsError = error;
    state.importStatus = null;
    console.error(e);
    return { ok: false, error };
  } finally {
    unlistenProgress?.();
  }
}
