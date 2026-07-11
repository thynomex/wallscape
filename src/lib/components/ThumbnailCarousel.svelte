<script lang="ts">
  import type { Wallpaper } from "$lib/types/wallpaper";
  import WallpaperPreviewMedia from "./WallpaperPreviewMedia.svelte";

  let {
    items,
    activeId,
    onSelect,
  }: {
    items: Wallpaper[];
    activeId?: number;
    onSelect?: (w: Wallpaper) => void;
  } = $props();
</script>

<div class="thumbnail-carousel">
  <div class="thumbnail-list">
    {#each items as item (item.id)}
      <button
        class="thumbnail-item"
        class:active={item.id === activeId}
        onclick={() => onSelect?.(item)}
        aria-label={`Preview ${item.title}`}
      >
        <WallpaperPreviewMedia
          wallpaper={item}
          mode="thumbnail"
          showVideo={false}
          imageClass="thumb-image"
          videoClass="thumb-video"
          fallbackClass="thumb-fallback"
          loading="lazy"
        />
      </button>
    {/each}
  </div>
</div>

<style>
  .thumbnail-carousel {
    width: 100%;
    padding: 0 clamp(46px, 4vw, 76px);
    overflow-x: auto;
    overflow-y: hidden;
    white-space: nowrap;
    scrollbar-width: none;
    -ms-overflow-style: none;
    scroll-behavior: smooth;
    will-change: scroll-position;
    mask-image: linear-gradient(
      90deg,
      transparent 0,
      #000 42px,
      #000 calc(100% - 42px),
      transparent 100%
    );
  }

  .thumbnail-carousel::-webkit-scrollbar {
    display: none;
  }

  .thumbnail-list {
    display: flex;
    justify-content: center;
    gap: 22px;
    min-width: max-content;
  }

  .thumbnail-item {
    width: clamp(172px, 12.6vw, 228px);
    height: clamp(98px, 7vw, 140px);
    border-radius: 22px;
    overflow: hidden;
    border: 3px solid transparent;
    padding: 0;
    background: rgba(255, 255, 255, 0.05);
    flex-shrink: 0;
    transition:
      transform var(--motion-med) var(--ease-emphasized),
      border-color var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-med) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard),
      filter var(--motion-med) var(--ease-standard);
    animation: thumb-enter var(--motion-slow) var(--ease-emphasized) both;
  }

  .thumbnail-item:nth-child(2) {
    animation-delay: 55ms;
  }

  .thumbnail-item:nth-child(3) {
    animation-delay: 110ms;
  }

  .thumbnail-item:nth-child(4) {
    animation-delay: 165ms;
  }

  .thumbnail-item:nth-child(5) {
    animation-delay: 220ms;
  }

  .thumbnail-item:nth-child(n + 6) {
    animation-delay: 275ms;
  }

  .thumbnail-item:hover {
    transform: translateY(-4px) scale(1.025);
    filter: saturate(1.08);
  }

  .thumbnail-item.active {
    border-color: rgba(255, 255, 255, 0.9);
    transform: translateY(-4px) scale(1.035);
    box-shadow:
      0 8px 22px rgba(0, 0, 0, 0.22),
      inset 0 0 0 1px rgba(255, 255, 255, 0.05);
  }

  .thumbnail-item:active {
    transform: translateY(-1px) scale(0.985);
  }

  .thumbnail-item :global(.thumb-image),
  .thumbnail-item :global(.thumb-fallback) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition:
      transform 640ms var(--ease-emphasized),
      filter var(--motion-med) var(--ease-standard);
  }

  .thumbnail-item:hover :global(.thumb-image),
  .thumbnail-item:hover :global(.thumb-fallback),
  .thumbnail-item:focus-visible :global(.thumb-image),
  .thumbnail-item:focus-visible :global(.thumb-fallback) {
    transform: scale(1.08);
    filter: saturate(1.08);
  }

  .thumbnail-item :global(.thumb-fallback) {
    background: linear-gradient(135deg, #10161c, #39434d);
  }

  @keyframes thumb-enter {
    from {
      opacity: 0;
      transform: translate3d(0, 18px, 0) scale(0.96);
      filter: blur(8px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0) scale(1);
      filter: blur(0);
    }
  }

  @media (max-width: 640px) {
    .thumbnail-carousel {
      padding: 0 var(--space-6);
      mask-image: linear-gradient(
        90deg,
        #000 0,
        #000 calc(100% - 34px),
        transparent 100%
      );
    }
    .thumbnail-list {
      justify-content: flex-start;
      gap: 14px;
    }
    .thumbnail-item {
      width: 150px;
      height: 104px;
      border-radius: 18px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .thumbnail-item {
      animation: none !important;
    }

    .thumbnail-item :global(.thumb-image),
    .thumbnail-item :global(.thumb-fallback) {
      transition: none !important;
    }
  }
</style>
