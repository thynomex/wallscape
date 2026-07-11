<script lang="ts">
  import { openExternalUrl } from "$lib/api/wallpaperApi";
  import type { RemoteDownloadState, RemoteDownloadStates } from "$lib/types/downloads";
  import type { MotionBgsWallpaper, WallhavenWallpaper } from "$lib/types/wallpaper";
  import {
    formatRemoteDownloadButtonLabel,
    isRemoteDownloadActive,
    remoteDownloadKey,
    remoteDownloadPercent,
  } from "$lib/utils/downloadProgress";
  import { resolveMediaSrc } from "$lib/utils/media";
  import {
    motionBgsDownloadOptions,
    motionBgsDownloadSourceId,
    motionBgsWallpaperWithDownload,
  } from "$lib/utils/motionbgsDownload";
  import Icon from "./Icon.svelte";

  type DiscoverWallpaper = WallhavenWallpaper | MotionBgsWallpaper;

  let {
    wallpaper,
    downloadState = null,
    downloadStates = null,
    downloadSource,
    onOpen,
    onDownload,
  }: {
    wallpaper: DiscoverWallpaper;
    downloadState?: RemoteDownloadState | null;
    downloadStates?: RemoteDownloadStates | null;
    downloadSource?: string;
    onOpen?: (wallpaper: DiscoverWallpaper) => void;
    onDownload?: (wallpaper: DiscoverWallpaper) => void;
  } = $props();

  let previewFallbackIndex = $state(0);
  let selectedMotionBgsQuality = $state<string | null>(null);

  const sourceName = $derived(isMotionBgsWallpaper(wallpaper) ? "MotionBGS" : "Wallhaven");
  const motionBgsDownloads = $derived(
    isMotionBgsWallpaper(wallpaper) ? motionBgsDownloadOptions(wallpaper) : [],
  );
  const selectedMotionBgsDownload = $derived(
    motionBgsDownloads.find((download) => download.quality === selectedMotionBgsQuality) ??
      (isMotionBgsWallpaper(wallpaper)
        ? (motionBgsDownloads.find((download) => download.quality === wallpaper.quality) ?? null)
        : null) ??
      motionBgsDownloads[0] ??
      null,
  );
  const selectedWallpaper = $derived(
    isMotionBgsWallpaper(wallpaper)
      ? motionBgsWallpaperWithDownload(wallpaper, selectedMotionBgsDownload)
      : wallpaper,
  );
  const activeDownloadState = $derived(
    isMotionBgsWallpaper(selectedWallpaper) && downloadStates && downloadSource
      ? (downloadStates[
          remoteDownloadKey(downloadSource, motionBgsDownloadSourceId(selectedWallpaper))
        ] ?? null)
      : downloadState,
  );
  const fileSize = $derived(
    isMotionBgsWallpaper(selectedWallpaper)
      ? selectedWallpaper.fileSize
      : selectedWallpaper.file_size,
  );
  const fileSizeMb = $derived(fileSize > 0 ? Math.max(1, Math.round(fileSize / 1024 / 1024)) : null);
  const previewCandidates = $derived(
    (isMotionBgsWallpaper(wallpaper)
      ? [wallpaper.thumbnailUrl]
      : [wallpaper.thumbs.large, wallpaper.thumbs.small, wallpaper.path])
      .map((src) => resolveMediaSrc(src))
      .filter((src): src is string => Boolean(src)),
  );
  const previewSrc = $derived(previewCandidates[previewFallbackIndex] ?? "");
  const sourceUrl = $derived(
    selectedWallpaper.url ||
      (isMotionBgsWallpaper(selectedWallpaper)
        ? ""
        : `https://wallhaven.cc/w/${selectedWallpaper.id}`),
  );
  const cardTitle = $derived(
    isMotionBgsWallpaper(selectedWallpaper)
      ? selectedWallpaper.title
      : `Wallhaven ${selectedWallpaper.id}`,
  );
  const cardSubtitle = $derived(
    isMotionBgsWallpaper(selectedWallpaper)
      ? `${selectedWallpaper.width}x${selectedWallpaper.height} · ${fileSizeMb ? `${fileSizeMb}MB · ` : ""}${selectedWallpaper.quality}`
      : `${selectedWallpaper.resolution} · ${fileSizeMb ?? 0}MB · ${selectedWallpaper.category}`,
  );
  const badgeLabel = $derived(
    isMotionBgsWallpaper(selectedWallpaper)
      ? selectedWallpaper.quality
      : selectedWallpaper.purity,
  );
  const downloadActive = $derived(isRemoteDownloadActive(activeDownloadState));
  const downloadFailed = $derived(activeDownloadState?.status === "failed");
  const downloadComplete = $derived(activeDownloadState?.status === "complete");
  const downloadPercent = $derived(remoteDownloadPercent(activeDownloadState));
  const downloadButtonLabel = $derived(
    formatRemoteDownloadButtonLabel(activeDownloadState, "Add to Library"),
  );
  const downloadIcon = $derived(
    downloadFailed ? "refresh" : downloadComplete ? "check" : "download",
  );

  $effect(() => {
    wallpaper.id;
    selectedMotionBgsQuality = isMotionBgsWallpaper(wallpaper)
      ? (motionBgsDownloads.find((download) => download.quality === wallpaper.quality)?.quality ??
        motionBgsDownloads[0]?.quality ??
        wallpaper.quality)
      : null;
    previewFallbackIndex = 0;
  });

  function useNextPreviewCandidate() {
    if (previewFallbackIndex < previewCandidates.length - 1) {
      previewFallbackIndex += 1;
    }
  }

  function preventMediaDrag(event: DragEvent) {
    event.preventDefault();
  }

  async function openImageLink(event: MouseEvent) {
    event.stopPropagation();
    if (!sourceUrl) return;

    try {
      await openExternalUrl(sourceUrl);
    } catch (error) {
      console.warn("Failed to open via Tauri command, falling back to window.open:", error);
      window.open(sourceUrl, "_blank", "noopener,noreferrer");
    }
  }

  function isMotionBgsWallpaper(
    candidate: DiscoverWallpaper,
  ): candidate is MotionBgsWallpaper {
    return "thumbnailUrl" in candidate;
  }
