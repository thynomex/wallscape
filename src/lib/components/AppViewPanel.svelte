<script lang="ts">
  import DiscoverView from "$lib/views/DiscoverView.svelte";
  import ExploreView from "$lib/views/ExploreView.svelte";
  import HomeView from "$lib/views/HomeView.svelte";
  import LibraryView from "$lib/views/LibraryView.svelte";
  import type {
    Collection,
    CollectionMembership,
    MotionBgsMeta,
    MotionBgsWallpaper,
    SavedFilter,
    WallhavenMeta,
    WallhavenWallpaper,
    Wallpaper,
  } from "$lib/types/wallpaper";
  import type { RemoteDownloadStates } from "$lib/types/downloads";
  import type { ViewName } from "$lib/utils/viewNavigation";

  type DiscoverSource = "wallhaven" | "motionbgs";

  let {
    activeView,
    searchQuery = $bindable(""),
    discoverSource = "wallhaven",
    wallhavenQuery = $bindable(""),
    wallhavenCategory = $bindable("111"),
    wallhavenPurity = $bindable("100"),
    wallhavenSorting = $bindable("date_added"),
    wallhavenResolution = $bindable(""),
    wallhavenRatio = $bindable(""),
    motionBgsQuery = $bindable(""),
    motionBgsCategory = $bindable("latest"),
    wallpapers,
    featured,
    selected,
    loading = false,
    favoriteBusyId = null,
    wallhavenPage,
    wallhavenMeta,
    wallhavenError,
    wallhavenLoading = false,
    wallhavenResults,
    motionBgsPage,
    motionBgsMeta,
    motionBgsError,
    motionBgsLoading = false,
    motionBgsResults,
    remoteDownloads = {},
    localSavedFilters = [],
    wallhavenSavedFilters = [],
    collections = [],
    collectionMemberships = [],
    collectionBusyId = null,
    importing = false,
    dropActive = false,
    rotationEnabled = false,
    rotationIntervalMinutes = 30,
    rotationBusy = false,
    onApply,
    onOpen,
    onSelectFeatured,
    onFavoriteChange,
    onCarouselPausedChange,
    onSearch,
    onSaveLocalFilter,
    onApplyLocalFilter,
    onDeleteFilter,
    onDiscoverSourceChange,
    onSearchWallhaven,
    onSearchMotionBgs,
    onSaveWallhavenFilter,
    onApplyWallhavenFilter,
    onOpenWallhaven,
    onOpenMotionBgs,
    onDownloadWallhaven,
    onDownloadMotionBgs,
    onBrowseWallhaven,
    onBrowseMotionBgs,
    onRandomFavorite,
    onImportFiles,
    onImportFolder,
    onCheckFiles,
    onRotationEnabledChange,
    onRotationIntervalChange,
    onCreateCollection,
    onDeleteCollection,
    onCollectionMembershipChange,
  }: {
    activeView: ViewName;
    searchQuery: string;
    discoverSource?: DiscoverSource;
    wallhavenQuery: string;
    wallhavenCategory: string;
    wallhavenPurity: string;
    wallhavenSorting: string;
    wallhavenResolution: string;
    wallhavenRatio: string;
    motionBgsQuery: string;
    motionBgsCategory: string;
    wallpapers: Wallpaper[];
    featured: Wallpaper | null;
    selected: Wallpaper | null;
    loading?: boolean;
    favoriteBusyId?: number | null;
    wallhavenPage: number;
    wallhavenMeta: WallhavenMeta | null;
    wallhavenError: string | null;
    wallhavenLoading?: boolean;
    wallhavenResults: WallhavenWallpaper[];
    motionBgsPage: number;
    motionBgsMeta: MotionBgsMeta | null;
    motionBgsError: string | null;
    motionBgsLoading?: boolean;
    motionBgsResults: MotionBgsWallpaper[];
    remoteDownloads?: RemoteDownloadStates;
    localSavedFilters?: SavedFilter[];
    wallhavenSavedFilters?: SavedFilter[];
    collections?: Collection[];
    collectionMemberships?: CollectionMembership[];
    collectionBusyId?: number | null;
    importing?: boolean;
    dropActive?: boolean;
    rotationEnabled?: boolean;
    rotationIntervalMinutes?: number;
    rotationBusy?: boolean;
    onApply: (wallpaper: Wallpaper, monitorId?: string | null) => void | Promise<void>;
    onOpen: (wallpaper: Wallpaper) => void;
    onSelectFeatured: (wallpaper: Wallpaper) => void;
    onFavoriteChange: (wallpaper: Wallpaper, isFavorite: boolean) => void | Promise<void>;
    onCarouselPausedChange: (paused: boolean) => void;
    onSearch: () => void;
    onSaveLocalFilter: (name: string) => void | Promise<void>;
    onApplyLocalFilter: (filter: SavedFilter) => void | Promise<void>;
    onDeleteFilter: (filter: SavedFilter) => void | Promise<void>;
    onDiscoverSourceChange: (source: DiscoverSource) => void;
    onSearchWallhaven: (page?: number) => void | Promise<void>;
    onSearchMotionBgs: (page?: number) => void | Promise<void>;
    onSaveWallhavenFilter: (name: string) => void | Promise<void>;
    onApplyWallhavenFilter: (filter: SavedFilter) => void | Promise<void>;
    onOpenWallhaven: (wallpaper: WallhavenWallpaper) => void;
    onOpenMotionBgs: (wallpaper: MotionBgsWallpaper) => void;
    onDownloadWallhaven: (wallpaper: WallhavenWallpaper) => void | Promise<void>;
    onDownloadMotionBgs: (wallpaper: MotionBgsWallpaper) => void | Promise<void>;
    onBrowseWallhaven: () => void;
    onBrowseMotionBgs: () => void;
    onRandomFavorite: () => void | Promise<void>;
    onImportFiles: () => void | Promise<void>;
    onImportFolder: () => void | Promise<void>;
    onCheckFiles: () => void | Promise<void>;
    onRotationEnabledChange: (enabled: boolean) => void | Promise<void>;
    onRotationIntervalChange: (minutes: number) => void | Promise<void>;
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

{#key activeView}
  <div class="view-transition-panel">
    {#if activeView === "home"}
      <HomeView
        {wallpapers}
        {featured}
        {selected}
        {loading}
        onApply={onApply}
        onOpen={onOpen}
        onSelectFeatured={onSelectFeatured}
        onFavoriteChange={onFavoriteChange}
        {onBrowseWallhaven}
        {onBrowseMotionBgs}
        {onImportFiles}
        {favoriteBusyId}
        {onCarouselPausedChange}
      />
    {:else if activeView === "discover"}
      <DiscoverView
        source={discoverSource}
        bind:query={wallhavenQuery}
        bind:category={wallhavenCategory}
        bind:purity={wallhavenPurity}
        bind:sorting={wallhavenSorting}
        bind:resolution={wallhavenResolution}
        bind:ratio={wallhavenRatio}
        bind:motionBgsQuery
        bind:motionBgsCategory
        page={wallhavenPage}
        meta={wallhavenMeta}
        {motionBgsPage}
        {motionBgsMeta}
        error={wallhavenError}
        {motionBgsError}
        loading={wallhavenLoading}
        {motionBgsLoading}
        results={wallhavenResults}
        {motionBgsResults}
        downloadStates={remoteDownloads}
        savedFilters={wallhavenSavedFilters}
        onSourceChange={onDiscoverSourceChange}
        onSearch={onSearchWallhaven}
        {onSearchMotionBgs}
        onSaveFilter={onSaveWallhavenFilter}
        onApplyFilter={onApplyWallhavenFilter}
        onDeleteFilter={onDeleteFilter}
        onOpen={onOpenWallhaven}
        {onOpenMotionBgs}
        onDownload={onDownloadWallhaven}
        {onDownloadMotionBgs}
      />
    {:else if activeView === "explore"}
      <ExploreView
        bind:query={searchQuery}
        {wallpapers}
        {selected}
        {loading}
        {favoriteBusyId}
        savedFilters={localSavedFilters}
        onSearch={onSearch}
        onSaveFilter={onSaveLocalFilter}
        onApplyFilter={onApplyLocalFilter}
        onDeleteFilter={onDeleteFilter}
        onOpen={onOpen}
        onFavoriteChange={onFavoriteChange}
      />
    {:else}
      <LibraryView
        {wallpapers}
        {selected}
        {loading}
        {favoriteBusyId}
        {collections}
        {collectionMemberships}
        {collectionBusyId}
        onBrowse={onBrowseWallhaven}
        onOpen={onOpen}
        onFavoriteChange={onFavoriteChange}
        onRandomFavorite={onRandomFavorite}
        {importing}
        {dropActive}
        {onImportFiles}
        {onImportFolder}
        {onCheckFiles}
        {rotationEnabled}
        {rotationIntervalMinutes}
        {rotationBusy}
        onRotationEnabledChange={onRotationEnabledChange}
        onRotationIntervalChange={onRotationIntervalChange}
        {onCreateCollection}
        {onDeleteCollection}
        {onCollectionMembershipChange}
      />
    {/if}
  </div>
{/key}
