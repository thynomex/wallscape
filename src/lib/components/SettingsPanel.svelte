<script lang="ts">
  import { onMount } from "svelte";
  import {
    cleanupMissingLibraryEntries,
    cleanupUnusedThumbnails,
    clearWallhavenCache,
    getStorageStats,
  } from "$lib/api/wallpaperApi";
  import {
    DEFAULT_SETTINGS,
    normalizeFavoriteRotationInterval,
    settingsStore,
    type Settings,
  } from "$lib/stores/settings.svelte";
  import { toastStore } from "$lib/stores/toasts.svelte";
  import { store } from "$lib/stores/wallpapers.svelte";
  import type { StorageCleanupResult, StorageStats } from "$lib/types/wallpaper";
  import Toggle from "./Toggle.svelte";
  import Icon from "./Icon.svelte";

  let { onClose }: { onClose?: () => void } = $props();

  const s = settingsStore;
  let storageStats = $state<StorageStats | null>(null);
  let storageLoading = $state(false);
  let storageError = $state<string | null>(null);
  let storageAction = $state<
    "wallhaven" | "thumbnails" | "missing" | "refresh" | null
  >(null);
  let playbackSpeedValue = $state(String(DEFAULT_SETTINGS.defaultPlaybackSpeed));
  let fitModeValue = $state(DEFAULT_SETTINGS.defaultFitMode);
  let fpsCapValue = $state(fpsCapSelectValue(DEFAULT_SETTINGS.defaultFpsCap));

  const playbackSpeedOptions = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];
  const fpsCapOptions = [24, 30, 60, 120, 144, 240];
  const fitModeOptions = [
    { value: "fit", label: "Fit (letterbox)" },
    { value: "fill", label: "Fill (crop edges)" },
    { value: "stretch", label: "Stretch (ignore aspect)" },
  ];

  let customPlaybackSpeed = $derived.by(() => {
    const speed = numberFromSelectValue(playbackSpeedValue);
    return speed !== null && !isListedNumber(playbackSpeedOptions, speed) ? speed : null;
  });
  let customFpsCap = $derived.by(() => {
    const fps = fpsCapFromSelectValue(fpsCapValue);
    return fps !== null && !isListedNumber(fpsCapOptions, fps) ? fps : null;
  });

  $effect(() => {
    playbackSpeedValue = playbackSpeedSelectValue(s.settings.defaultPlaybackSpeed);
    fitModeValue = fitModeSelectValue(s.settings.defaultFitMode);
    fpsCapValue = fpsCapSelectValue(s.settings.defaultFpsCap);
  });

  type BooleanSettingKey = {
    [K in keyof Settings]: Settings[K] extends boolean ? K : never;
  }[keyof Settings];

  interface Row {
    key: BooleanSettingKey;
    title: string;
    description: string;
  }

  const groups: { title: string; rows: Row[] }[] = [
    {
      title: "Startup",
      rows: [
        {
          key: "launchAtStartup",
          title: "Launch at startup",
          description: "Open Wallscape automatically when you sign in to Windows.",
        },
        {
          key: "startMinimized",
          title: "Start in tray",
          description: "Launch hidden in the system tray instead of opening the window.",
        },
        {
          key: "restoreLastWallpaper",
          title: "Restore last wallpaper",
          description: "Re-apply your most recent video wallpaper when Wallscape starts.",
        },
      ],
    },
    {
      title: "Window",
      rows: [
        {
          key: "closeToTray",
          title: "Close to tray",
          description: "Closing the window keeps Wallscape running in the tray.",
        },
        {
          key: "minimizeToTray",
          title: "Minimize to tray",
          description: "Minimizing hides the window to the tray instead of the taskbar.",
        },
      ],
    },
    {
      title: "Auto-pause",
      rows: [
        {
          key: "autoPauseEnabled",
          title: "Enable auto-pause",
          description: "Pause live wallpapers automatically when a rule matches.",
        },
        {
          key: "autoPauseOnBattery",
          title: "On battery",
          description: "Pause live wallpapers while the PC is running on battery power.",
        },
        {
          key: "autoPauseFullscreenApps",
          title: "Fullscreen apps",
          description: "Pause while another fullscreen app or game is in front.",
        },
        {
          key: "autoPauseOccluded",
          title: "Covered desktop",
          description: "Pause when every screen is fully covered by other windows.",
        },
        {
          key: "autoPauseRemoteSession",
          title: "Remote desktop",
          description: "Pause while the session is running through Remote Desktop.",
        },
        {
          key: "autoPauseDisplaySleep",
          title: "Display unavailable",
          description: "Pause when the interactive desktop is unavailable, such as lock or display sleep.",
        },
      ],
    },
  ];

  function numberFromSelectValue(value: string) {
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  }

  function playbackSpeedSelectValue(value: unknown) {
    return String(numberFromSetting(value) ?? DEFAULT_SETTINGS.defaultPlaybackSpeed);
  }

  function fitModeSelectValue(value: unknown) {
    return typeof value === "string" &&
      fitModeOptions.some((option) => option.value === value)
      ? value
      : DEFAULT_SETTINGS.defaultFitMode;
  }

  function fpsCapSelectValue(value: unknown) {
    const fps = numberFromSetting(value);
    return fps && fps > 0 ? String(Math.round(fps)) : "0";
  }

  function fpsCapFromSelectValue(value: string) {
    const fps = Number.parseInt(value, 10);
    return Number.isFinite(fps) && fps > 0 ? fps : null;
  }

  function numberFromSetting(value: unknown) {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string") {
      const number = Number(value);
      return Number.isFinite(number) ? number : null;
    }
    return null;
  }

  function isListedNumber(options: number[], value: number) {
    return options.some((option) => option === value);
  }

  function playbackSpeedLabel(speed: number) {
    const rounded = Math.round(speed * 100) / 100;
    return `${Number.isInteger(rounded) ? rounded.toFixed(0) : rounded}×`;
  }

  function fpsCapLabel(fps: number) {
    return `${fps} FPS`;
  }

  async function handleSpeedChange(value: number) {
    const previous = s.settings.defaultPlaybackSpeed;
    await s.update({ defaultPlaybackSpeed: value });

    if (s.error) {
      toastStore.error(s.error);
    } else if (previous !== value) {
      toastStore.success("Settings saved");
    }
  }

  async function handleFitModeChange(value: string) {
    const previous = s.settings.defaultFitMode;
    await s.update({ defaultFitMode: value });

    if (s.error) {
      toastStore.error(s.error);
    } else if (previous !== value) {
      toastStore.success("Settings saved");
    }
  }

  async function handleFpsCapChange(value: number | null) {
    const previous = s.settings.defaultFpsCap;
    await s.update({ defaultFpsCap: value });

    if (s.error) {
      toastStore.error(s.error);
    } else if (previous !== value) {
      toastStore.success("Settings saved");
    }
  }

  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose?.();
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose?.();
  }

  onMount(() => {
    void loadStorageStats();
  });

  async function loadStorageStats() {
    storageLoading = true;
    storageAction = storageAction ?? "refresh";
    storageError = null;

    try {
      storageStats = await getStorageStats();
    } catch (e) {
      storageError = `Storage stats unavailable: ${e}`;
    } finally {
      storageLoading = false;
      if (storageAction === "refresh") {
        storageAction = null;
      }
    }
  }

  async function handleToggle(key: BooleanSettingKey, value: boolean) {
    const previous = s.settings[key];
    await s.update({ [key]: value } as Partial<Settings>);

    if (s.error) {
      toastStore.error(s.error);
    } else if (previous !== value) {
      toastStore.success("Settings saved");
    }
  }

  async function handleIntervalChange(value: number) {
    const next = normalizeFavoriteRotationInterval(value);
    const previous = s.settings.favoriteRotationIntervalMinutes;
    await s.update({ favoriteRotationIntervalMinutes: next });

    if (s.error) {
      toastStore.error(s.error);
    } else if (previous !== next) {
      toastStore.success("Settings saved");
    }
  }

  async function runStorageAction(
    action: "wallhaven" | "thumbnails" | "missing",
    task: () => Promise<StorageCleanupResult>,
  ) {
    storageAction = action;
    storageError = null;

    try {
      const result = await task();
      if (action === "missing") {
        await store.loadWallpapers();
      }
      await loadStorageStats();
      reportStorageResult(action, result);
    } catch (e) {
      storageError = `Storage cleanup failed: ${e}`;
      toastStore.error(storageError);
    } finally {
      storageAction = null;
    }
  }

  function reportStorageResult(
    action: "wallhaven" | "thumbnails" | "missing",
    result: StorageCleanupResult,
  ) {
    if (result.warnings.length) {
      toastStore.warning(result.warnings[0]);
      return;
    }

    if (action === "wallhaven") {
      toastStore.success(
        `Unused Wallhaven cache cleared: ${result.removed_files} ${fileWord(result.removed_files)} freed ${formatBytes(result.removed_bytes)}. Saved wallpapers kept.`,
      );
      return;
    }

    if (action === "thumbnails") {
      toastStore.success(
        `Unused thumbnails removed: ${result.removed_files} ${fileWord(result.removed_files)} freed ${formatBytes(result.removed_bytes)}.`,
      );
      return;
    }

    toastStore.success(
      `Missing library entries cleaned: ${result.removed_entries} ${entryWord(result.removed_entries)} removed.`,
    );
  }

  function fileWord(count: number) {
    return count === 1 ? "file" : "files";
  }

  function entryWord(count: number) {
    return count === 1 ? "entry" : "entries";
  }

  function formatBytes(bytes: number | null | undefined) {
    const value = Math.max(0, bytes ?? 0);
    if (value < 1024) return `${value} B`;

    const units = ["KB", "MB", "GB", "TB"];
    let size = value / 1024;
    let unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex += 1;
    }

    return `${size >= 10 ? size.toFixed(1) : size.toFixed(2)} ${units[unitIndex]}`;
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="overlay" role="presentation" onclick={handleBackdrop}>
  <div class="panel" role="dialog" aria-modal="true" aria-label="Settings">
    <header class="panel-head">
      <h2>Settings</h2>
      <button class="close-btn" aria-label="Close settings" onclick={() => onClose?.()}>
        <Icon name="close" size={18} />
      </button>
    </header>

    <div class="panel-body">
      {#each groups as group, groupIndex (group.title)}
        <section class="group">
          <h3 class="group-title">{group.title}</h3>
          {#each group.rows as row, rowIndex (row.key)}
            <div class="row" style={`animation-delay:${groupIndex * 120 + rowIndex * 55}ms`}>
              <div class="row-text">
                <span class="row-title">{row.title}</span>
                <span class="row-desc">{row.description}</span>
              </div>
              <Toggle
                checked={s.settings[row.key]}
                disabled={s.saving}
                label={row.title}
                onChange={(v) => handleToggle(row.key, v)}
              />
            </div>
          {/each}
        </section>
      {/each}

      <section class="group">
        <h3 class="group-title">Rotation</h3>
        <div class="row">
          <div class="row-text">
            <span class="row-title">Rotate favorites</span>
            <span class="row-desc">Change wallpaper automatically from your favorites.</span>
          </div>
          <Toggle
            checked={s.settings.favoriteRotationEnabled}
            disabled={s.saving}
            label="Rotate favorites"
            onChange={(v) => handleToggle("favoriteRotationEnabled", v)}
          />
        </div>

        <div class="row">
          <div class="row-text">
            <span class="row-title">Rotation interval</span>
            <span class="row-desc">Minutes between favorite wallpaper changes.</span>
          </div>
          <input
            class="number-input"
            type="number"
            min="1"
            max="1440"
            step="1"
            value={s.settings.favoriteRotationIntervalMinutes}
            disabled={s.saving}
            aria-label="Favorite rotation interval in minutes"
            onchange={(e) =>
              handleIntervalChange((e.currentTarget as HTMLInputElement).valueAsNumber)}
          />
        </div>

        <div class="row">
          <div class="row-text">
            <span class="row-title">Rotate on startup</span>
            <span class="row-desc">Apply a random favorite when Wallscape opens.</span>
          </div>
          <Toggle
            checked={s.settings.favoriteRotationOnStartup}
            disabled={s.saving}
            label="Rotate on startup"
            onChange={(v) => handleToggle("favoriteRotationOnStartup", v)}
          />
        </div>
      </section>

      <section class="group">
        <h3 class="group-title">Playback</h3>
        <div class="row">
          <div class="row-text">
            <span class="row-title">Default playback speed</span>
            <span class="row-desc">Starting speed for video wallpapers (can adjust per video).</span>
          </div>
          <select
            class="select-input"
            bind:value={playbackSpeedValue}
            disabled={s.saving}
            aria-label="Default playback speed"
            onchange={(e) => {
              const value = numberFromSelectValue((e.currentTarget as HTMLSelectElement).value);
              if (value !== null) handleSpeedChange(value);
            }}
          >
            {#if customPlaybackSpeed !== null}
              <option value={playbackSpeedValue}>{playbackSpeedLabel(customPlaybackSpeed)}</option>
            {/if}
            {#each playbackSpeedOptions as speed (speed)}
              <option value={String(speed)}>{playbackSpeedLabel(speed)}</option>
            {/each}
          </select>
        </div>

        <div class="row">
          <div class="row-text">
            <span class="row-title">Default fit mode</span>
            <span class="row-desc">How video wallpapers scale to fit your screen.</span>
          </div>
          <select
            class="select-input"
            bind:value={fitModeValue}
            disabled={s.saving}
            aria-label="Default fit mode"
            onchange={(e) =>
              handleFitModeChange((e.currentTarget as HTMLSelectElement).value)}
          >
            {#each fitModeOptions as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>

        <div class="row">
          <div class="row-text">
            <span class="row-title">FPS cap</span>
            <span class="row-desc">Limit frame rate to reduce GPU usage.</span>
          </div>
          <select
            class="select-input"
            bind:value={fpsCapValue}
            disabled={s.saving}
            aria-label="FPS cap"
            onchange={(e) => {
              handleFpsCapChange(
                fpsCapFromSelectValue((e.currentTarget as HTMLSelectElement).value),
              );
            }}
          >
            <option value="0">Unlimited</option>
            {#if customFpsCap !== null}
              <option value={fpsCapValue}>{fpsCapLabel(customFpsCap)}</option>
            {/if}
            {#each fpsCapOptions as fps (fps)}
              <option value={String(fps)}>{fpsCapLabel(fps)}</option>
            {/each}
          </select>
        </div>
      </section>

      <section class="group storage-group">
        <div class="group-heading">
          <h3 class="group-title">Storage</h3>
          <button
            class="icon-action"
            aria-label="Refresh storage stats"
            disabled={storageLoading || storageAction !== null}
            onclick={() => loadStorageStats()}
          >
            <Icon name="refresh" size={15} />
          </button>
        </div>

        <div class="storage-metrics" aria-live="polite">
          <div class="metric">
            <span class="metric-label">Cache total</span>
            <span class="metric-value">
              {storageStats ? formatBytes(storageStats.total_cache_bytes) : "-"}
            </span>
          </div>
          <div class="metric">
            <span class="metric-label">Wallhaven</span>
            <span class="metric-value">
              {storageStats ? formatBytes(storageStats.wallhaven_cache_bytes) : "-"}
            </span>
          </div>
          <div class="metric">
            <span class="metric-label">Thumbnails</span>
            <span class="metric-value">
              {storageStats ? formatBytes(storageStats.thumbnail_cache_bytes) : "-"}
            </span>
          </div>
        </div>

        <div class="storage-action">
          <div class="row-text">
            <span class="row-title">Clear Wallhaven cache</span>
            <span class="row-desc">
              {storageStats
                ? `${storageStats.unused_wallhaven_cache_files} unused ${fileWord(storageStats.unused_wallhaven_cache_files)}, ${storageStats.wallhaven_library_entries} saved ${entryWord(storageStats.wallhaven_library_entries)} kept`
                : "Unused Wallhaven downloads only"}
            </span>
          </div>
          <button
            class="action-btn"
            disabled={!storageStats || storageAction !== null || storageStats.unused_wallhaven_cache_files === 0}
            onclick={() => runStorageAction("wallhaven", clearWallhavenCache)}
          >
            {#if storageAction === "wallhaven"}
              <Icon name="refresh" size={15} />
            {:else}
              <Icon name="trash" size={15} />
            {/if}
            Clear
          </button>
        </div>

        <div class="storage-action">
          <div class="row-text">
            <span class="row-title">Remove unused thumbnails</span>
            <span class="row-desc">
              {storageStats
                ? `${storageStats.unused_thumbnail_files} stale ${fileWord(storageStats.unused_thumbnail_files)}, ${formatBytes(storageStats.unused_thumbnail_bytes)}`
                : "Generated previews no longer used"}
            </span>
          </div>
          <button
            class="action-btn"
            disabled={!storageStats || storageAction !== null || storageStats.unused_thumbnail_files === 0}
            onclick={() => runStorageAction("thumbnails", cleanupUnusedThumbnails)}
          >
            {#if storageAction === "thumbnails"}
              <Icon name="refresh" size={15} />
            {:else}
              <Icon name="trash" size={15} />
            {/if}
            Remove
          </button>
        </div>

        <div class="storage-action">
          <div class="row-text">
            <span class="row-title">Clean missing library entries</span>
            <span class="row-desc">
              {storageStats
                ? `${storageStats.missing_library_entries} missing ${entryWord(storageStats.missing_library_entries)}`
                : "Library rows pointing to missing files"}
            </span>
          </div>
          <button
            class="action-btn"
            disabled={!storageStats || storageAction !== null || storageStats.missing_library_entries === 0}
            onclick={() => runStorageAction("missing", cleanupMissingLibraryEntries)}
          >
            {#if storageAction === "missing"}
              <Icon name="refresh" size={15} />
            {:else}
              <Icon name="check" size={15} />
            {/if}
            Clean
          </button>
        </div>

        {#if storageError}
          <p class="error storage-error">{storageError}</p>
        {/if}
      </section>

      {#if s.error}
        <p class="error">{s.error}</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    background: rgba(5, 7, 11, 0.55);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    animation: overlay-in var(--motion-med) var(--ease-standard) both;
  }

  .panel {
    width: 100%;
    max-width: 520px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-lg);
    background: var(--glass-panel-strong);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid var(--glass-border);
    box-shadow: var(--shadow-card), inset 0 1px 0 var(--glass-highlight);
    overflow: hidden;
    animation: panel-in 440ms var(--ease-emphasized) both;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-5) var(--space-6);
    border-bottom: 1px solid var(--glass-border);
  }

  .panel-head h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 50%;
    background: var(--glass-control);
    color: var(--text-secondary);
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.14);
    color: var(--text-primary);
    transform: rotate(6deg) scale(1.04);
  }

  .panel-body {
    padding: var(--space-5) var(--space-6) var(--space-6);
    overflow-y: auto;
  }

  .group + .group {
    margin-top: var(--space-6);
  }

  .group-title {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    margin-bottom: var(--space-2);
  }

  .group-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }

  .group-heading .group-title {
    margin-bottom: 0;
  }

  .icon-action {
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--glass-border);
    border-radius: 50%;
    background: var(--glass-control);
    color: var(--text-secondary);
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .icon-action:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    color: var(--text-primary);
    transform: rotate(12deg);
  }

  .icon-action:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-5);
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--glass-border);
    animation: row-in 420ms var(--ease-emphasized) both;
  }

  .row:last-child {
    border-bottom: none;
  }

  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row-title {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .row-desc {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .number-input {
    width: 92px;
    height: 38px;
    padding: 0 12px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: var(--glass-control);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
    text-align: center;
  }

  .number-input:focus {
    outline: none;
    border-color: var(--accent-blue);
    background: rgba(35, 43, 50, 0.72);
  }

  .number-input:disabled {
    opacity: 0.62;
    cursor: not-allowed;
  }

  .select-input {
    min-width: 140px;
    height: 38px;
    padding: 0 12px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: var(--glass-control);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }

  .select-input:focus {
    outline: none;
    border-color: var(--accent-blue);
    background: rgba(35, 43, 50, 0.72);
  }

  .select-input:disabled {
    opacity: 0.62;
    cursor: not-allowed;
  }

  .storage-metrics {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-3);
    padding: var(--space-3) 0 var(--space-4);
    border-bottom: 1px solid var(--glass-border);
  }

  .metric {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .metric-label {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .metric-value {
    min-height: 20px;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    overflow-wrap: anywhere;
  }

  .storage-action {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--glass-border);
    animation: row-in 420ms var(--ease-emphasized) both;
  }

  .storage-action:last-of-type {
    border-bottom: none;
  }

  .action-btn {
    min-width: 92px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 0 13px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    background: var(--glass-control);
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    transition:
      background var(--motion-fast) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .action-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    border-color: rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
  }

  .action-btn:disabled {
    opacity: 0.52;
    cursor: not-allowed;
  }

  .storage-error {
    margin-top: var(--space-3);
  }

  .error {
    margin-top: var(--space-5);
    font-size: 13px;
    color: #ffb4ab;
    animation: soft-enter var(--motion-med) var(--ease-emphasized) both;
  }

  @keyframes overlay-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes panel-in {
    from {
      opacity: 0;
      transform: translateY(18px) scale(0.96);
      filter: blur(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
      filter: blur(0);
    }
  }

  @keyframes row-in {
    from {
      opacity: 0;
      transform: translateX(-10px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  @media (max-width: 640px) {
    .overlay {
      align-items: flex-end;
      padding: var(--space-3);
    }
    .panel {
      max-height: min(86dvh, 680px);
      max-width: none;
      border-radius: 22px;
      animation-name: panel-sheet-in;
    }
    .panel-head,
    .panel-body {
      padding-left: var(--space-5);
      padding-right: var(--space-5);
    }
    .row {
      align-items: flex-start;
      gap: var(--space-4);
    }
    .storage-metrics {
      grid-template-columns: 1fr;
      gap: var(--space-2);
    }
    .storage-action {
      align-items: flex-start;
      gap: var(--space-3);
    }
    .action-btn {
      min-width: 84px;
    }
    .row-desc {
      max-width: 30ch;
    }
  }

  @keyframes panel-sheet-in {
    from {
      opacity: 0;
      transform: translateY(32px);
      filter: blur(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
      filter: blur(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .panel,
    .row,
    .error {
      animation: none !important;
    }
  }
</style>
