import type { BatchImportItem, ImportProbe, Wallpaper } from "$lib/types/wallpaper";
import { probeImage } from "$lib/utils/imageProbe";
import { fileNameFromPath, isImagePath } from "$lib/utils/media";
import { probeVideo } from "$lib/utils/videoProbe";

export interface ImportPreparationOptions {
  onStatus?: (status: string) => void;
}

export async function prepareImportItems(
  files: string[],
  options: ImportPreparationOptions = {},
): Promise<BatchImportItem[]> {
  const items: BatchImportItem[] = [];

  for (const [index, filePath] of files.entries()) {
    const label = fileNameFromPath(filePath);
    const prefix = `Preparing ${index + 1}/${files.length}`;
    const probe = await probeWallpaperFile(filePath, {
      onStatus: (status) => options.onStatus?.(`${prefix}: ${label} - ${status}`),
    });
    items.push({ videoPath: filePath, probe });
  }

  return items;
}

export function probeWallpaperFile(
  path: string,
  options: ImportPreparationOptions = {},
): Promise<ImportProbe> {
  return isImagePath(path) ? probeImage(path, options) : probeVideo(path, options);
}

export function prepareThumbnailRegenerationProbe(
  wallpaper: Wallpaper,
  options: ImportPreparationOptions = {},
): Promise<ImportProbe> {
  options.onStatus?.(`Reading "${wallpaper.title}" video...`);
  return probeVideo(wallpaper.file_path, options);
}
