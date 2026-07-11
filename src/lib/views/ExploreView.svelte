<script lang="ts">
  import type { SavedFilter, Wallpaper } from "$lib/types/wallpaper";
  import { reveal } from "$lib/actions/reveal";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import LoadingGrid from "$lib/components/LoadingGrid.svelte";
  import SavedFilterControls from "$lib/components/SavedFilterControls.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import WallpaperCard from "$lib/components/WallpaperCard.svelte";

  const INITIAL_RENDER_COUNT = 48;
  const RENDER_INCREMENT = 48;

  let {
    query = $bindable(""),
    wallpapers,
    selected,
    loading,
    favoriteBusyId = null,
    savedFilters = [],
    onSearch,
    onSaveFilter,
    onApplyFilter,
    onDeleteFilter,
    onOpen,
    onFavoriteChange,
  }: {
    query: string;
    wallpapers: Wallpaper[];
    selected: Wallpaper | null;
    loading: boolean;
    favoriteBusyId?: number | null;
    savedFilters?: SavedFilter[];
    onSearch: () => void;
    onSaveFilter: (name: string) => void;
    onApplyFilter: (filter: SavedFilter) => void;
    onDeleteFilter: (filter: SavedFilter) => void;
    onOpen: (wallpaper: Wallpaper) => void;
    onFavoriteChange: (wallpaper: Wallpaper, isFavorite: boolean) => void;
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

<section class="content-section view-pad" use:reveal={{ distance: "20px", delay: 40 }}>
  <div class="section-header">
    <div>
      <h2 class="section-title">Explore</h2>
      <p class="section-description">Search the full library by title or tag.</p>
    </div>
  </div>

  <SearchBar
    placeholder="Search wallpapers..."
    bind:value={query}
    onSearch={onSearch}
  />

  <div class="explore-presets">
    <SavedFilterControls
      filters={savedFilters}
      disabled={loading}
      onSave={onSaveFilter}
      onApply={onApplyFilter}
      onDelete={onDeleteFilter}
    />
  </div>

  {#if loading}
    <LoadingGrid />
  {:else if wallpapers.length === 0}
    <EmptyState message="No matches found." />
  {:else}
    <div class="card-grid">
      {#each visibleWallpapers as wallpaper (wallpaper.id)}
        <WallpaperCard
          {wallpaper}
          active={selected?.id === wallpaper.id}
          onApply={onOpen}
          onFavoriteChange={onFavoriteChange}
          favoriteBusy={favoriteBusyId === wallpaper.id}
        />
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
</section>

<style>
  .explore-presets {
    margin: calc(-1 * var(--space-5)) 0 var(--space-8);
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
</style>
