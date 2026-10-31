<script lang="ts">
  import { onMount } from "svelte";
  import {
    openExternalUrl,
    setWallpaperFitMode,
    setWallpaperSpeed,
  } from "$lib/api/wallpaperApi";
  import { PlaybackStore } from "$lib/stores/playback.svelte";
  import { toastStore } from "$lib/stores/toasts.svelte";
  import type { RuntimeStatus } from "$lib/types/generated";
  import type { Monitor, Wallpaper } from "$lib/types/wallpaper";
  import { formatDimensions, wallpaperMediaLabel } from "$lib/utils/media";
  import Icon from "./Icon.svelte";
  import WallpaperPreviewMedia from "./WallpaperPreviewMedia.svelte";

  let {
    wallpaper,
    onClose,
    onApply,
    onCancel,
    canCancel = false,
    onRemove,
    monitors = [],
    applyTargetMonitorId = null,
    onApplyTargetChange,
    favoriteBusy = false,
    applying = false,
    onFavoriteChange,
    onReveal,
    onRegenerateThumbnail,
    thumbnailBusy = false,
    applyLabel = null,
    onPrev,
    onNext,
  }: {
    wallpaper: Wallpaper;
    onClose?: () => void;
    onApply?: (w: Wallpaper, monitorId: string | null) => void;
    onCancel?: (w: Wallpaper) => void;
    canCancel?: boolean;
    onRemove?: (w: Wallpaper) => void;
    monitors?: Monitor[];
    applyTargetMonitorId?: string | null;
    onApplyTargetChange?: (monitorId: string | null) => void;
    favoriteBusy?: boolean;
    applying?: boolean;
    onFavoriteChange?: (w: Wallpaper, isFavorite: boolean) => void;
    onReveal?: (w: Wallpaper) => void;
    onRegenerateThumbnail?: (w: Wallpaper) => void;
    thumbnailBusy?: boolean;
    /** Busy text for the apply button, e.g. download progress. */
    applyLabel?: string | null;
    onPrev?: () => void;
    onNext?: () => void;
  } = $props();

  let dialogEl = $state<HTMLDivElement | null>(null);
  let favorited = $derived(Boolean(wallpaper.is_favorite));
  let canTargetMonitor = $derived(monitors.length > 1);
  let activeApplyTargetMonitorId = $derived(
    canTargetMonitor ? applyTargetMonitorId : null,
  );
  let dimensions = $derived(formatDimensions(wallpaper.width, wallpaper.height));
  let mediaLabel = $derived(wallpaperMediaLabel(wallpaper));
  let wallhavenSourceUrl = $derived(
    wallpaper.source === "wallhaven" && wallpaper.source_id
      ? `https://wallhaven.cc/w/${wallpaper.source_id}`
      : null,
  );

  // Playback controls act on the live desktop players showing this file, so
  // they only appear while it is playing on at least one display. Speed and
  // fit changes apply to those displays only and are not saved as defaults.
  const playback = new PlaybackStore();
  let isLocalVideo = $derived(
    wallpaper.id > 0 && wallpaper.media_type === "video" && Boolean(wallpaper.file_path),
  );
  let livePlayers = $derived(
    isLocalVideo
      ? playback.status.players.filter((player) => player.videoPath === wallpaper.file_path)
      : [],
  );
  let showPlaybackControls = $derived(livePlayers.length > 0);
  // Pause is shared by every live wallpaper, so say so when another file plays.
  let pauseCoversOthers = $derived(
    playback.status.players.some((player) => player.videoPath !== wallpaper.file_path),
  );
  let isPaused = $derived(playback.status.paused);
  let playbackSpeed = $derived(sharedValue(livePlayers.map((player) => player.playbackSpeed)));
  let fitMode = $derived(sharedValue(livePlayers.map((player) => player.fitMode)));
  let speedMenuOpen = $state(false);
  const speedOptions = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];

  // The value every matching display has, or null when they differ.
  function sharedValue<T>(values: T[]): T | null {
    return values.length > 0 && values.every((value) => value === values[0]) ? values[0] : null;
  }

  $effect(() => {
    if (!isLocalVideo) return;
    // Re-read right away when this preview changes or an apply finishes.
    wallpaper.file_path;
    applying;
    void playback.refresh();
    const interval = window.setInterval(() => void playback.refresh(), 2000);
    return () => window.clearInterval(interval);
  });

  $effect(() => {
    if (!showPlaybackControls) speedMenuOpen = false;
  });

  onMount(() => {
    // The page behind is inert while this is open; the page restores focus.
    dialogEl?.focus({ preventScroll: true });
  });

  // Keys the focused control already uses: arrows change a <select> (the
  // display picker), Space presses a button.
  const ARROW_KEY_CONTROLS = "select, input, textarea, [role='menu']";
  const SPACE_KEY_CONTROLS = "button, select, input, textarea, a[href], [role='menu']";

  function focusedControlHandles(e: KeyboardEvent, selector: string) {
    return e.target instanceof Element && e.target.closest(selector) !== null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return;

    if (e.key === "Escape") {
      e.preventDefault();
      if (speedMenuOpen) {
        speedMenuOpen = false;
      } else {
        onClose?.();
      }
      return;
    }

    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      if (focusedControlHandles(e, ARROW_KEY_CONTROLS)) return;
      e.preventDefault();
      if (e.key === "ArrowLeft") onPrev?.();
      else onNext?.();
      return;
    }

    if (e.key === " " && showPlaybackControls) {
      if (focusedControlHandles(e, SPACE_KEY_CONTROLS)) return;
      e.preventDefault();
      void togglePause();
    }
  }

  const fitModeIcon = $derived(
    fitMode === "fit" ? "maximize-2" : fitMode === "fill" ? "maximize" : "move",
  );

  const fitModeLabel = $derived(
    fitMode === null
      ? "Mixed across displays"
      : fitMode === "fit"
      ? "Fit (letterbox)"
      : fitMode === "fill"
        ? "Fill (crop edges)"
        : "Stretch (ignore aspect)",
  );

  $effect(() => {
    if (!canTargetMonitor && applyTargetMonitorId) {
      onApplyTargetChange?.(null);
    }
  });

  function toggleFavorite() {
    if (favoriteBusy) return;
    onFavoriteChange?.(wallpaper, !favorited);
  }

  async function openWallhavenSource() {
    if (!wallhavenSourceUrl) return;

    try {
      await openExternalUrl(wallhavenSourceUrl);
    } catch (error) {
      console.warn("Failed to open via Tauri command, falling back to window.open:", error);
      window.open(wallhavenSourceUrl, "_blank", "noopener,noreferrer");
    }
  }

  async function togglePause() {
    if (applying || playback.busy) return;
    const result = await playback.toggle();
    if (!result.ok) toastStore.error(result.error);
  }

  // Applies a live change to each display showing this file, or to all of them
  // in one call when this file is the only one playing.
  async function changeLivePlayers(change: (monitorId: string | null) => Promise<RuntimeStatus>) {
    const targets = pauseCoversOthers ? livePlayers.map((player) => player.monitorId) : [null];
    try {
      await playback.update(async () => {
        let status: RuntimeStatus | null = null;
        for (const monitorId of targets) status = await change(monitorId);
        return status ?? playback.status;
      });
    } catch (error) {
      toastStore.error(String(error));
    }
  }

  function setPlaybackSpeed(speed: number) {
    return changeLivePlayers((monitorId) => setWallpaperSpeed(speed, monitorId));
  }

  async function choosePlaybackSpeed(speed: number) {
    speedMenuOpen = false;
    await setPlaybackSpeed(speed);
  }

  function speedLabel(speed: number | null) {
    if (speed === null) return "Mixed";
    return `${Number.isInteger(speed) ? speed.toFixed(0) : speed}×`;
  }

  async function cycleFitMode() {
    const modes = ["fit", "fill", "stretch"];
    const currentIndex = fitMode === null ? -1 : modes.indexOf(fitMode);
    const nextMode = modes[(currentIndex + 1) % modes.length];

    await changeLivePlayers((monitorId) => setWallpaperFitMode(nextMode, monitorId));
  }
