// Playwright serializes this function into the page before any application code.
// The shape follows @tauri-apps/api/mocks: mockIPC and mockWindows. Keeping the
// initializer synchronous prevents Svelte from calling IPC before mocks exist.
// It is never imported by the app or included in its production build.
export function installCaptureBridge(fixtures) {
  const callbacks = new Map();
  const listeners = new Map();
  let nextCallbackId = 1;
  const state = structuredClone(fixtures);
  const calls = [];
  const unexpected = [];
  const toUrl = (path) => new URL(path, window.location.origin).href;
  state.wallpapers = state.wallpapers.map((wallpaper) => ({
    ...wallpaper,
    file_path: toUrl(wallpaper.file_path),
    thumbnail_path: toUrl(wallpaper.thumbnail_path),
  }));
  state.wallhaven.data = state.wallhaven.data.map((wallpaper) => ({
    ...wallpaper,
    path: toUrl(wallpaper.path),
    thumbs: Object.fromEntries(Object.entries(wallpaper.thumbs).map(([size, path]) => [size, toUrl(path)])),
  }));
  state.motionBgs.data = state.motionBgs.data.map((wallpaper) => ({
    ...wallpaper,
    thumbnailUrl: toUrl(wallpaper.thumbnailUrl),
  }));
  const registerCallback = (callback, once = false) => {
    const id = nextCallbackId++;
    callbacks.set(id, (payload) => {
      if (once) callbacks.delete(id);
      return callback?.(payload);
    });
    return id;
  };

  async function invoke(command, args = {}) {
    calls.push(command);
    let result;
    switch (command) {
      case 'list_wallpapers': result = state.wallpapers; break;
      case 'get_monitors': result = state.monitors; break;
      case 'list_collections': result = state.collections; break;
      case 'list_collection_memberships': result = state.memberships; break;
      case 'list_saved_filters':
      case 'list_wallpaper_history': result = []; break;
      case 'get_original_wallpaper_backup':
      case 'get_previous_wallpaper': result = { path: null, can_restore: false }; break;
      case 'search_wallpapers': {
        // Matches the library search contract: title or tag, case-insensitive.
        const query = String(args.query ?? '').trim().toLowerCase();
        result = state.wallpapers.filter((wallpaper) =>
          [wallpaper.title, ...wallpaper.tags].some((text) => text.toLowerCase().includes(query)));
        break;
      }
      case 'search_wallhaven': result = state.wallhaven; break;
      case 'search_motionbgs': result = state.motionBgs; break;
      case 'get_storage_stats': result = state.storageStats; break;
      case 'get_settings': result = state.settings; break;
      case 'get_runtime_status':
        result = { active: false, paused: false, autoPaused: false, playbackSpeed: null, fitMode: null };
        break;
      case 'plugin:window|is_visible': result = true; break;
      case 'plugin:window|is_minimized': result = false; break;
      case 'plugin:event|listen':
        listeners.set(args.handler, args.event);
        result = args.handler;
        break;
      case 'plugin:event|unlisten':
        listeners.delete(args.eventId ?? args.id);
        result = null;
        break;
      default:
        unexpected.push(command);
        throw new Error(`Screenshot bridge does not implement ${command}. Native commands are unavailable.`);
    }
    return structuredClone(result);
  }

  window.__TAURI_INTERNALS__ = {
    invoke,
    convertFileSrc: toUrl,
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
    transformCallback: registerCallback,
    unregisterCallback: (id) => callbacks.delete(id),
    runCallback: (id, payload) => callbacks.get(id)?.(payload),
    callbacks,
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (_event, id) => {
      callbacks.delete(id);
      listeners.delete(id);
    },
  };
  window.__WALLSCAPE_CAPTURE__ = { calls, unexpected };
}
