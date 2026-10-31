<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import type { LibraryFilter } from "$lib/types/library";
  import type { Collection } from "$lib/types/wallpaper";

  let {
    collections = [],
    activeFilter,
    wallpaperCount,
    favoriteCount,
    onFilterChange,
  }: {
    collections?: Collection[];
    activeFilter: LibraryFilter;
    wallpaperCount: number;
    favoriteCount: number;
    onFilterChange: (filter: LibraryFilter) => void;
  } = $props();

  function collectionFilter(id: number): LibraryFilter {
    return `collection:${id}`;
  }
</script>

<!-- One filter for the grid below: everything, favorites, or a collection. -->
<div class="collection-chip-row" role="group" aria-label="Show in library">
  <button
    class="collection-chip"
    class:active={activeFilter === "all"}
    aria-pressed={activeFilter === "all"}
    onclick={() => onFilterChange("all")}
  >
    All
    <span>{wallpaperCount}</span>
  </button>
  <button
    class="collection-chip"
    class:active={activeFilter === "favorites"}
    aria-pressed={activeFilter === "favorites"}
    onclick={() => onFilterChange("favorites")}
  >
    <Icon name="heart" size={15} />
    Favorites
    <span>{favoriteCount}</span>
  </button>
  {#each collections as collection (collection.id)}
    {@const filter = collectionFilter(collection.id)}
    <button
      class="collection-chip"
      class:active={activeFilter === filter}
      aria-pressed={activeFilter === filter}
      onclick={() => onFilterChange(filter)}
    >
      <Icon name="folder" size={15} />
      {collection.name}
      <span>{collection.wallpaper_count}</span>
    </button>
  {/each}
</div>

<style>
  .collection-chip-row {
    display: flex;
    align-items: stretch;
    gap: 8px;
    flex-wrap: wrap;
  }

  .collection-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 38px;
    padding: 0 13px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    background: var(--glass-control);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .collection-chip span {
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.12);
    font-size: 11px;
  }

  .collection-chip:hover {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .collection-chip.active {
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
    animation: segment-pop 260ms var(--ease-spring) backwards;
  }

  .collection-chip.active span {
    background: rgba(0, 0, 0, 0.1);
  }

  @keyframes segment-pop {
    0% {
      transform: scale(0.96);
    }
    100% {
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .collection-chip.active {
      animation: none !important;
    }
  }
</style>
