<script lang="ts">
  import { onMount } from "svelte";
  import type { Wallpaper } from "$lib/types/wallpaper";
  import { reveal } from "$lib/actions/reveal";
  import { appVisibility } from "$lib/stores/appVisibility.svelte";
  import { isGifSrc, resolveMediaSrc, resolveWallpaperPreviewSrc } from "$lib/utils/media";
  import HeroSection from "$lib/components/HeroSection.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import LoadingGrid from "$lib/components/LoadingGrid.svelte";
  import ThumbnailCarousel from "$lib/components/ThumbnailCarousel.svelte";
  import WallpaperCard from "$lib/components/WallpaperCard.svelte";

  let {
    wallpapers,
    featured,
    selected,
    loading,
    onOpen,
    onSelectFeatured,
    onFavoriteChange,
    onBrowseWallhaven,
    onBrowseMotionBgs,
    onImportFiles,
    favoriteBusyId = null,
    onCarouselPausedChange,
  }: {
    wallpapers: Wallpaper[];
    featured: Wallpaper | null;
    selected: Wallpaper | null;
    loading: boolean;
    onOpen: (wallpaper: Wallpaper) => void;
    onSelectFeatured: (wallpaper: Wallpaper) => void;
    onFavoriteChange: (wallpaper: Wallpaper, isFavorite: boolean) => void;
    onBrowseWallhaven: () => void;
    onBrowseMotionBgs: () => void;
    onImportFiles: () => void;
    favoriteBusyId?: number | null;
    onCarouselPausedChange: (paused: boolean) => void;
  } = $props();

  let backdropVideoReady = $state(false);
  let enableBackdropVideo = $state(false);
  let backdropSrc = $derived(resolveWallpaperPreviewSrc(featured));
  let backdropPosterSrc = $derived(resolveMediaSrc(featured?.thumbnail_path));
  let backdropVideoSrc = $derived(
    appVisibility.visible &&
      enableBackdropVideo &&
      featured?.media_type === "video" &&
      featured?.file_path &&
      !isGifSrc(featured.file_path)
      ? resolveMediaSrc(featured.file_path)
      : null,
  );
  let pickWallpapers = $derived(wallpapers.slice(0, 6));
  let hasWallpapers = $derived(wallpapers.length > 0);

  onMount(() => {
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduceMotion) return;

    const timeout = window.setTimeout(() => {
      enableBackdropVideo = true;
    }, 1200);

    return () => window.clearTimeout(timeout);
  });

  $effect(() => {
    backdropVideoSrc;
    backdropVideoReady = false;
  });

  function preventMediaDrag(event: DragEvent) {
    event.preventDefault();
  }
</script>

