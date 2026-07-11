<script lang="ts">
  import type { Wallpaper } from "$lib/types/wallpaper";
  import Badge from "./Badge.svelte";
  import Icon from "./Icon.svelte";
  import WallpaperPreviewMedia from "./WallpaperPreviewMedia.svelte";

  let {
    wallpaper,
    active = false,
    favoriteBusy = false,
    onApply,
    onFavoriteChange,
  }: {
    wallpaper: Wallpaper;
    active?: boolean;
    favoriteBusy?: boolean;
    onApply?: (w: Wallpaper) => void;
    onFavoriteChange?: (w: Wallpaper, isFavorite: boolean) => void;
  } = $props();

  let previewing = $state(false);
  let favorited = $derived(Boolean(wallpaper.is_favorite));
  let hasVideoPreview = $derived(wallpaper.media_type === "video" && Boolean(wallpaper.file_path));
  let mediaLabel = $derived(
    wallpaper.media_type === "image"
      ? "image"
      : `${wallpaper.fps}fps`,
  );

  function toggleFavorite(e: MouseEvent | KeyboardEvent) {
    e.stopPropagation();
    e.preventDefault();
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

<button
  class="wallpaper-card"
  class:active
  class:previewing={previewing}
  onclick={() => onApply?.(wallpaper)}
  aria-label={`Set ${wallpaper.title} as wallpaper`}
  onmouseenter={startPreview}
  onmouseleave={stopPreview}
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

  {#if wallpaper.badge}
    <div class="card-badge">
      <Badge variant={wallpaper.badge}>{wallpaper.badge}</Badge>
    </div>
  {/if}

  <span
    class="fav"
    class:on={favorited}
    role="button"
    tabindex="0"
    aria-label={favorited ? "Remove from favorites" : "Add to favorites"}
    aria-pressed={favorited}
    onclick={toggleFavorite}
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && toggleFavorite(e)}
  >
    <Icon name={favorited ? "heart-filled" : "heart"} size={18} />
  </span>

  <div class="card-overlay">
    <h3 class="card-title">{wallpaper.title}</h3>
    <div class="card-metadata">
      <span>{wallpaper.width}×{wallpaper.height}</span>
      <span>{mediaLabel}</span>
      {#if wallpaper.tags.length}
        <span>{wallpaper.tags[0]}</span>
      {/if}
    </div>
  </div>
</button>

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
    animation: card-enter var(--motion-slow) var(--ease-emphasized) both;
  }

  .wallpaper-card:nth-child(2) {
    animation-delay: 70ms;
  }

  .wallpaper-card:nth-child(3) {
    animation-delay: 140ms;
  }

  .wallpaper-card:nth-child(4) {
    animation-delay: 210ms;
  }

  .wallpaper-card:nth-child(5) {
    animation-delay: 280ms;
  }

  .wallpaper-card:nth-child(n + 6) {
    animation-delay: 350ms;
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

  .wallpaper-card.active {
    border-color: var(--border-active);
    box-shadow: 0 0 0 2px var(--border-active), 0 12px 28px rgba(0, 0, 0, 0.4);
    animation:
      card-enter var(--motion-slow) var(--ease-emphasized) both,
      active-card-glow 1600ms var(--ease-standard) 220ms both;
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

  .card-badge {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 2;
    transition:
      transform var(--motion-med) var(--ease-emphasized),
      opacity var(--motion-med) var(--ease-standard);
  }

  .wallpaper-card:hover .card-badge {
    transform: translateY(-2px);
  }

  .fav {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 2;
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
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

  .wallpaper-card:hover .fav,
  .fav.on {
    opacity: 1;
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
    text-transform: capitalize;
    opacity: 0;
    transform: translateY(8px);
    transition:
      opacity var(--motion-med) var(--ease-standard),
      transform var(--motion-med) var(--ease-emphasized);
    transition-delay: 45ms;
  }

  .wallpaper-card:hover .card-title,
  .wallpaper-card.active .card-title,
  .wallpaper-card:hover .card-metadata,
  .wallpaper-card.active .card-metadata {
    opacity: 1;
    transform: translateY(0);
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

  @media (max-width: 520px) {
    .wallpaper-card {
      border-radius: 18px;
    }
    .card-overlay {
      padding: 18px;
    }
    .card-title {
      font-size: 16px;
    }
    .card-metadata {
      gap: 8px;
      font-size: 12px;
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
