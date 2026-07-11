import { open } from "@tauri-apps/plugin-dialog";
import { WALLPAPER_EXTENSIONS } from "$lib/utils/media";

export async function selectWallpaperFiles() {
  const selected = await open({
    multiple: true,
    title: "Import wallpapers",
    filters: [
      {
        name: "Wallpaper files",
        extensions: WALLPAPER_EXTENSIONS,
      },
    ],
  });

  return selectedPaths(selected);
}

export async function selectWallpaperFolders() {
  const selected = await open({
    directory: true,
    multiple: true,
    title: "Import wallpaper folders",
  });

  return selectedPaths(selected);
}

function selectedPaths(selected: string | string[] | null) {
  return Array.isArray(selected)
    ? selected
    : typeof selected === "string"
      ? [selected]
      : [];
}