<div class="home-view">
  <div class="home-backdrop" aria-hidden="true">
    {#key backdropVideoSrc ?? backdropSrc ?? "fallback"}
      {#if backdropVideoSrc}
        <video
          class="home-backdrop-media home-backdrop-video"
          class:ready={backdropVideoReady}
          src={backdropVideoSrc}
          poster={backdropPosterSrc ?? undefined}
          muted
          loop
          playsinline
          preload="metadata"
          autoplay
          draggable={false}
          ondragstart={preventMediaDrag}
          onloadeddata={() => (backdropVideoReady = true)}
        ></video>
        {#if backdropPosterSrc}
          <img
            class="home-backdrop-media home-backdrop-poster"
            src={backdropPosterSrc}
            alt=""
            decoding="async"
            draggable={false}
            ondragstart={preventMediaDrag}
          />
        {/if}
      {:else if backdropSrc}
        <img
          class="home-backdrop-media"
          src={backdropSrc}
          alt=""
          decoding="async"
          fetchpriority="high"
          draggable={false}
          ondragstart={preventMediaDrag}
        />
      {:else}
        <div class="home-backdrop-fallback"></div>
      {/if}
    {/key}
    <div class="home-backdrop-overlay"></div>
  </div>

  {#if loading || hasWallpapers}
    <div class="hero-wrap" use:reveal={{ distance: "28px", delay: 40 }}>
      <HeroSection
        wallpaper={featured}
        {onOpen}
        onFavoriteChange={onFavoriteChange}
        favoriteBusy={featured ? favoriteBusyId === featured.id : false}
      />
    </div>

    {#if hasWallpapers}
      <div
        class="carousel-wrap"
        role="group"
        aria-label="Featured wallpaper carousel"
        use:reveal={{ distance: "18px", delay: 100 }}
        onpointerenter={() => onCarouselPausedChange(true)}
        onpointerleave={() => onCarouselPausedChange(false)}
        onfocusin={() => onCarouselPausedChange(true)}
        onfocusout={() => onCarouselPausedChange(false)}
      >
        <ThumbnailCarousel
          items={pickWallpapers}
          activeId={featured?.id}
          onSelect={onSelectFeatured}
        />
      </div>
    {/if}
  {:else}
    <section class="home-start" use:reveal={{ distance: "28px", delay: 40 }}>
      <div class="home-start-copy">
        <p class="home-start-label">Library Empty</p>
        <h1>Choose your first wallpaper</h1>
        <p>
          Browse Wallhaven for still wallpapers, MotionBGS for live wallpapers, or
          import files already on this PC.
        </p>
      </div>
      <div class="home-start-actions" role="group" aria-label="Add wallpapers">
        <button type="button" class="start-action primary" onclick={onBrowseWallhaven}>
          <span>Wallhaven</span>
          <Icon name="chevron-right" size={16} />
        </button>
        <button type="button" class="start-action" onclick={onBrowseMotionBgs}>
          <span>MotionBGS</span>
          <Icon name="chevron-right" size={16} />
        </button>
        <button type="button" class="start-action" onclick={onImportFiles}>
          <span>Import files</span>
          <Icon name="upload" size={16} />
        </button>
      </div>
    </section>
  {/if}

  {#if loading || hasWallpapers}
    <section class="content-section home-section" use:reveal={{ distance: "26px", delay: 140 }}>
      <div class="section-header">
        <div>
          <h2 class="section-title">Library Picks</h2>
          <p class="section-description">Recently added wallpapers from your library.</p>
        </div>
      </div>

      {#if loading}
        <LoadingGrid />
      {:else}
        <div class="card-grid">
          {#each pickWallpapers as wallpaper, index (wallpaper.id)}
            <WallpaperCard
              {wallpaper}
              {index}
              active={selected?.id === wallpaper.id}
              {onOpen}
              onFavoriteChange={onFavoriteChange}
              favoriteBusy={favoriteBusyId === wallpaper.id}
            />
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .home-start {
    min-height: min(76vh, 720px);
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: 0 var(--page-gutter) 150px var(--page-inset);
  }

  .home-start-copy {
    max-width: 760px;
  }

  .home-start-label {
    margin: 0 0 16px;
    color: rgba(248, 251, 253, 0.92);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    text-shadow: 0 2px 18px rgba(0, 0, 0, 0.55);
  }

  .home-start h1 {
    margin: 0;
    color: var(--text-primary);
    font-size: clamp(42px, 6vw, 82px);
    line-height: 0.95;
    font-weight: 720;
    letter-spacing: 0;
    text-shadow: 0 2px 26px rgba(0, 0, 0, 0.55);
  }

  .home-start p {
    max-width: 580px;
    margin: 22px 0 0;
    color: rgba(236, 243, 248, 0.86);
    font-size: clamp(15px, 1.1vw, 18px);
    line-height: 1.6;
    text-shadow: 0 2px 18px rgba(0, 0, 0, 0.62);
  }

  .home-start-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 34px;
  }

  .start-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 48px;
    padding: 0 20px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    background: rgba(12, 18, 24, 0.42);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    font: inherit;
    font-size: 14px;
    font-weight: 700;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard);
  }

  .start-action.primary {
    color: #282828;
    background: rgba(255, 255, 255, 0.88);
    border-color: rgba(255, 255, 255, 0.36);
  }

  .start-action:hover {
    transform: translateY(-1px);
    background: rgba(40, 48, 60, 0.7);
  }

  .start-action.primary:hover {
    background: #ffffff;
    box-shadow: 0 14px 28px rgba(0, 0, 0, 0.24);
  }

  .start-action:active {
    transform: translateY(0) scale(0.98);
  }
</style>
