<script lang="ts">
  import type {
    Collection,
    CollectionMembership,
    Wallpaper,
  } from "$lib/types/wallpaper";
  import { reveal } from "$lib/actions/reveal";
  import LibraryCollectionPanel from "$lib/components/LibraryCollectionPanel.svelte";
  import LibraryHeader from "$lib/components/LibraryHeader.svelte";
  import LibraryImportPanel from "$lib/components/LibraryImportPanel.svelte";
  import LibraryResults from "$lib/components/LibraryResults.svelte";

  type LibraryFilter = "all" | "favorites" | `collection:${number}`;

  let {
    wallpapers,
    selected,
    loading = false,
    favoriteBusyId = null,
    importing = false,
    dropActive = false,
    rotationEnabled = false,
    rotationIntervalMinutes = 30,
    rotationBusy = false,
    collections = [],
    collectionMemberships = [],
    collectionBusyId = null,
    onBrowse,
    onOpen,
    onFavoriteChange,
    onImportFiles,
    onImportFolder,
    onCheckFiles,
    onRandomFavorite,
    onRotationEnabledChange,
    onRotationIntervalChange,
    onCreateCollection,
    onDeleteCollection,
    onCollectionMembershipChange,
  }: {
    wallpapers: Wallpaper[];
    selected: Wallpaper | null;
    loading?: boolean;
    favoriteBusyId?: number | null;
    importing?: boolean;
    dropActive?: boolean;
    rotationEnabled?: boolean;
    rotationIntervalMinutes?: number;
    rotationBusy?: boolean;
    collections?: Collection[];
    collectionMemberships?: CollectionMembership[];
    collectionBusyId?: number | null;
    onBrowse: () => void;
    onOpen: (wallpaper: Wallpaper) => void;
    onFavoriteChange: (wallpaper: Wallpaper, isFavorite: boolean) => void;
    onImportFiles: () => void;
    onImportFolder: () => void;
    onCheckFiles: () => void;
    onRandomFavorite: () => void;
    onRotationEnabledChange: (enabled: boolean) => void;
    onRotationIntervalChange: (minutes: number) => void;
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

  let filter = $state<LibraryFilter>("all");
  let favoriteWallpapers = $derived(
    wallpapers.filter((wallpaper) => wallpaper.is_favorite),
  );
  let selectedCollectionId = $derived(
    filter.startsWith("collection:") ? Number(filter.slice("collection:".length)) : null,
  );
  let selectedCollection = $derived(
    selectedCollectionId
      ? collections.find((collection) => collection.id === selectedCollectionId) ?? null
      : null,
  );
  let selectedCollectionWallpaperIds = $derived.by(
    () =>
      new Set(
        selectedCollectionId
          ? collectionMemberships
              .filter((membership) => membership.collection_id === selectedCollectionId)
              .map((membership) => membership.wallpaper_id)
          : [],
      ),
  );
  let visibleWallpapers = $derived.by(() => {
    if (filter === "favorites") return favoriteWallpapers;
    if (selectedCollection) {
      return wallpapers.filter((wallpaper) =>
        selectedCollectionWallpaperIds.has(wallpaper.id),
      );
    }

    return wallpapers;
  });
  let emptyMessage = $derived(
    selectedCollection
      ? "This collection is empty."
      : filter === "favorites"
        ? "No favorites yet."
        : "Browse Wallhaven or MotionBGs, or import files to add wallpapers.",
  );

  $effect(() => {
    if (selectedCollectionId && !selectedCollection) {
      filter = "all";
    }
  });

  function collectionFilter(id: number): LibraryFilter {
    return `collection:${id}` as LibraryFilter;
  }

  async function removeFromSelectedCollection(wallpaper: Wallpaper) {
    if (!selectedCollection) return;
    await onCollectionMembershipChange(selectedCollection, wallpaper, false);
  }
</script>

<section class="content-section view-pad" use:reveal={{ distance: "20px", delay: 40 }}>
  <LibraryHeader
    activeFilter={filter}
    wallpaperCount={wallpapers.length}
    favoriteCount={favoriteWallpapers.length}
    {rotationEnabled}
    {rotationIntervalMinutes}
    {rotationBusy}
    onFilterChange={(nextFilter) => (filter = nextFilter)}
    {onRandomFavorite}
    {onRotationEnabledChange}
    {onRotationIntervalChange}
  />

  <LibraryImportPanel
    {importing}
    {loading}
    {dropActive}
    {onImportFiles}
    {onImportFolder}
    {onCheckFiles}
  />

  <LibraryCollectionPanel
    {wallpapers}
    {collections}
    {selectedCollection}
    {selectedCollectionId}
    {selectedCollectionWallpaperIds}
    {collectionBusyId}
    onSelectCollection={(collectionId) => (filter = collectionFilter(collectionId))}
    onSelectAll={() => (filter = "all")}
    {onCreateCollection}
    {onDeleteCollection}
    {onCollectionMembershipChange}
  />

  {#key filter}
    <LibraryResults
      wallpapers={visibleWallpapers}
      {selected}
      {loading}
      {favoriteBusyId}
      {emptyMessage}
      showAllOnEmpty={(filter === "favorites" || Boolean(selectedCollection)) && wallpapers.length > 0}
      {selectedCollection}
      {collectionBusyId}
      onShowAll={() => (filter = "all")}
      {onBrowse}
      {onOpen}
      {onFavoriteChange}
      onRemoveFromCollection={removeFromSelectedCollection}
    />
  {/key}
</section>
