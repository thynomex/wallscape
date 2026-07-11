<script lang="ts">
  import LibraryCollectionChips from "$lib/components/LibraryCollectionChips.svelte";
  import LibraryCollectionCreateForm from "$lib/components/LibraryCollectionCreateForm.svelte";
  import LibraryCollectionPicker from "$lib/components/LibraryCollectionPicker.svelte";
  import LibrarySelectedCollectionTools from "$lib/components/LibrarySelectedCollectionTools.svelte";
  import type { Collection, Wallpaper } from "$lib/types/wallpaper";

  let {
    wallpapers,
    collections = [],
    selectedCollection = null,
    selectedCollectionId = null,
    selectedCollectionWallpaperIds,
    collectionBusyId = null,
    onSelectCollection,
    onSelectAll,
    onCreateCollection,
    onDeleteCollection,
    onCollectionMembershipChange,
  }: {
    wallpapers: Wallpaper[];
    collections?: Collection[];
    selectedCollection?: Collection | null;
    selectedCollectionId?: number | null;
    selectedCollectionWallpaperIds: Set<number>;
    collectionBusyId?: number | null;
    onSelectCollection: (collectionId: number) => void;
    onSelectAll: () => void;
    onCreateCollection: (
      name: string,
    ) => Collection | null | void | Promise<Collection | null | void>;
    onDeleteCollection: (collection: Collection) => void | Promise<void>;
    onCollectionMembershipChange: (
      collection: Collection,
      wallpaper: Wallpaper,
      inCollection: boolean,
    ) => void | Promise<void>;
  } = $props();
</script>

<div class="collection-panel">
  <div class="collection-panel-head">
    <div>
      <h3>Collections</h3>
      <p>{collections.length ? `${collections.length} saved` : "Create a wallpaper set."}</p>
    </div>

    <LibraryCollectionCreateForm
      disabled={collectionBusyId !== null}
      {onCreateCollection}
      onCollectionCreated={(collection) => onSelectCollection(collection.id)}
    />
  </div>

  <LibraryCollectionChips
    {collections}
    {selectedCollectionId}
    {onSelectCollection}
  />

  {#if selectedCollection}
    <LibrarySelectedCollectionTools
      {selectedCollection}
      {collectionBusyId}
      {onDeleteCollection}
      onDeleted={onSelectAll}
    />

    <LibraryCollectionPicker
      {wallpapers}
      {selectedCollection}
      {selectedCollectionWallpaperIds}
      {collectionBusyId}
      {onCollectionMembershipChange}
    />
  {/if}
</div>

<style>
  .collection-panel {
    display: grid;
    gap: 12px;
    margin: 0 0 24px;
    padding: 16px 18px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    background: rgba(255, 255, 255, 0.045);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .collection-panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
  }

  .collection-panel h3 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 750;
    color: var(--text-primary);
  }

  .collection-panel p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }

  @media (max-width: 720px) {
    .collection-panel-head {
      align-items: stretch;
      width: 100%;
    }
  }
</style>
