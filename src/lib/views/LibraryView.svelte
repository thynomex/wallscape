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
  import type { LibraryFilter, RenderWindow } from "$lib/types/library";

  let {
    wallpapers,
    selected,
    loading = false,
    favoriteBusyId = null,
    filter = $bindable("all"),
    renderWindow = $bindable(null),
    importing = false,
    importStatus = null,
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
    /** Bound by the owner so the filter outlives this view. */
    filter?: LibraryFilter;
    renderWindow?: RenderWindow | null;
    importing?: boolean;
    importStatus?: string | null;
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
    /** Resolves to true only when the collection was deleted. */
    onDeleteCollection: (collection: Collection) => boolean | Promise<boolean>;
    onCollectionMembershipChange: (
      collection: Collection,
      wallpaper: Wallpaper,
      inCollection: boolean,
    ) => void | Promise<void>;
  } = $props();

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
        : "Browse Wallhaven or MotionBGS, or import files to add wallpapers.",
  );

  $effect(() => {
    if (selectedCollectionId && !selectedCollection) {
      filter = "all";
    }
  });

  async function removeFromSelectedCollection(wallpaper: Wallpaper) {
    if (!selectedCollection) return;
    await onCollectionMembershipChange(selectedCollection, wallpaper, false);
  }
</script>

<section class="content-section view-pad" use:reveal={{ distance: "20px", delay: 40 }}>
  <LibraryHeader
    favoriteCount={favoriteWallpapers.length}
    {rotationEnabled}
    {rotationIntervalMinutes}
    {rotationBusy}
    {onRandomFavorite}
    {onRotationEnabledChange}
    {onRotationIntervalChange}
  />

  <LibraryImportPanel
    {importing}
    status={importStatus}
    {loading}
    {dropActive}
    {onImportFiles}
    {onImportFolder}
    {onCheckFiles}
  />

  <LibraryCollectionPanel
    {wallpapers}
    {collections}
    activeFilter={filter}
    favoriteCount={favoriteWallpapers.length}
    {selectedCollection}
    {selectedCollectionWallpaperIds}
    {collectionBusyId}
    onFilterChange={(nextFilter) => (filter = nextFilter)}
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
      bind:renderWindow
      onShowAll={() => (filter = "all")}
      {onBrowse}
      {onOpen}
      {onFavoriteChange}
      onRemoveFromCollection={removeFromSelectedCollection}
    />
  {/key}
</section>
