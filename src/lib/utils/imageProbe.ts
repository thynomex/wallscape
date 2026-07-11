import { convertFileSrc } from "@tauri-apps/api/core";
import type { ImportProbe } from "$lib/types/wallpaper";

interface ProbeImageOptions {
  onStatus?: (status: string) => void;
}

export async function probeImage(
  filePath: string,
  options: ProbeImageOptions = {},
): Promise<ImportProbe> {
  const warnings: string[] = [];
  const image = new Image();

  try {
    options.onStatus?.("Reading image metadata...");
    image.decoding = "async";
    image.src = convertFileSrc(filePath);
    await image.decode();

    return {
      width: image.naturalWidth || undefined,
      height: image.naturalHeight || undefined,
      fps: 0,
      durationMs: 0,
      warnings,
    };
  } catch (error) {
    warnings.push(`Image metadata probe failed: ${String(error)}`);
    return {
      fps: 0,
      durationMs: 0,
      warnings,
    };
  }
}