</script>

<svelte:window
  onkeydown={handleKeydown}
  onclick={() => {
    speedMenuOpen = false;
  }}
/>

<div
  class="detail"
  role="dialog"
  aria-modal="true"
  aria-label={wallpaper.title}
  tabindex="-1"
  bind:this={dialogEl}
>
  <WallpaperPreviewMedia
    {wallpaper}
    mode="detail"
    imageClass="detail-bg"
    posterClass="detail-bg detail-poster"
    videoClass="detail-bg detail-video"
    fallbackClass="detail-bg detail-fallback"
  />
  <div class="detail-scrim"></div>

  <button class="close" aria-label="Close preview" onclick={() => onClose?.()}>
    <Icon name="close" size={20} />
  </button>

  <!-- Floating frosted control bar -->
  <div class="control-bar glass-surface">
    <div class="meta">
      <div class="title">{wallpaper.title}</div>
      <div class="sub">
        {#if dimensions}
          <span>{dimensions}</span>
          <span>·</span>
        {/if}
        <span>{mediaLabel}</span>
        {#if wallpaper.tags.length}
          <span>·</span>
          <span class="cat">{wallpaper.tags[0]}</span>
        {/if}
      </div>
      {#if wallpaper.source === "wallhaven"}
        <div class="source-note">Source: Wallhaven. Image rights remain with the original owner.</div>
      {/if}
    </div>

    <div class="nav-group">
      <button aria-label="Previous" onclick={() => onPrev?.()}>
        <Icon name="chevron-left" size={18} />
      </button>
      <button aria-label="Next" onclick={() => onNext?.()}>
        <Icon name="chevron-right" size={18} />
      </button>
    </div>

    {#if showPlaybackControls}
      <div class="playback-group" role="group" aria-label="Desktop playback">
        <button
          class="pause-btn"
          class:playing={!isPaused}
          aria-label={pauseCoversOthers
            ? isPaused ? "Resume all live wallpapers" : "Pause all live wallpapers"
            : isPaused ? "Resume playback" : "Pause playback"}
          title={pauseCoversOthers ? "Pause applies to every display" : undefined}
          aria-pressed={!isPaused}
          disabled={applying || playback.busy}
          onclick={togglePause}
        >
          <Icon name={isPaused ? "play" : "pause"} size={18} />
        </button>

        <div class="speed-control">
          <button
            class="speed-button"
            type="button"
            aria-label="Playback speed"
            aria-haspopup="menu"
            aria-expanded={speedMenuOpen}
            disabled={applying || playback.busy}
            onclick={(event) => {
              event.stopPropagation();
              speedMenuOpen = !speedMenuOpen;
            }}
          >
            <Icon name="gauge" size={15} />
            <span>{speedLabel(playbackSpeed)}</span>
            <span class="speed-caret">
              <Icon name="chevron-right" size={14} />
            </span>
          </button>

          {#if speedMenuOpen}
            <div class="speed-menu" role="menu" aria-label="Playback speed">
              {#each speedOptions as speed}
                <button
                  type="button"
                  role="menuitemradio"
                  aria-checked={playbackSpeed === speed}
                  class:active={playbackSpeed === speed}
                  disabled={applying || playback.busy}
                  onclick={(event) => {
                    event.stopPropagation();
                    choosePlaybackSpeed(speed);
                  }}
                >
                  {speedLabel(speed)}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <button
          class="fit-mode-btn"
          aria-label="Fit mode: {fitModeLabel}"
          title={fitModeLabel}
          disabled={applying || playback.busy}
          onclick={cycleFitMode}
        >
          <Icon name={fitModeIcon} size={18} />
        </button>
      </div>
    {/if}

    <div class="icon-group">
      {#if wallpaper.id > 0 && onReveal}
        <button aria-label="Reveal in Explorer" onclick={() => onReveal?.(wallpaper)}>
          <Icon name="folder" size={18} />
        </button>
      {/if}
      {#if wallpaper.id > 0 && wallpaper.media_type === "video" && onRegenerateThumbnail}
        <button
          aria-label="Regenerate thumbnail"
          disabled={thumbnailBusy}
          aria-busy={thumbnailBusy}
          onclick={() => onRegenerateThumbnail?.(wallpaper)}
        >
          <Icon name="refresh" size={18} spin={thumbnailBusy} />
        </button>
      {/if}
      {#if onFavoriteChange}
        <button
          class="fav"
          class:on={favorited}
          aria-label={favorited ? "Remove from favorites" : "Add to favorites"}
          aria-pressed={favorited}
          disabled={favoriteBusy}
          onclick={toggleFavorite}
        >
          <Icon name={favorited ? "heart-filled" : "heart"} size={18} />
        </button>
      {/if}
      {#if wallhavenSourceUrl}
        <button aria-label="Open original Wallhaven page" onclick={openWallhavenSource}>
          <Icon name="external" size={18} />
        </button>
      {/if}
      {#if wallpaper.id > 0}
        <button
          class="remove"
          aria-label="Remove from library"
          onclick={() => onRemove?.(wallpaper)}
        >
          <Icon name="trash" size={18} />
        </button>
      {/if}
    </div>

    <div class="action-group">
      {#if canTargetMonitor}
        <label class="target-select">
          <Icon name="monitor" size={15} />
          <select
            value={activeApplyTargetMonitorId ?? ""}
            aria-label="Wallpaper target display"
            onchange={(e) => {
              const value = (e.currentTarget as HTMLSelectElement).value;
              onApplyTargetChange?.(value || null);
            }}
          >
            <option value="">All displays</option>
            {#each monitors as monitor (monitor.id)}
              <option value={monitor.id}>
                {monitor.is_primary ? "Primary" : monitor.name || "Display"}
                · {monitor.width}×{monitor.height}
              </option>
            {/each}
          </select>
        </label>
      {/if}

      {#if canCancel && !applying}
        <button class="cancel" onclick={() => onCancel?.(wallpaper)}>
          <Icon name="rotate-ccw" size={16} />
          Cancel
        </button>
      {/if}

      <button
        class="apply"
        disabled={applying}
        aria-busy={applying}
        onclick={() => {
          if (!applying) onApply?.(wallpaper, activeApplyTargetMonitorId);
        }}
      >
        <Icon name={applying ? "refresh" : "play"} size={16} spin={applying} />
        {applying ? (applyLabel ?? "Applying") : "Set Wallpaper"}
      </button>
    </div>
  </div>
</div>

<style>
  .detail {
    position: absolute;
    inset: 0;
    z-index: 200;
    overflow: hidden;
    /* `backwards`: the backdrop blur only covers the fade-in. Kept after it,
       it keeps blurring the page under an opaque image for no visible gain. */
    animation: detail-in var(--motion-med) var(--ease-standard) backwards;
  }

  .detail:focus-visible {
    outline: none;
  }

  .detail :global(.detail-bg) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: detail-media-in 900ms var(--ease-emphasized) backwards;
  }

  .detail :global(.detail-poster) {
    z-index: 0;
  }

  .detail :global(.detail-video) {
    z-index: 1;
    opacity: 0;
    transition: opacity var(--motion-med) var(--ease-standard);
    background: #0b1014;
  }

  .detail :global(.detail-video.ready) {
    opacity: 1;
  }

  .detail :global(.detail-fallback) {
    background: linear-gradient(135deg, #1a2028, #2a3540 60%, #343d48);
  }

  .detail-scrim {
    position: absolute;
    inset: 0;
    z-index: 1;
    background: linear-gradient(
      to bottom,
      rgba(0, 0, 0, 0.25) 0%,
      transparent 25%,
      transparent 55%,
      rgba(9, 12, 17, 0.85) 100%
    );
  }

  .close {
    position: absolute;
    top: var(--space-5);
    left: var(--space-5);
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    color: #fff;
    background: var(--glass-control);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid var(--glass-border);
    box-shadow: inset 0 1px 0 var(--glass-highlight);
    z-index: 3;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
    animation: soft-enter 420ms var(--ease-emphasized) 120ms backwards;
  }

  .close:hover {
    background: rgba(40, 48, 60, 0.8);
    transform: translateX(-1px) scale(1.04);
  }

  .control-bar {
    position: absolute;
    left: 50%;
    bottom: var(--space-6);
    transform: translateX(-50%);
    z-index: 3;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    border-radius: var(--radius-lg);
    max-width: calc(100% - 48px);
    /* A lingering filter here would stop the speed menu's backdrop blur. */
    animation: rise 480ms var(--ease-emphasized) backwards;
  }

  .meta {
    padding: 0 var(--space-3) 0 var(--space-3);
    min-width: 0;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    display: flex;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .sub .cat {
    text-transform: capitalize;
  }

  .source-note {
    margin-top: 4px;
    color: var(--text-tertiary);
    font-size: 12px;
    line-height: 1.3;
  }

  .nav-group,
  .icon-group,
  .playback-group {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius-full);
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--glass-border);
  }

  .playback-group {
    gap: 6px;
  }

  .control-bar button {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--text-primary);
    border-radius: 50%;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard);
  }

  .control-bar .nav-group button:hover,
  .control-bar .icon-group button:hover,
  .control-bar .playback-group button:hover {
    background: rgba(255, 255, 255, 0.12);
    transform: translateY(-1px);
  }

  .pause-btn.playing {
    background: rgba(100, 200, 255, 0.1);
    box-shadow: inset 0 0 0 1px rgba(100, 200, 255, 0.2);
  }

  .pause-btn.playing:hover {
    background: rgba(100, 200, 255, 0.15);
  }

  .speed-control {
    position: relative;
    flex: 0 0 auto;
  }

  .speed-button {
    height: 36px;
    padding: 0 10px;
    width: 94px !important;
    justify-content: center !important;
    gap: 5px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: rgba(0, 0, 0, 0.22);
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .speed-button span {
    min-width: 36px;
    text-align: center;
  }

  .speed-caret {
    min-width: 0 !important;
    transform: rotate(90deg);
    opacity: 0.8;
  }

  .speed-button:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .speed-button:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.5);
    outline-offset: 2px;
  }

  .speed-menu {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 8px);
    transform: translateX(-50%);
    display: grid;
    grid-template-columns: repeat(2, minmax(54px, 1fr));
    gap: 4px;
    width: 128px;
    padding: 6px;
    border-radius: 14px;
    border: 1px solid var(--glass-border);
    background: rgba(18, 22, 28, 0.96);
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.34), inset 0 1px 0 var(--glass-highlight);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .speed-menu button {
    width: 100% !important;
    height: 30px !important;
    border-radius: 10px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 700;
  }

  .speed-menu button:hover,
  .speed-menu button.active {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text-primary);
    transform: none;
  }

  .fit-mode-btn {
    position: relative;
  }

  .fit-mode-btn:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.5);
    outline-offset: 2px;
  }

  .control-bar button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .fav.on {
    color: #ff3b30;
    animation: press-pop 260ms var(--ease-spring);
  }

  .remove:hover {
    color: #ff8a80;
  }

  .action-group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .target-select {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 38px;
    padding: 0 10px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: rgba(0, 0, 0, 0.22);
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .target-select select {
    max-width: 180px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
  }

  .target-select select:focus {
    outline: none;
  }

  .apply,
  .cancel {
    display: flex !important;
    align-items: center;
    gap: var(--space-2);
    width: auto !important;
    height: auto !important;
    padding: 10px 20px !important;
    border-radius: var(--radius-full) !important;
    background: rgba(255, 255, 255, 0.95) !important;
    color: #1a1a1a !important;
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }

  .cancel {
    padding: 10px 16px !important;
    border: 1px solid var(--glass-border) !important;
    background: rgba(0, 0, 0, 0.22) !important;
    color: var(--text-primary) !important;
  }

  .cancel:hover {
    background: rgba(255, 255, 255, 0.12) !important;
    transform: translateY(-1px);
  }

  .apply:hover {
    background: #fff !important;
    transform: translateY(-1px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.22);
  }

  .apply:active,
  .cancel:active,
  .control-bar button:active,
  .close:active {
    transform: scale(0.96);
  }

  @keyframes detail-in {
    from {
      opacity: 0;
      backdrop-filter: blur(0);
    }
    to {
      opacity: 1;
      backdrop-filter: blur(10px);
    }
  }

  @keyframes detail-media-in {
    from {
      transform: scale(1.04);
      filter: saturate(0.92) brightness(0.86);
    }
    to {
      transform: scale(1);
      filter: saturate(1) brightness(1);
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 18px) scale(0.96);
      filter: blur(10px);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0) scale(1);
      filter: blur(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .detail,
    .detail :global(.detail-bg),
    .close,
    .control-bar {
      animation: none !important;
    }
  }
</style>
