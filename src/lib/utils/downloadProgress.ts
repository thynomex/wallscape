import { listen } from "@tauri-apps/api/event";
import type {
  DownloadProgressEvent,
  RemoteDownloadState,
  RemoteDownloadStates,
  RemoteDownloadStatus,
} from "$lib/types/downloads";

export const DOWNLOAD_PROGRESS_EVENT = "download-progress";

const STATUSES = new Set<RemoteDownloadStatus>([
  "queued",
  "downloading",
  "retrying",
  "saving",
  "complete",
  "failed",
]);

export function remoteDownloadKey(source: string, sourceId: string) {
  return `${source}:${sourceId}`;
}

export function initialRemoteDownloadState(
  source: string,
  sourceId: string,
  totalBytes: number | null = null,
): RemoteDownloadState {
  return {
    source,
    sourceId,
    downloadKey: remoteDownloadKey(source, sourceId),
    status: "queued",
    attempt: 1,
    receivedBytes: 0,
    totalBytes,
    progress: null,
    message: null,
    canRetry: false,
    updatedAt: Date.now(),
  };
}

export function applyRemoteDownloadProgress(
  states: RemoteDownloadStates,
  progress: DownloadProgressEvent,
): RemoteDownloadStates {
  const normalized = normalizeDownloadProgress(progress);
  return {
    ...states,
    [normalized.downloadKey]: normalized,
  };
}

export function failRemoteDownload(
  states: RemoteDownloadStates,
  source: string,
  sourceId: string,
  error: string,
): RemoteDownloadStates {
  const key = remoteDownloadKey(source, sourceId);
  const previous = states[key] ?? initialRemoteDownloadState(source, sourceId);

  return {
    ...states,
    [key]: {
      ...previous,
      status: "failed",
      error,
      message: error,
      canRetry: true,
      updatedAt: Date.now(),
    },
  };
}

export function isRemoteDownloadActive(state: RemoteDownloadState | null | undefined) {
  return Boolean(
    state &&
      (state.status === "queued" ||
        state.status === "downloading" ||
        state.status === "retrying" ||
        state.status === "saving"),
  );
}

export function remoteDownloadPercent(state: RemoteDownloadState | null | undefined) {
  if (!state || typeof state.progress !== "number") return null;
  return Math.round(state.progress * 100);
}

export function formatRemoteDownloadButtonLabel(
  state: RemoteDownloadState | null | undefined,
  idleLabel: string,
) {
  if (!state) return idleLabel;

  const percent = remoteDownloadPercent(state);
  if (state.status === "queued") return "Queued...";
  if (state.status === "retrying") return `Retrying ${state.attempt}`;
  if (state.status === "saving") return "Saving...";
  if (state.status === "complete") return "Saved";
  if (state.status === "failed") return "Retry";
  if (percent !== null) return `Downloading ${percent}%`;

  return "Downloading...";
}

export function formatRemoteDownloadStatus(
  state: RemoteDownloadState | null | undefined,
  fallback: string,
) {
  if (!state) return fallback;

  const percent = remoteDownloadPercent(state);
  if (state.status === "retrying") return state.message ?? `Retrying ${state.attempt}`;
  if (state.status === "saving") return state.message ?? "Saving download...";
  if (state.status === "complete") return state.message ?? "Download complete";
  if (state.status === "failed") return state.error ?? state.message ?? "Download failed";
  if (percent !== null) return `${fallback} ${percent}%`;

  return state.message ?? fallback;
}

export async function listenForRemoteDownloadProgress(
  downloadKey: string,
  onProgress: (progress: DownloadProgressEvent) => void,
) {
  return listen<DownloadProgressEvent>(DOWNLOAD_PROGRESS_EVENT, (event) => {
    if (event.payload.downloadKey === downloadKey) {
      onProgress(event.payload);
    }
  });
}

function normalizeDownloadProgress(progress: DownloadProgressEvent): RemoteDownloadState {
  const status = normalizeStatus(progress.status);
  const message = progress.message ?? null;

  return {
    ...progress,
    status,
    message,
    error: status === "failed" ? message ?? "Download failed" : undefined,
    canRetry: status === "failed",
    updatedAt: Date.now(),
  };
}

function normalizeStatus(status: string): RemoteDownloadStatus {
  return STATUSES.has(status as RemoteDownloadStatus)
    ? (status as RemoteDownloadStatus)
    : "downloading";
}