</script>

<article class="discover-card">
  <button class="preview" onclick={() => onOpen?.(wallpaper)} aria-label={`Open ${sourceName} wallpaper ${wallpaper.id}`}>
    <img
      src={previewSrc}
      alt={cardTitle}
      loading="lazy"
      draggable={false}
      ondragstart={preventMediaDrag}
      onerror={useNextPreviewCandidate}
    />
    <span class="purity">{badgeLabel}</span>
  </button>

  <div class="body">
    <div class="title-row">
      <div>
        <h3>{cardTitle}</h3>
        <p>{cardSubtitle}</p>
        <p class="source">Source: {sourceName}</p>
      </div>
      <button class="open-link" type="button" onclick={openImageLink} aria-label={`Open original ${sourceName} page`}>
        <Icon name="external" size={17} />
      </button>
    </div>

    {#if isMotionBgsWallpaper(wallpaper) && motionBgsDownloads.length > 1}
      <div class="quality-selector" role="group" aria-label="Download quality">
        {#each motionBgsDownloads as download (download.quality)}
          <button
            type="button"
            class:active={selectedMotionBgsDownload?.quality === download.quality}
            aria-pressed={selectedMotionBgsDownload?.quality === download.quality}
            disabled={downloadActive}
            onclick={() => (selectedMotionBgsQuality = download.quality)}
          >
            {download.quality}
          </button>
        {/each}
      </div>
    {/if}

    <button
      class="download"
      class:failed={downloadFailed}
      class:complete={downloadComplete}
      disabled={downloadActive || downloadComplete}
      aria-busy={downloadActive}
      onclick={() => onDownload?.(selectedWallpaper)}
    >
      <Icon name={downloadIcon} size={16} />
      <span>{downloadButtonLabel}</span>
    </button>

    {#if downloadActive}
      <div
        class="download-progress"
        class:indeterminate={downloadPercent === null}
        aria-label={downloadPercent === null ? "Download in progress" : `Download ${downloadPercent}% complete`}
      >
        <span style={`width: ${downloadPercent ?? 36}%`}></span>
      </div>
    {:else if downloadFailed}
      <p class="download-state error">{activeDownloadState?.error ?? "Download failed"}</p>
    {/if}
  </div>
</article>

<style>
  .discover-card {
    position: relative;
    overflow: hidden;
    border-radius: 18px;
    background: rgba(12, 18, 22, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.22);
    transition:
      transform var(--motion-med) var(--ease-emphasized),
      border-color var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-med) var(--ease-standard),
      background var(--motion-med) var(--ease-standard);
    animation: card-enter var(--motion-slow) var(--ease-emphasized) both;
    will-change: transform;
  }

  .discover-card:nth-child(2) {
    animation-delay: 70ms;
  }

  .discover-card:nth-child(3) {
    animation-delay: 140ms;
  }

  .discover-card:nth-child(4) {
    animation-delay: 210ms;
  }

  .discover-card:nth-child(5) {
    animation-delay: 280ms;
  }

  .discover-card:nth-child(n + 6) {
    animation-delay: 350ms;
  }

  .discover-card::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    border-radius: inherit;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.16), transparent 32%),
      linear-gradient(315deg, rgba(125, 211, 252, 0.12), transparent 40%);
    opacity: 0;
    transition: opacity var(--motion-med) var(--ease-standard);
  }

  .discover-card:hover {
    transform: translateY(-6px) scale(1.01);
    border-color: rgba(255, 255, 255, 0.18);
    background: rgba(15, 22, 27, 0.78);
    box-shadow: var(--shadow-card-hover);
  }

  .discover-card:hover::after {
    opacity: 1;
  }

  .preview {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    background: var(--bg-secondary);
    border: none;
    padding: 0;
    text-align: left;
  }

  .preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 700ms var(--ease-emphasized), filter var(--motion-med) var(--ease-standard);
  }

  .discover-card:hover .preview img,
  .preview:focus-visible img {
    transform: scale(1.045);
    filter: saturate(1.08);
  }

  .preview:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
  }

  .purity {
    position: absolute;
    left: 12px;
    top: 12px;
    padding: 5px 9px;
    border-radius: var(--radius-full);
    background: rgba(0, 0, 0, 0.5);
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .discover-card:hover .purity {
    background: rgba(0, 0, 0, 0.62);
    transform: translateY(-1px);
  }

  .body {
    padding: 16px;
    display: grid;
    gap: 14px;
    position: relative;
    z-index: 1;
  }

  .title-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
  }

  h3 {
    margin: 0 0 4px;
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 700;
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
    text-transform: capitalize;
  }

  .source {
    margin-top: 4px;
    color: var(--text-tertiary);
    text-transform: none;
  }

  .open-link {
    width: 34px;
    height: 34px;
    border: none;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
    cursor: pointer;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .open-link:hover {
    background: rgba(255, 255, 255, 0.16);
    transform: translateY(-1px) rotate(3deg);
  }

  .open-link:active {
    transform: translateY(0) scale(0.94);
  }

  .quality-selector {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px;
    padding: 4px;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.08);
  }

  .quality-selector button {
    min-width: 0;
    height: 30px;
    border: none;
    border-radius: var(--radius-full);
    color: var(--text-secondary);
    background: transparent;
    font-size: 12px;
    font-weight: 800;
    cursor: pointer;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .quality-selector button:hover:not(:disabled),
  .quality-selector button.active {
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
  }

  .quality-selector button:disabled {
    cursor: wait;
    opacity: 0.65;
  }

  .download {
    width: 100%;
    height: 40px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: none;
    border-radius: var(--radius-full);
    color: #111820;
    background: rgba(255, 255, 255, 0.94);
    font-size: 13px;
    font-weight: 700;
    transition:
      transform var(--motion-fast) var(--ease-standard),
      background var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .download:hover:not(:disabled) {
    transform: translateY(-1px);
    background: #fff;
    box-shadow: 0 10px 22px rgba(0, 0, 0, 0.24);
  }

  .download:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  .download:disabled {
    cursor: wait;
    opacity: 0.72;
    animation: saving-pulse 900ms var(--ease-standard) infinite alternate;
  }

  .download.complete:disabled {
    cursor: default;
    opacity: 0.88;
    color: #0f2a1b;
    background: rgba(187, 247, 208, 0.92);
    animation: none;
  }

  .download.failed {
    color: #3b1212;
    background: rgba(254, 202, 202, 0.95);
  }

  .download-progress {
    height: 4px;
    overflow: hidden;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.1);
  }

  .download-progress span {
    display: block;
    height: 100%;
    min-width: 8%;
    border-radius: inherit;
    background: rgba(255, 255, 255, 0.9);
    transition: width var(--motion-fast) var(--ease-standard);
  }

  .download-progress.indeterminate span {
    width: 36%;
    animation: progress-sweep 1.1s var(--ease-standard) infinite;
  }

  .download-state {
    color: var(--text-tertiary);
    font-size: 12px;
    line-height: 1.35;
    text-transform: none;
  }

  .download-state.error {
    color: #fecaca;
  }

  @keyframes card-enter {
    from {
      opacity: 0;
      transform: translate3d(0, 18px, 0) scale(0.98);
      filter: blur(8px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0) scale(1);
      filter: blur(0);
    }
  }

  @keyframes saving-pulse {
    from {
      filter: saturate(0.9);
    }
    to {
      filter: saturate(1.1);
    }
  }

  @keyframes progress-sweep {
    from {
      transform: translateX(-120%);
    }
    to {
      transform: translateX(300%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .discover-card,
    .download:disabled,
    .download-progress.indeterminate span {
      animation: none !important;
    }
  }
</style>
