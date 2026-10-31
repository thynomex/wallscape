<script lang="ts">
  import type { Wallpaper } from "$lib/types/wallpaper";
  import { formatDimensions, formatFileSizeMb, wallpaperMediaLabel } from "$lib/utils/media";
  import Icon from "./Icon.svelte";

  let {
    wallpaper,
    favoriteBusy = false,
    onOpen,
    onFavoriteChange,
  }: {
    wallpaper: Wallpaper | null;
    favoriteBusy?: boolean;
    onOpen?: (w: Wallpaper) => void;
    onFavoriteChange?: (w: Wallpaper, isFavorite: boolean) => void;
  } = $props();

  let favorited = $derived(Boolean(wallpaper?.is_favorite));
  let dimensions = $derived(formatDimensions(wallpaper?.width, wallpaper?.height));
  let mediaLabel = $derived(wallpaperMediaLabel(wallpaper));
  let fileSize = $derived(formatFileSizeMb(wallpaper?.file_size_bytes));

  function toggleFavorite() {
    if (!wallpaper || favoriteBusy) return;
    onFavoriteChange?.(wallpaper, !favorited);
  }
</script>

<section class="hero-section">
  {#key wallpaper?.id ?? "empty"}
    <div class="hero-content">
      <p class="hero-label">Featured</p>
      {#if wallpaper}
        <div class="hero-metadata">
          {#if wallpaper.tags.length}
            <span class="cat">{wallpaper.tags[0]}</span>
          {/if}
          {#if dimensions}
            <span>{dimensions}</span>
          {/if}
          <span>{mediaLabel}</span>
          {#if fileSize}
            <span>{fileSize}</span>
          {/if}
        </div>
      {/if}

      <div class="hero-actions">
        <button
          class="btn-primary"
          disabled={!wallpaper}
          onclick={() => wallpaper && onOpen?.(wallpaper)}
        >
          View Wallpaper
          <Icon name="chevron-right" size={16} />
        </button>
        <button
          class="btn-icon"
          class:active={favorited}
          aria-label={favorited ? "Remove from favorites" : "Add to favorites"}
          aria-pressed={favorited}
          disabled={!wallpaper || favoriteBusy}
          onclick={toggleFavorite}
        >
          <Icon name={favorited ? "heart-filled" : "heart"} size={18} />
        </button>
      </div>
    </div>
  {/key}
</section>

<style>
  .hero-section {
    position: relative;
    height: min(76vh, 780px);
    min-height: 660px;
    overflow: visible;
    isolation: isolate;
  }

  .hero-content {
    position: relative;
    z-index: 1;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    /* Start on the shared page edge so the copy lines up with the nav logo
       and the section headings below. */
    padding: 0 0 166px var(--page-inset);
    max-width: calc(var(--page-inset) + 616px);
  }

  .hero-label {
    font-size: clamp(12px, 0.92vw, 16px);
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(248, 251, 253, 0.92);
    margin-bottom: 16px;
    text-shadow: 0 2px 18px rgba(0, 0, 0, 0.55);
    animation: hero-copy-in 680ms var(--ease-emphasized) 120ms backwards;
  }

  .hero-metadata {
    display: flex;
    align-items: center;
    gap: 24px;
    font-size: clamp(14px, 1.05vw, 18px);
    color: rgba(236, 243, 248, 0.86);
    margin-bottom: 30px;
    flex-wrap: wrap;
    text-shadow: 0 2px 18px rgba(0, 0, 0, 0.62);
    animation: hero-copy-in 760ms var(--ease-emphasized) 260ms backwards;
  }

  .hero-metadata .cat {
    text-transform: capitalize;
    color: var(--text-primary);
    font-weight: 500;
  }

  .hero-actions {
    display: flex;
    align-items: center;
    gap: 28px;
    /* `backwards` holds the start frame during the delay, then lets go. With
       `both` the final filter stays, and an element with a filter stops the
       heart button's backdrop-filter from blurring the hero behind it. */
    animation: hero-copy-in 760ms var(--ease-emphasized) 330ms backwards;
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 54px;
    padding: 0 28px;
    background: rgba(255, 255, 255, 0.86);
    border: 1px solid rgba(255, 255, 255, 0.36);
    border-radius: var(--radius-full);
    font-size: clamp(14px, 1.02vw, 18px);
    font-weight: 500;
    color: #282828;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.52),
      0 12px 28px rgba(0, 0, 0, 0.13);
  }

  .btn-primary:hover {
    background: #ffffff;
    transform: translateY(-2px) scale(1.015);
    box-shadow: 0 14px 28px rgba(0, 0, 0, 0.28);
  }

  .btn-primary:active {
    transform: translateY(0) scale(0.985);
  }

  .btn-icon {
    width: 52px;
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(12, 18, 24, 0.42);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid rgba(255, 255, 255, 0.16);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.12);
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.9);
    transition:
      background var(--motion-fast) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .btn-icon:hover {
    background: rgba(40, 48, 60, 0.7);
    transform: translateY(-1px) scale(1.04);
  }

  .btn-icon.active {
    background: rgba(255, 59, 48, 0.2);
    border-color: rgba(255, 59, 48, 0.4);
    color: #ff3b30;
    animation: press-pop 260ms var(--ease-spring);
  }

  @keyframes hero-copy-in {
    from {
      opacity: 0;
      transform: translate3d(0, 18px, 0);
      filter: blur(10px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0);
      filter: blur(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .hero-label,
    .hero-metadata,
    .hero-actions {
      animation: none !important;
    }
  }
</style>
