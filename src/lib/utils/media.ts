import { convertFileSrc } from "@tauri-apps/api/core";
import type { MediaType } from "$lib/types/wallpaper";

export const VIDEO_EXTENSIONS = ["mp4", "mov", "m4v", "webm", "mkv", "avi", "wmv"];
export const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "bmp", "webp"];
export const WALLPAPER_EXTENSIONS = [...VIDEO_EXTENSIONS, ...IMAGE_EXTENSIONS];

export function resolveMediaSrc(src: string | null | undefined): string | null {
  if (!src) return null;
  if (
    src.startsWith("http://") ||
    src.startsWith("https://") ||
    src.startsWith("data:") ||
    src.startsWith("blob:") ||
    src.startsWith("file:")
  ) {
    return src;
  }
  return convertFileSrc(src);
}

export function resolveWallpaperThumbnailSrc(
  wallpaper:
    | {
        media_type?: string | null;
        thumbnail_path?: string | null;
      }
    | null
    | undefined,
): string | null {
  if (!wallpaper?.thumbnail_path) return null;
  if (isGifSrc(wallpaper.thumbnail_path)) return null;
  return resolveMediaSrc(wallpaper.thumbnail_path);
}

export function resolveWallpaperPreviewSrc(
  wallpaper:
    | {
        media_type?: string | null;
        file_path?: string | null;
        thumbnail_path?: string | null;
      }
    | null
    | undefined,
): string | null {
  if (!wallpaper) return null;

  const src =
    wallpaper.media_type === "image" && wallpaper.file_path
      ? wallpaper.file_path
      : wallpaper.thumbnail_path;

  if (isGifSrc(src)) return null;
  return resolveMediaSrc(src);
}

export function isGifSrc(src: string | null | undefined) {
  if (!src) return false;
  const path = src.split(/[?#]/, 1)[0] ?? src;
  return path.toLowerCase().endsWith(".gif");
}

export function fileNameFromPath(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

export function isImagePath(path: string) {
  return IMAGE_EXTENSIONS.includes(pathExtension(path));
}

export function mediaTypeFromPath(path: string): MediaType {
  return isImagePath(path) ? "image" : "video";
}

export function mediaTypeLabel(mediaType: string | null | undefined) {
  return mediaType === "image" ? "Image" : "Video";
}

/** Short media descriptor shared by the hero, cards, and detail view. */
export function wallpaperMediaLabel(
  wallpaper:
    | {
        media_type?: string | null;
        fps?: number | null;
        duration_ms?: number | null;
      }
    | null
    | undefined,
): string {
  if (!wallpaper) return "";
  if (wallpaper.media_type === "image") return "Image";

  const fps = Math.round(wallpaper.fps ?? 0);
  if (fps > 0) return `${fps}fps video`;

  const seconds = Math.round((wallpaper.duration_ms ?? 0) / 1000);
  if (seconds > 0) return `${seconds}s video`;

  return "Video";
}

/** Formats pixel dimensions as `1920×1080`, or an empty string when unknown. */
export function formatDimensions(
  width: number | null | undefined,
  height: number | null | undefined,
): string {
  return width && height && width > 0 && height > 0 ? `${width}×${height}` : "";
}

/** Normalizes API resolution strings such as `1920x1080` to `1920×1080`. */
export function formatResolution(resolution: string | null | undefined): string {
  return (resolution ?? "").replace(/(\d)\s*x\s*(\d)/i, "$1×$2");
}

/** Formats a byte count as whole megabytes (minimum `1MB`), or "" when unknown. */
export function formatFileSizeMb(bytes: number | null | undefined): string {
  return bytes && bytes > 0 ? `${Math.max(1, Math.round(bytes / 1024 / 1024))}MB` : "";
}

function pathExtension(path: string) {
  const cleanPath = path.split(/[?#]/, 1)[0] ?? path;
  return cleanPath.split(".").pop()?.toLowerCase() ?? "";
}
