<script lang="ts">
  import type { Wallpaper } from "$lib/types/wallpaper";
  import { formatDimensions, wallpaperMediaLabel } from "$lib/utils/media";
  import Icon from "./Icon.svelte";
  import WallpaperPreviewMedia from "./WallpaperPreviewMedia.svelte";

  let {
    wallpaper,
    active = false,
    favoriteBusy = false,
    index = 0,
    onOpen,
    onFavoriteChange,
  }: {
    wallpaper: Wallpaper;
    active?: boolean;
    favoriteBusy?: boolean;
    /** Position in the grid, used to stagger the entrance animation. */
    index?: number;
    onOpen?: (w: Wallpaper) => void;
    onFavoriteChange?: (w: Wallpaper, isFavorite: boolean) => void;
  } = $props();

  let previewing = $state(false);
  let favorited = $derived(Boolean(wallpaper.is_favorite));
  let hasVideoPreview = $derived(wallpaper.media_type === "video" && Boolean(wallpaper.file_path));
  let dimensions = $derived(formatDimensions(wallpaper.width, wallpaper.height));
  let mediaLabel = $derived(wallpaperMediaLabel(wallpaper));

  function toggleFavorite() {
    if (favoriteBusy) return;
    onFavoriteChange?.(wallpaper, !favorited);
  }

  function startPreview() {
    if (!hasVideoPreview) return;
    previewing = true;
  }

  function stopPreview() {
    previewing = false;
  }
</script>

<!-- The open and favorite controls are sibling buttons: a button may not
     contain another interactive element. -->
<div
  class="wallpaper-card"
  class:active
  class:previewing={previewing}
  style={`--card-index:${index}`}
  role="group"
  aria-label={wallpaper.title}
  onpointerenter={startPreview}
  onpointerleave={stopPreview}
  onfocusin={startPreview}
  onfocusout={stopPreview}
