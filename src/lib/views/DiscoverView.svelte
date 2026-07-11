<script lang="ts">
  import type { RemoteDownloadStates } from "$lib/types/downloads";
  import type {
    MotionBgsMeta,
    MotionBgsWallpaper,
    SavedFilter,
    WallhavenMeta,
    WallhavenWallpaper,
  } from "$lib/types/wallpaper";
  import { reveal } from "$lib/actions/reveal";
  import DiscoverCard from "$lib/components/DiscoverCard.svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import LoadingGrid from "$lib/components/LoadingGrid.svelte";
  import SavedFilterControls from "$lib/components/SavedFilterControls.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import { remoteDownloadKey } from "$lib/utils/downloadProgress";

  const WALLHAVEN_DOWNLOAD_SOURCE = "wallhaven";
  const MOTIONBGS_DOWNLOAD_SOURCE = "motionbgs";

  type DiscoverSource = "wallhaven" | "motionbgs";

  let {
    source = "wallhaven",
    query = $bindable(""),
    category = $bindable("111"),
    purity = $bindable("100"),
    sorting = $bindable("date_added"),
    resolution = $bindable(""),
    ratio = $bindable(""),
    motionBgsQuery = $bindable(""),
    motionBgsCategory = $bindable("latest"),
    page,
    meta,
    motionBgsPage,
    motionBgsMeta,
    error,
    motionBgsError,
    loading,
    motionBgsLoading,
    results,
    motionBgsResults,
    downloadStates = {},
    savedFilters = [],
    onSourceChange,
    onSearch,
    onSearchMotionBgs,
    onSaveFilter,
    onApplyFilter,
    onDeleteFilter,
    onOpen,
    onOpenMotionBgs,
    onDownload,
    onDownloadMotionBgs,
  }: {
    source?: DiscoverSource;
    query: string;
    category: string;
    purity: string;
    sorting: string;
    resolution: string;
    ratio: string;
    motionBgsQuery: string;
    motionBgsCategory: string;
    page: number;
    meta: WallhavenMeta | null;
    motionBgsPage: number;
    motionBgsMeta: MotionBgsMeta | null;
    error: string | null;
    motionBgsError: string | null;
    loading: boolean;
    motionBgsLoading: boolean;
    results: WallhavenWallpaper[];
    motionBgsResults: MotionBgsWallpaper[];
    downloadStates?: RemoteDownloadStates;
    savedFilters?: SavedFilter[];
    onSourceChange: (source: DiscoverSource) => void;
    onSearch: (page?: number) => void;
    onSearchMotionBgs: (page?: number) => void;
    onSaveFilter: (name: string) => void;
    onApplyFilter: (filter: SavedFilter) => void;
    onDeleteFilter: (filter: SavedFilter) => void;
    onOpen: (wallpaper: WallhavenWallpaper) => void;
    onOpenMotionBgs: (wallpaper: MotionBgsWallpaper) => void;
    onDownload: (wallpaper: WallhavenWallpaper) => void;
    onDownloadMotionBgs: (wallpaper: MotionBgsWallpaper) => void;
  } = $props();

  const currentError = $derived(source === "motionbgs" ? motionBgsError : error);
  const currentLoading = $derived(
    source === "motionbgs" ? motionBgsLoading : loading,
  );
  const sourceDescription = $derived(
    source === "motionbgs"
      ? "Search MotionBGS and add live MP4 wallpapers to your local library."
      : "Search Wallhaven and add static wallpapers to your local library.",
  );
  const sourceNote = $derived(
    source === "motionbgs"
      ? "MotionBGS wallpapers remain property of their respective owners."
      : "Wallhaven wallpapers remain property of their respective owners.",
  );
</script>

