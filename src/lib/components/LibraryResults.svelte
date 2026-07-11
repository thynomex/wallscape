<script lang="ts">
  import EmptyState from "$lib/components/EmptyState.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import LoadingGrid from "$lib/components/LoadingGrid.svelte";
  import WallpaperCard from "$lib/components/WallpaperCard.svelte";
  import type { Collection, Wallpaper } from "$lib/types/wallpaper";

  const INITIAL_RENDER_COUNT = 48;
  const RENDER_INCREMENT = 48;

  let {
    wallpapers,
    selected,
    loading = false,
    favoriteBusyId = null,
    emptyMessage,
    showAllOnEmpty = false,
    selectedCollection = null,
    collectionBusyId = null,
    onShowAll,
    onBrowse,
    onOpen,
    onFavoriteChange,
    onRemoveFromCollection,
  }: {
    wallpapers: Wallpaper[];
    selected: Wallpaper | null;
    loading?: boolean;
    favoriteBusyId?: number | null;
    emptyMessage: string;
    showAllOnEmpty?: boolean;
    selectedCollection?: Collection | null;
    collectionBusyId?: number | null;
    onShowAll: () => void;
    onBrowse: () => void;
    onOpen: (wallpaper: Wallpaper) => void;
    onFavoriteChange: (wallpaper: Wallpaper, isFavorite: boolean) => void;
    onRemoveFromCollection: (wallpaper: Wallpaper) => void | Promise<void>;
  } = $props();

  let visibleCount = $state(INITIAL_RENDER_COUNT);
  let renderKey = $derived(wallpapers.map((wallpaper) => wallpaper.id).join("|"));
  let visibleWallpapers = $derived(wallpapers.slice(0, visibleCount));
  let hiddenCount = $derived(Math.max(0, wallpapers.length - visibleWallpapers.length));

  $effect(() => {
    renderKey;
    visibleCount = INITIAL_RENDER_COUNT;
  });

  function showMore() {
    visibleCount = Math.min(wallpapers.length, visibleCount + RENDER_INCREMENT);
  }
</script>

<div class="library-results">
  {#if loading}
    <LoadingGrid />
  {:else if wallpapers.length === 0}
    <EmptyState message={emptyMessage} />
    <button class="empty-action" onclick={showAllOnEmpty ? onShowAll : onBrowse}>
      {showAllOnEmpty ? "Show All" : "Browse Wallpapers"}
    </button>
  {:else}
    <div class="card-grid">
      {#each visibleWallpapers as wallpaper (wallpaper.id)}
        <div class="collection-card-wrap">
          <WallpaperCard
            {wallpaper}
            active={selected?.id === wallpaper.id}
            onApply={onOpen}
            onFavoriteChange={onFavoriteChange}
            favoriteBusy={favoriteBusyId === wallpaper.id}
          />
          {#if selectedCollection}
            <button
              class="collection-remove"
              aria-label={`Remove ${wallpaper.title} from ${selectedCollection.name}`}
              disabled={collectionBusyId === selectedCollection.id}
              onclick={() => onRemoveFromCollection(wallpaper)}
            >
              <Icon name="x" size={15} />
            </button>
          {/if}
        </div>
      {/each}
    </div>

    {#if hiddenCount > 0}
      <div class="render-window-actions">
        <button class="show-more" type="button" onclick={showMore}>
          Show {Math.min(RENDER_INCREMENT, hiddenCount)} more
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .library-results {
    animation: library-results-in 360ms var(--ease-emphasized) both;
  }

  .collection-card-wrap {
    position: relative;
    min-width: 0;
  }

  .render-window-actions {
    display: flex;
    justify-content: center;
    margin-top: var(--space-8);
  }

  .show-more {
    min-height: 42px;
    padding: 0 18px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    background: var(--glass-control);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .show-more:hover {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .show-more:active {
    transform: translateY(0) scale(0.98);
  }

  .collection-remove {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 3;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid var(--glass-border);
    border-radius: 50%;
    color: var(--text-primary);
    background: rgba(15, 20, 25, 0.5);
    backdrop-filter: blur(14px);
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .collection-remove:hover:not(:disabled) {
    background: rgba(0, 0, 0, 0.62);
    transform: scale(1.06);
  }

  .collection-remove:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .empty-action {
    margin-top: var(--space-4);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 42px;
    padding: 0 16px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    background: var(--glass-control);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .empty-action:hover {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .empty-action:active {
    transform: translateY(0) scale(0.98);
  }

  @keyframes library-results-in {
    from {
      opacity: 0;
      transform: translate3d(0, 12px, 0);
      filter: blur(8px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0);
      filter: blur(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .library-results {
      animation: none !important;
    }
  }
</style>
