import { readFile, realpath, stat } from 'node:fs/promises';
import { isAbsolute, relative, resolve, sep } from 'node:path';

const CREATED_AT = 1792843200;

// Fixtures describe the real DTOs in src/lib/types/generated.ts. They exist only
// inside the capture process and its fresh browser context.
export async function loadFixtures(directory) {
  const manifest = JSON.parse(await readFile(resolve(directory, 'assets.json'), 'utf8'));
  const entries = Array.isArray(manifest) ? manifest : manifest.wallpapers;
  if (!Array.isArray(entries) || entries.length < 6) {
    throw new Error('assets.json must contain at least six wallpaper entries.');
  }

  const assetsRoot = await realpath(resolve(directory, 'assets'));
  const media = new Map();
  const wallpapers = await Promise.all(entries.map(async (entry, index) => {
    if (!entry.filename || !entry.title || !Number.isInteger(entry.width) ||
        !Number.isInteger(entry.height) || entry.width < 1 || entry.height < 1 ||
        !Array.isArray(entry.tags) || entry.tags.some((tag) => typeof tag !== 'string')) {
      throw new Error(`Invalid wallpaper entry ${index + 1}: expected filename, title, width, height, tags.`);
    }
    const file = await realpath(resolve(assetsRoot, entry.filename));
    const inside = relative(assetsRoot, file);
    if (isAbsolute(inside) || inside === '..' || inside.startsWith(`..${sep}`)) {
      throw new Error(`Wallpaper asset must stay inside assets/: ${entry.filename}`);
    }
    const extension = file.toLowerCase().split('.').pop();
    const contentType = { jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', webp: 'image/webp' }[extension];
    if (!contentType) throw new Error(`Unsupported screenshot asset: ${entry.filename}`);
    const pathname = `/__capture_media/${index + 1}.${extension}`;
    const [body, info] = await Promise.all([readFile(file), stat(file)]);
    media.set(pathname, { body, contentType });
    return {
      id: index + 1,
      title: entry.title,
      file_path: pathname,
      thumbnail_path: pathname,
      tags: entry.tags,
      width: entry.width,
      height: entry.height,
      fps: 0,
      duration_ms: 0,
      file_size_bytes: info.size,
      created_at: CREATED_AT - index * 3600,
      media_type: 'image',
      source: null,
      source_id: null,
      is_favorite: entry.isFavorite ?? index < 3,
    };
  }));

  const collectionNames = entries.map((entry, index) =>
    typeof entry.collection === 'string' && entry.collection.trim()
      ? entry.collection.trim()
      : index < 3 ? 'Daily rotation' : 'Focus');
  const collections = [...new Set(collectionNames)].map((name, index) => ({
    id: index + 1,
    name,
    created_at: CREATED_AT,
    wallpaper_count: collectionNames.filter((candidate) => candidate === name).length,
  }));
  const memberships = wallpapers.map((wallpaper, index) => ({
    collection_id: collections.find((collection) => collection.name === collectionNames[index]).id,
    wallpaper_id: wallpaper.id,
  }));

  // Discover results reuse the local artwork so capture never contacts Wallhaven
  // or MotionBGS. IDs, counts, and URLs are sample values in the real DTO shapes.
  const wallhavenIds = ['8o2jv1', 'kx6m3q', 'p9z2wd', '5w7e1y', 'zm3r8k', 'gj4l6o'];
  const wallhaven = wallpapers.map((wallpaper, index) => {
    const id = wallhavenIds[index] ?? `capture${index + 1}`;
    return {
      id,
      url: `https://wallhaven.cc/w/${id}`,
      short_url: `https://whvn.cc/${id}`,
      views: 4200 + index * 1375,
      favorites: 180 + index * 41,
      purity: 'sfw',
      category: 'general',
      dimension_x: wallpaper.width,
      dimension_y: wallpaper.height,
      resolution: `${wallpaper.width}x${wallpaper.height}`,
      ratio: (wallpaper.width / wallpaper.height).toFixed(2),
      file_size: wallpaper.file_size_bytes,
      file_type: media.get(wallpaper.file_path).contentType,
      created_at: new Date((CREATED_AT - index * 86400) * 1000).toISOString().slice(0, 19).replace('T', ' '),
      colors: [],
      path: wallpaper.file_path,
      thumbs: { large: wallpaper.thumbnail_path, original: wallpaper.file_path, small: wallpaper.thumbnail_path },
    };
  });
  const motionBgs = wallpapers.map((wallpaper, index) => {
    const id = String(48210 + index * 7);
    const slug = wallpaper.title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    const downloads = [
      { quality: '4K', url: `https://motionbgs.com/dl/4k/${id}`, width: 3840, height: 2160, fileSize: (38 + index * 6) * 1024 * 1024 },
      { quality: 'HD', url: `https://motionbgs.com/dl/hd/${id}`, width: 1920, height: 1080, fileSize: (12 + index * 2) * 1024 * 1024 },
    ];
    return {
      id,
      slug,
      title: wallpaper.title,
      url: `https://motionbgs.com/${slug}`,
      thumbnailUrl: wallpaper.thumbnail_path,
      previewVideoUrl: null,
      quality: '4K',
      width: 3840,
      height: 2160,
      fileSize: downloads[0].fileSize,
      tags: wallpaper.tags,
      downloads,
    };
  });

  return {
    media,
    data: {
      wallpapers,
      collections,
      memberships,
      monitors: [
        { id: 'capture-primary', name: 'Studio display', x: 0, y: 0, width: 3840, height: 2160, is_primary: true },
        { id: 'capture-secondary', name: 'Display 2', x: 3840, y: 0, width: 2560, height: 1440, is_primary: false },
      ],
      wallhaven: {
        data: wallhaven,
        meta: { current_page: 1, last_page: 4, per_page: wallhaven.length, total: wallhaven.length * 4, seed: null },
      },
      motionBgs: { data: motionBgs, meta: { currentPage: 1, hasNextPage: true } },
      storageStats: {
        total_cache_bytes: 186_400_000, wallhaven_cache_bytes: 172_900_000,
        wallhaven_cache_files: 9, unused_wallhaven_cache_bytes: 41_300_000,
        unused_wallhaven_cache_files: 3, wallhaven_library_entries: 6,
        thumbnail_cache_bytes: 13_500_000, thumbnail_cache_files: 14,
        unused_thumbnail_bytes: 2_100_000, unused_thumbnail_files: 2,
        missing_library_entries: 0,
      },
      settings: {
        launchAtStartup: false, startMinimized: false, closeToTray: true,
        minimizeToTray: false, restoreLastWallpaper: true,
        favoriteRotationEnabled: false, favoriteRotationIntervalMinutes: 30,
        favoriteRotationOnStartup: false, autoPauseEnabled: true,
        autoPauseOnBattery: true, autoPauseFullscreenApps: true,
        autoPauseOccluded: true, autoPauseRemoteSession: true,
        autoPauseDisplaySleep: true, diagnosticWebhookUrl: null,
        diagnosticAutoSend: false, defaultPlaybackSpeed: 1,
        defaultFitMode: 'fit', defaultFpsCap: null,
      },
    },
  };
}
