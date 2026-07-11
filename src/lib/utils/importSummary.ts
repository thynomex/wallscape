import type { BatchImportResult } from "$lib/types/wallpaper";
import { mediaTypeFromPath, mediaTypeLabel } from "$lib/utils/media";

interface BatchImportMessageOptions {
  suffix?: string;
}

export function formatBatchImportMessage(
  result: BatchImportResult,
  options: BatchImportMessageOptions = {},
) {
  const importedCount = result.imported.length;
  const duplicateCount = result.duplicates.length;
  const failedCount = result.failed.length;
  const total = importedCount + duplicateCount + failedCount;

  if (total === 1) {
    const imported = result.imported[0];
    if (imported) {
      return withSuffix(`${mediaTypeLabel(imported.media_type)} imported`, options.suffix);
    }

    const duplicate = result.duplicates[0];
    if (duplicate) {
      return withSuffix(
        `${mediaTypeLabel(duplicate.media_type)} already in library`,
        options.suffix,
      );
    }

    const failed = result.failed[0];
    if (failed) {
      return withSuffix(
        `${mediaTypeLabel(mediaTypeFromPath(failed.path))} import failed`,
        options.suffix,
      );
    }
  }

  const parts = [];
  if (importedCount > 0) parts.push(`${importedCount} imported`);
  if (duplicateCount > 0) {
    parts.push(`${duplicateCount} duplicate${duplicateCount === 1 ? "" : "s"} skipped`);
  }
  if (failedCount > 0) parts.push(`${failedCount} failed`);

  return withSuffix(parts.length ? parts.join(", ") : "No wallpapers imported", options.suffix);
}

function withSuffix(message: string, suffix = "") {
  if (!suffix || message.endsWith(suffix)) return message;
  return `${message}${suffix}`;
}