>
  <WallpaperPreviewMedia
    {wallpaper}
    mode="thumbnail"
    showVideo={previewing}
    imageClass="card-image"
    videoClass="card-video"
    fallbackClass="card-image placeholder"
    loading="lazy"
    fallbackIconName="play"
    fallbackIconSize={40}
  />

  <div class="card-overlay">
    <h3 class="card-title">{wallpaper.title}</h3>
    <div class="card-metadata">
      {#if dimensions}
        <span>{dimensions}</span>
      {/if}
      <span>{mediaLabel}</span>
      {#if wallpaper.tags.length}
        <span class="card-tag">{wallpaper.tags[0]}</span>
      {/if}
    </div>
  </div>

  <button
    type="button"
    class="card-open"
    aria-label={`Open ${wallpaper.title}`}
    onclick={() => onOpen?.(wallpaper)}
  ></button>

  {#if onFavoriteChange}
    <button
      type="button"
      class="fav"
      class:on={favorited}
      aria-label={favorited ? "Remove from favorites" : "Add to favorites"}
      aria-pressed={favorited}
      aria-disabled={favoriteBusy}
      onclick={toggleFavorite}
    >
      <Icon name={favorited ? "heart-filled" : "heart"} size={18} />
    </button>
  {/if}
</div>

<style>
  .wallpaper-card {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 20px;
    overflow: hidden;
    padding: 0;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(12, 18, 22, 0.72);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.24);
    transition:
      transform var(--motion-med) var(--ease-emphasized),
      box-shadow var(--motion-med) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard),
      filter var(--motion-med) var(--ease-standard);
    will-change: transform;
    isolation: isolate;
    /* `backwards`, not `both`: a filled animation keeps setting transform
       after it ends, which overrides the hover lift. The stagger uses the
       grid index because Library wraps each card, so :nth-child is always 1. */
    animation: card-enter var(--motion-slow) var(--ease-emphasized) backwards;
    animation-delay: var(--card-enter-delay);
    --card-enter-delay: calc(min(var(--card-index, 0), 5) * 70ms);
  }

  .wallpaper-card::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    border-radius: inherit;
    background:
      linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.22),
        transparent 28%,
        transparent 72%,
        rgba(125, 211, 252, 0.16)
      );
    opacity: 0;
    transition: opacity var(--motion-med) var(--ease-standard);
  }

  .wallpaper-card:hover {
    transform: translateY(-7px) scale(1.012);
    box-shadow: var(--shadow-card-hover);
    border-color: rgba(255, 255, 255, 0.18);
    filter: saturate(1.05);
  }

  .wallpaper-card:hover::before,
  .wallpaper-card.active::before {
    opacity: 1;
  }

  .wallpaper-card:active {
    transform: translateY(-2px) scale(0.992);
  }

  .wallpaper-card:has(.card-open:focus-visible) {
    outline: 2px solid var(--accent-blue);
    outline-offset: 4px;
  }

  .card-open {
    position: absolute;
    inset: 0;
    z-index: 3;
    width: 100%;
    height: 100%;
    padding: 0;
    border: none;
    border-radius: inherit;
    background: transparent;
  }

  /* The ring is drawn on the card, which clips this full-size button. */
  .card-open:focus-visible {
    outline: none;
  }

  .wallpaper-card.active {
    border-color: var(--border-active);
    box-shadow: 0 0 0 2px var(--border-active), 0 12px 28px rgba(0, 0, 0, 0.4);
    animation:
      card-enter var(--motion-slow) var(--ease-emphasized) var(--card-enter-delay) backwards,
      active-card-glow 1600ms var(--ease-standard) 220ms backwards;
  }

  :global(.card-image) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition:
      transform 700ms var(--ease-emphasized),
      filter var(--motion-med) var(--ease-standard);
  }

  .wallpaper-card:hover :global(.card-image),
  .wallpaper-card.previewing :global(.card-image) {
    transform: scale(1.055);
    filter: saturate(1.08);
  }

  :global(.card-video) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity var(--motion-med) var(--ease-standard);
    pointer-events: none;
    background: #0b1014;
  }

  :global(.card-video.ready) {
    opacity: 1;
  }

  :global(.placeholder) {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
    background: var(--bg-tertiary);
  }

  .fav {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 4;
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    color: #ffffff;
    background: rgba(15, 20, 25, 0.44);
    backdrop-filter: blur(14px);
    opacity: 0;
    transition:
      opacity var(--motion-fast) var(--ease-standard),
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard);
  }

  /* Keyboard focus anywhere in the card reveals the heart, so tabbing never
     lands on an invisible control. */
  .wallpaper-card:hover .fav,
  .wallpaper-card:focus-within .fav,
  .fav.on {
    opacity: 1;
  }

  .fav[aria-disabled="true"] {
    cursor: wait;
  }

  .fav.on {
    color: #ff3b30;
    animation: press-pop 260ms var(--ease-spring);
  }

  .fav:hover {
    background: rgba(0, 0, 0, 0.55);
    transform: scale(1.1);
  }

  .fav:active {
    transform: scale(0.92);
  }

  .card-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 24px;
    text-align: left;
    background: linear-gradient(
      to top,
      rgba(0, 0, 0, 0.78) 0%,
      rgba(0, 0, 0, 0.34) 52%,
      transparent 100%
    );
    opacity: 0;
    transform: translateY(8px);
    transition:
      opacity var(--motion-med) var(--ease-standard),
      transform var(--motion-med) var(--ease-emphasized);
  }

  .wallpaper-card:hover .card-overlay,
  .wallpaper-card:focus-within .card-overlay,
  .wallpaper-card.active .card-overlay {
    opacity: 1;
    transform: translateY(0);
  }

  .card-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 4px;
    text-wrap: balance;
    transform: translateY(6px);
    transition: transform var(--motion-med) var(--ease-emphasized);
  }

  .card-metadata {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    font-size: 14px;
    color: var(--text-secondary);
    opacity: 0;
    transform: translateY(8px);
    transition:
      opacity var(--motion-med) var(--ease-standard),
      transform var(--motion-med) var(--ease-emphasized);
    transition-delay: 45ms;
  }

  .wallpaper-card:hover .card-title,
  .wallpaper-card:focus-within .card-title,
  .wallpaper-card.active .card-title,
  .wallpaper-card:hover .card-metadata,
  .wallpaper-card:focus-within .card-metadata,
  .wallpaper-card.active .card-metadata {
    opacity: 1;
    transform: translateY(0);
  }

  .card-tag {
    text-transform: capitalize;
  }

  @keyframes card-enter {
    from {
      opacity: 0;
      transform: translate3d(0, 22px, 0) scale(0.97);
      filter: blur(10px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0) scale(1);
      filter: blur(0);
    }
  }

  @keyframes active-card-glow {
    0% {
      box-shadow: 0 0 0 2px rgba(255, 255, 255, 0), 0 12px 28px rgba(0, 0, 0, 0.4);
    }
    48% {
      box-shadow:
        0 0 0 2px var(--border-active),
        0 0 34px rgba(125, 211, 252, 0.24),
        0 12px 28px rgba(0, 0, 0, 0.4);
    }
    100% {
      box-shadow: 0 0 0 2px var(--border-active), 0 12px 28px rgba(0, 0, 0, 0.4);
    }
  }

  @media (hover: none) {
    .card-overlay {
      opacity: 1;
      transform: translateY(0);
    }
    .fav {
      opacity: 1;
    }
    .card-title,
    .card-metadata {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wallpaper-card {
      animation: none !important;
    }
    .wallpaper-card.active {
      animation: none !important;
    }
  }
</style>
