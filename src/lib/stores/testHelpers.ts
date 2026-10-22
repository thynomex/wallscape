import type { Wallpaper } from "$lib/types/wallpaper";

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((accept, decline) => {
    resolve = accept;
    reject = decline;
  });
  return { promise, resolve, reject };
}

export function wallpaper(id: number, isFavorite = false): Wallpaper {
  return {
    id,
    title: `Wallpaper ${id}`,
    file_path: `C:/wallpapers/${id}.jpg`,
    thumbnail_path: null,
    tags: [],
    width: 1920,
    height: 1080,
    fps: 0,
    duration_ms: 0,
    media_type: "image",
    source: null,
    source_id: null,
    is_favorite: isFavorite,
  };
}
