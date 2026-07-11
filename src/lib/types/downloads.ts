import type { DownloadProgressEvent as GeneratedDownloadProgressEvent } from "./generated";

export type RemoteDownloadStatus =
  | "queued"
  | "downloading"
  | "retrying"
  | "saving"
  | "complete"
  | "failed";

export interface DownloadProgressEvent
  extends Omit<GeneratedDownloadProgressEvent, "status"> {
  status: string;
}

export interface RemoteDownloadState
  extends Omit<DownloadProgressEvent, "status"> {
  status: RemoteDownloadStatus;
  error?: string;
  canRetry: boolean;
  updatedAt: number;
}

export type RemoteDownloadStates = Record<string, RemoteDownloadState>;
