import type { MotionBgsWallpaper, Wallpaper } from "$lib/types/wallpaper";

type MotionBgsDownloadOption = MotionBgsWallpaper["downloads"][number];

const QUALITY_ORDER = ["4K", "HD"];

export function motionBgsDownloadSourceId(
  wallpaper: Pick<MotionBgsWallpaper, "id" | "quality"> | Pick<Wallpaper, "source_id">,
) {
  if ("source_id" in wallpaper) {
    return wallpaper.source_id ?? "";
  }

  const quality =
    wallpaper.quality.replace(/[^a-z0-9]/gi, "").toLowerCase() || "hd";
  return `${wallpaper.id}-${quality}`;
}

export function motionBgsBaseSourceId(sourceId: string | null | undefined) {
  return sourceId?.replace(/-(?:4k|hd)$/i, "") ?? null;
}

export function motionBgsDownloadOptions(wallpaper: MotionBgsWallpaper) {
  const unique = new Map<string, MotionBgsDownloadOption>();

  for (const download of wallpaper.downloads) {
    if (!QUALITY_ORDER.includes(download.quality)) continue;
    unique.set(download.quality, download);
  }

  return QUALITY_ORDER.map((quality) => unique.get(quality)).filter(
    (download): download is MotionBgsDownloadOption => Boolean(download),
  );
}

export function motionBgsWallpaperWithDownload(
  wallpaper: MotionBgsWallpaper,
  download: MotionBgsDownloadOption | null,
): MotionBgsWallpaper {
  if (!download) return wallpaper;

  return {
    ...wallpaper,
    quality: download.quality,
    width: download.width,
    height: download.height,
    fileSize: download.fileSize,
  };
}