<section class="content-section view-pad" use:reveal={{ distance: "20px", delay: 40 }}>
  <div class="section-header discover-header">
    <div>
      <h2 class="section-title">Discover</h2>
      <p class="section-description">{sourceDescription}</p>
      <p class="source-note">{sourceNote}</p>
    </div>
    {#if source === "wallhaven" && meta}
      <span class="result-count">{meta.total.toLocaleString()} results</span>
    {:else if source === "motionbgs" && motionBgsMeta}
      <span class="result-count">Page {motionBgsMeta.currentPage}</span>
    {/if}
  </div>

  <div class="discover-panel">
    <div class="source-tabs" aria-label="Discover source">
      <button
        type="button"
        class:active={source === "wallhaven"}
        aria-pressed={source === "wallhaven"}
        onclick={() => onSourceChange("wallhaven")}
      >
        Wallhaven
      </button>
      <button
        type="button"
        class:active={source === "motionbgs"}
        aria-pressed={source === "motionbgs"}
        onclick={() => onSourceChange("motionbgs")}
      >
        MotionBGS
      </button>
    </div>

    {#if source === "wallhaven"}
      <SearchBar
        class="discover-search"
        placeholder="Search Wallhaven..."
        bind:value={query}
        onSearch={() => onSearch()}
      />

      <div class="filter-grid">
        <label>
          Category
          <select bind:value={category} onchange={() => onSearch()}>
            <option value="111">All</option>
            <option value="100">General</option>
            <option value="010">Anime</option>
            <option value="001">People</option>
            <option value="110">General + Anime</option>
            <option value="101">General + People</option>
          </select>
        </label>
        <label>
          Purity
          <select bind:value={purity} onchange={() => onSearch()}>
            <option value="100">SFW</option>
            <option value="110">SFW + Sketchy</option>
            <option value="010">Sketchy</option>
          </select>
        </label>
        <label>
          Sort
          <select bind:value={sorting} onchange={() => onSearch()}>
            <option value="date_added">Latest</option>
            <option value="relevance">Relevance</option>
            <option value="hot">Hot</option>
            <option value="toplist">Toplist</option>
            <option value="views">Views</option>
            <option value="favorites">Favorites</option>
            <option value="random">Random</option>
          </select>
        </label>
        <label>
          Minimum Resolution
          <select bind:value={resolution} onchange={() => onSearch()}>
            <option value="">Any</option>
            <option value="1920x1080">1080p</option>
            <option value="2560x1440">1440p</option>
            <option value="3440x1440">Ultrawide</option>
            <option value="3840x2160">4K</option>
          </select>
        </label>
        <label>
          Ratio
          <select bind:value={ratio} onchange={() => onSearch()}>
            <option value="">Any</option>
            <option value="16x9">16:9</option>
            <option value="16x10">16:10</option>
            <option value="21x9">21:9</option>
            <option value="32x9">32:9</option>
          </select>
        </label>
      </div>

      <SavedFilterControls
        filters={savedFilters}
        disabled={loading}
        onSave={onSaveFilter}
        onApply={onApplyFilter}
        onDelete={onDeleteFilter}
      />
    {:else}
      <SearchBar
        class="discover-search"
        placeholder="Search MotionBGS..."
        bind:value={motionBgsQuery}
        onSearch={() => onSearchMotionBgs()}
      />

      <div class="filter-grid motionbgs-filter-grid">
        <label>
          Feed
          <select bind:value={motionBgsCategory} onchange={() => onSearchMotionBgs()}>
            <option value="latest">Latest</option>
            <option value="4k">4K</option>
            <option value="mobile">Mobile</option>
            <option value="gifs">GIFs</option>
          </select>
        </label>
      </div>
    {/if}
  </div>

  {#if currentError}
    <EmptyState message={currentError} />
  {:else if currentLoading}
    <LoadingGrid />
  {:else if source === "wallhaven" && results.length === 0}
    <EmptyState message="No Wallhaven results found." />
  {:else if source === "motionbgs" && motionBgsResults.length === 0}
    <EmptyState message="No MotionBGS results found." />
  {:else if source === "wallhaven"}
    <div class="discover-grid">
      {#each results as result (result.id)}
        <DiscoverCard
          wallpaper={result}
          downloadState={downloadStates[remoteDownloadKey(WALLHAVEN_DOWNLOAD_SOURCE, result.id)] ?? null}
          onOpen={(wallpaper) => onOpen(wallpaper as WallhavenWallpaper)}
          onDownload={(wallpaper) => onDownload(wallpaper as WallhavenWallpaper)}
        />
      {/each}
    </div>

    {#if meta && meta.last_page > 1}
      <div class="pager">
        <button disabled={page <= 1 || loading} onclick={() => onSearch(page - 1)}>
          <Icon name="chevron-left" size={16} />
          Previous
        </button>
        <span>Page {page} of {meta.last_page}</span>
        <button disabled={page >= meta.last_page || loading} onclick={() => onSearch(page + 1)}>
          Next
          <Icon name="chevron-right" size={16} />
        </button>
      </div>
    {/if}
  {:else}
    <div class="discover-grid">
      {#each motionBgsResults as result (result.id)}
        <DiscoverCard
          wallpaper={result}
          downloadStates={downloadStates}
          downloadSource={MOTIONBGS_DOWNLOAD_SOURCE}
          onOpen={(wallpaper) => onOpenMotionBgs(wallpaper as MotionBgsWallpaper)}
          onDownload={(wallpaper) => onDownloadMotionBgs(wallpaper as MotionBgsWallpaper)}
        />
      {/each}
    </div>

    {#if motionBgsMeta && (motionBgsPage > 1 || motionBgsMeta.hasNextPage)}
      <div class="pager">
        <button
          disabled={motionBgsPage <= 1 || motionBgsLoading}
          onclick={() => onSearchMotionBgs(motionBgsPage - 1)}
        >
          <Icon name="chevron-left" size={16} />
          Previous
        </button>
        <span>Page {motionBgsPage}</span>
        <button
          disabled={!motionBgsMeta.hasNextPage || motionBgsLoading}
          onclick={() => onSearchMotionBgs(motionBgsPage + 1)}
        >
          Next
          <Icon name="chevron-right" size={16} />
        </button>
      </div>
    {/if}
  {/if}
</section>

<style>
  .source-tabs {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    width: fit-content;
    padding: 4px;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.08);
  }

  .source-tabs button {
    border: none;
    border-radius: var(--radius-full);
    padding: 8px 14px;
    color: var(--text-secondary);
    background: transparent;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition:
      color var(--motion-fast) var(--ease-standard),
      background var(--motion-fast) var(--ease-standard);
  }

  .source-tabs button:hover,
  .source-tabs button.active {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.12);
  }

  .motionbgs-filter-grid {
    grid-template-columns: minmax(180px, 260px);
  }

  .source-note {
    margin: 6px 0 0;
    color: var(--text-tertiary);
    font-size: 12px;
  }
</style>
