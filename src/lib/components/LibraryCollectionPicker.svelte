<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import WallpaperPreviewMedia from "$lib/components/WallpaperPreviewMedia.svelte";
  import type { Collection, Wallpaper } from "$lib/types/wallpaper";
  import { fileNameFromPath } from "$lib/utils/media";

  let {
    wallpapers,
    selectedCollection,
    selectedCollectionWallpaperIds,
    collectionBusyId = null,
    onCollectionMembershipChange,
  }: {
    wallpapers: Wallpaper[];
    selectedCollection: Collection;
    selectedCollectionWallpaperIds: Set<number>;
    collectionBusyId?: number | null;
    onCollectionMembershipChange: (
      collection: Collection,
      wallpaper: Wallpaper,
      inCollection: boolean,
    ) => void | Promise<void>;
  } = $props();

  let addSearch = $state("");

  let addableWallpapers = $derived(
    wallpapers.filter(
      (wallpaper) =>
        wallpaper.id > 0 && !selectedCollectionWallpaperIds.has(wallpaper.id),
    ),
  );
  let filteredAddableWallpapers = $derived.by(() => {
    const query = addSearch.trim().toLowerCase();
    if (!query) return addableWallpapers;

    return addableWallpapers.filter((wallpaper) =>
      searchableWallpaperText(wallpaper).includes(query),
    );
  });

  async function addWallpaperToSelectedCollection(wallpaper: Wallpaper) {
    await onCollectionMembershipChange(selectedCollection, wallpaper, true);
  }

  function searchableWallpaperText(wallpaper: Wallpaper) {
    return [
      wallpaper.title,
      wallpaper.source,
      wallpaper.source_id,
      wallpaper.file_path,
      `${wallpaper.width}x${wallpaper.height}`,
      ...wallpaper.tags,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
  }

  function mediaLabel(wallpaper: Wallpaper) {
    return wallpaper.media_type === "image" ? "image" : `${wallpaper.fps}fps video`;
  }

  function identityLabel(wallpaper: Wallpaper) {
    if (wallpaper.source === "wallhaven" && wallpaper.source_id) {
      return `Wallhaven ${wallpaper.source_id}`;
    }

    return fileNameFromPath(wallpaper.file_path);
  }
</script>

<div class="collection-picker">
  <div class="collection-picker-head">
    <div>
      <h4>Add wallpapers</h4>
      <p>{filteredAddableWallpapers.length} available</p>
    </div>

    <div class="collection-search">
      <Icon name="search" size={16} />
      <input
        bind:value={addSearch}
        placeholder="Search available wallpapers"
        disabled={collectionBusyId === selectedCollection.id || addableWallpapers.length === 0}
      />
    </div>
  </div>

  {#if addableWallpapers.length === 0}
    <div class="collection-picker-empty">All saved wallpapers are in this collection.</div>
  {:else if filteredAddableWallpapers.length === 0}
    <div class="collection-picker-empty">No matches found.</div>
  {:else}
    <div class="collection-picker-grid">
      {#each filteredAddableWallpapers as wallpaper (wallpaper.id)}
        <button
          class="collection-picker-card"
          disabled={collectionBusyId === selectedCollection.id}
          onclick={() => addWallpaperToSelectedCollection(wallpaper)}
          aria-label={`Add ${wallpaper.title} to ${selectedCollection.name}`}
        >
          <div class="collection-picker-thumb">
            <WallpaperPreviewMedia
              {wallpaper}
              mode="thumbnail"
              showVideo={false}
              imageClass="picker-image"
              videoClass="picker-video"
              fallbackClass="picker-fallback"
              loading="lazy"
              fallbackIconName="play"
              fallbackIconSize={28}
            />
          </div>
          <div class="collection-picker-copy">
            <div class="picker-kicker">{identityLabel(wallpaper)}</div>
            <div class="picker-title">{wallpaper.title}</div>
            <div class="picker-meta">
              <span>{wallpaper.width}&times;{wallpaper.height}</span>
              <span>{mediaLabel(wallpaper)}</span>
              {#if wallpaper.tags[0]}
                <span>{wallpaper.tags[0]}</span>
              {/if}
            </div>
          </div>
          <span class="picker-add">
            <Icon name="plus" size={15} />
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .collection-picker {
    display: grid;
    gap: 12px;
    padding: 12px;
    border-radius: var(--radius-md);
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(0, 0, 0, 0.14);
  }

  .collection-picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .collection-picker h4 {
    margin: 0 0 3px;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 760;
  }

  .collection-picker p {
    margin: 0;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .collection-search {
    position: relative;
    display: flex;
    align-items: center;
    min-width: min(270px, 100%);
    color: var(--text-tertiary);
  }

  .collection-search :global(.icon) {
    position: absolute;
    left: 12px;
    pointer-events: none;
  }

  .collection-search input {
    width: 100%;
    min-width: 0;
    height: 38px;
    padding: 0 12px 0 36px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    background: rgba(18, 25, 31, 0.86);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
  }

  .collection-search input:focus {
    outline: none;
    border-color: var(--accent-blue);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.16);
  }

  .collection-search input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .collection-picker-empty {
    display: grid;
    place-items: center;
    min-height: 104px;
    border: 1px dashed var(--glass-border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.03);
    font-size: 13px;
    font-weight: 650;
  }

  .collection-picker-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 10px;
    max-height: 360px;
    overflow: auto;
    padding-right: 2px;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.22) transparent;
  }

  .collection-picker-card {
    position: relative;
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    min-height: 76px;
    padding: 8px 42px 8px 8px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.055);
    text-align: left;
    overflow: hidden;
    transition:
      background var(--motion-fast) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .collection-picker-card:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.095);
    border-color: rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
  }

  .collection-picker-card:active:not(:disabled) {
    transform: translateY(0) scale(0.99);
  }

  .collection-picker-card:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .collection-picker-thumb {
    position: relative;
    width: 92px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: calc(var(--radius-md) - 4px);
    background: rgba(0, 0, 0, 0.24);
  }

  :global(.picker-image),
  :global(.picker-video),
  :global(.picker-fallback) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  :global(.picker-fallback) {
    display: grid;
    place-items: center;
    color: var(--text-tertiary);
    background: var(--bg-tertiary);
  }

  .collection-picker-copy {
    display: grid;
    align-content: center;
    gap: 3px;
    min-width: 0;
  }

  .picker-kicker,
  .picker-title,
  .picker-meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .picker-kicker {
    color: rgba(255, 255, 255, 0.76);
    font-size: 11px;
    font-weight: 760;
  }

  .picker-title {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 720;
  }

  .picker-meta {
    display: flex;
    gap: 7px;
    color: var(--text-tertiary);
    font-size: 11px;
    text-transform: capitalize;
  }

  .picker-add {
    position: absolute;
    top: 50%;
    right: 10px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
    transform: translateY(-50%);
  }

  @media (max-width: 720px) {
    .collection-picker-head {
      align-items: stretch;
      width: 100%;
    }

    .collection-search {
      width: 100%;
    }

    .collection-picker-grid {
      grid-template-columns: 1fr;
      max-height: 420px;
    }
  }
</style>
