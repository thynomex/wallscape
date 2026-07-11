<script lang="ts">
  import {
    openExternalUrl,
    setWallpaperFitMode,
    setWallpaperPaused,
    setWallpaperSpeed,
  } from "$lib/api/wallpaperApi";
  import type { Monitor, Wallpaper } from "$lib/types/wallpaper";
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
    onPrev?: () => void;
    onNext?: () => void;
  } = $props();

  let favorited = $derived(Boolean(wallpaper.is_favorite));
  let canTargetMonitor = $derived(monitors.length > 1);
  let activeApplyTargetMonitorId = $derived(
    canTargetMonitor ? applyTargetMonitorId : null,
  );
  let mediaLabel = $derived(
    wallpaper.media_type === "image"
      ? "image"
      : `${wallpaper.fps}fps`,
  );
  let wallhavenSourceUrl = $derived(
    wallpaper.source === "wallhaven" && wallpaper.source_id
      ? `https://wallhaven.cc/w/${wallpaper.source_id}`
      : null,
  );

  // Playback controls state
  type FitMode = "fit" | "fill" | "stretch";
  let isPaused = $state(false);
  let playbackSpeed = $state(1.0);
  let speedMenuOpen = $state(false);
  let fitMode = $state<FitMode>("fit");
  const speedOptions = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];

  const fitModeIcon = $derived(
    fitMode === "fit" ? "maximize-2" : fitMode === "fill" ? "maximize" : "move",
  );

  const fitModeLabel = $derived(
    fitMode === "fit"
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
    const newPausedState = !isPaused;
    isPaused = newPausedState;

    try {
      await setWallpaperPaused(newPausedState);
    } catch (error) {
      isPaused = !newPausedState;
      console.error("Failed to toggle pause:", error);
    }
  }

  async function setPlaybackSpeed(speed: number) {
    playbackSpeed = speed;
    try {
      await setWallpaperSpeed(speed);
    } catch (error) {
      console.error("Failed to set playback speed:", error);
    }
  }

  async function choosePlaybackSpeed(speed: number) {
    speedMenuOpen = false;
    await setPlaybackSpeed(speed);
  }

  function speedLabel(speed: number) {
    return `${Number.isInteger(speed) ? speed.toFixed(0) : speed}×`;
  }

  async function cycleFitMode() {
    const modes: FitMode[] = ["fit", "fill", "stretch"];
    const currentIndex = modes.indexOf(fitMode);
    const nextMode = modes[(currentIndex + 1) % modes.length];

    fitMode = nextMode;
    try {
      await setWallpaperFitMode(nextMode);
    } catch (error) {
      console.error("Failed to set fit mode:", error);
    }
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (speedMenuOpen && e.key === "Escape") {
      speedMenuOpen = false;
      e.preventDefault();
      return;
    }
    if (e.key === "Escape") onClose?.();
    if (e.key === "ArrowLeft") onPrev?.();
    if (e.key === "ArrowRight") onNext?.();
    if (e.key === " " && wallpaper.media_type === "video") {
      e.preventDefault();
      togglePause();
    }
  }}
  onclick={() => {
    speedMenuOpen = false;
  }}
/>

<div class="detail">
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
    <Icon name="chevron-left" size={20} />
  </button>

  <!-- Floating frosted control bar -->
  <div class="control-bar glass-surface">
    <div class="meta">
      <div class="title">{wallpaper.title}</div>
      <div class="sub">
        <span>{wallpaper.width}×{wallpaper.height}</span>
        <span>·</span>
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

    {#if wallpaper.media_type === "video"}
      <div class="playback-group">
        <button
          class="pause-btn"
          class:playing={!isPaused}
          aria-label={isPaused ? "Resume playback" : "Pause playback"}
          aria-pressed={!isPaused}
          disabled={applying}
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
            disabled={applying}
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
          disabled={applying}
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
          onclick={() => onRegenerateThumbnail?.(wallpaper)}
        >
          <Icon name="refresh" size={18} />
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
        <Icon name={applying ? "refresh" : "play"} size={16} />
        {applying ? "Applying" : "Set Wallpaper"}
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
    animation: detail-in var(--motion-med) var(--ease-standard) both;
  }

  .detail :global(.detail-bg) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: detail-media-in 900ms var(--ease-emphasized) both;
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
    animation: soft-enter 420ms var(--ease-emphasized) 120ms both;
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
    animation: rise 480ms var(--ease-emphasized) both;
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

  @media (max-width: 720px) {
    .control-bar {
      left: var(--space-4);
      right: var(--space-4);
      bottom: var(--space-4);
      max-width: none;
      transform: none;
      flex-wrap: wrap;
      justify-content: space-between;
      border-radius: 18px;
      padding: var(--space-3);
      animation-name: rise-mobile;
    }
    .meta {
      width: 100%;
      padding: 0 var(--space-2) var(--space-2);
    }
    .nav-group,
    .icon-group,
    .playback-group {
      flex: 1;
      justify-content: center;
      min-width: 120px;
    }
    .playback-group {
      justify-content: space-evenly;
    }
    .speed-control {
      width: 94px;
    }
    .speed-menu {
      bottom: calc(100% + 10px);
    }
    .apply {
      flex: 1 1 100%;
      justify-content: center !important;
      height: 42px !important;
    }
    .action-group {
      flex: 1 1 100%;
      width: 100%;
    }
    .cancel,
    .apply,
    .target-select {
      flex: 1 1 0;
      justify-content: center !important;
      min-width: 0;
    }

    .target-select select {
      max-width: none;
      min-width: 0;
      width: 100%;
    }
  }

  @media (max-width: 420px) {
    .icon-group {
      order: 4;
      flex: 1 1 100%;
    }
    .nav-group {
      order: 2;
    }
    .playback-group {
      order: 3;
      flex: 1 1 100%;
    }
    .speed-control {
      flex: 1;
    }
    .speed-button {
      width: 100% !important;
    }
    .apply {
      order: 5;
    }
    .action-group {
      order: 5;
    }
    .target-select {
      flex: 1 1 100%;
    }
    .sub {
      flex-wrap: wrap;
    }
  }

  @keyframes rise-mobile {
    from {
      opacity: 0;
      transform: translateY(18px) scale(0.96);
      filter: blur(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
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
