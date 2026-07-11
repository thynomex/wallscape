<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "$lib/stores/wallpapers.svelte";
  import "$lib/styles/views.css";
  import AmbientBackground from "$lib/components/AmbientBackground.svelte";
  import AppDetailModals from "$lib/components/AppDetailModals.svelte";
  import AppViewPanel from "$lib/components/AppViewPanel.svelte";
  import ImportDropOverlay from "$lib/components/ImportDropOverlay.svelte";
  import MonitorStatusBar from "$lib/components/MonitorStatusBar.svelte";
  import NavigationBar from "$lib/components/NavigationBar.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import ToastContainer from "$lib/components/ToastContainer.svelte";
  import WindowControls from "$lib/components/WindowControls.svelte";
  import { motionBgsToWallpaper } from "$lib/mappers/motionbgs";
  import { wallhavenToWallpaper } from "$lib/mappers/wallhaven";
  import {
    normalizeFavoriteRotationInterval,
    settingsStore,
  } from "$lib/stores/settings.svelte";
  import { appVisibility } from "$lib/stores/appVisibility.svelte";
  import { toastStore } from "$lib/stores/toasts.svelte";
  import type {
    Collection,
    MotionBgsWallpaper,
    SavedFilter,
    WallhavenWallpaper,
    Wallpaper,
  } from "$lib/types/wallpaper";
  import {
    deleteCollectionConfirmationMessage,
    removeWallpaperConfirmationMessage,
  } from "$lib/utils/confirmationMessages";
  import {
    createCollection as createCollectionWorkflow,
    deleteCollection as deleteCollectionWorkflow,
    setCollectionMembership as setCollectionMembershipWorkflow,
  } from "$lib/utils/collectionWorkflow";
  import { indexById, nextCircularIndex } from "$lib/utils/detailNavigation";
  import {
    runFavoriteRotation,
    updateFavoriteRotationEnabled as updateFavoriteRotationEnabledWorkflow,
    updateFavoriteRotationInterval as updateFavoriteRotationIntervalWorkflow,
    updateWallpaperFavorite,
    type FavoriteRotationReason,
  } from "$lib/utils/favoriteWorkflow";
  import { registerImportDropListener } from "$lib/utils/importDropListener";
  import { selectWallpaperFiles, selectWallpaperFolders } from "$lib/utils/importDialogs";
  import { importSelectedWallpaperPaths } from "$lib/utils/importWorkflow";
  import {
    checkLibraryFiles as checkLibraryFilesWorkflow,
    regenerateWallpaperThumbnail,
    removeWallpaperFromLibrary,
    revealWallpaperInExplorer,
  } from "$lib/utils/libraryMaintenanceWorkflow";
  import { resolveWallpaperThumbnailSrc } from "$lib/utils/media";
  import {
    motionBgsSearchArgs,
    type MotionBgsSearchFields,
  } from "$lib/utils/motionbgsSearchArgs";
  import {
    downloadMotionBgsDetailWallpaper,
    downloadMotionBgsWallpaper,
  } from "$lib/utils/motionbgsWorkflow";
  import {
    applyLocalSavedFilter,
    applyWallhavenSavedFilter,
    deleteSavedFilter as deleteSavedFilterWorkflow,
    saveLocalSavedFilter,
    saveWallhavenSavedFilter,
  } from "$lib/utils/savedFilterWorkflow";
  import {
    effectiveWallhavenSorting,
    wallhavenSearchArgs,
    type WallhavenSearchFields,
  } from "$lib/utils/wallhavenSearchArgs";
  import {
    downloadWallhavenDetailWallpaper,
    downloadWallhavenWallpaper,
  } from "$lib/utils/wallhavenWorkflow";
  import {
    applyWallpaper as applyWallpaperWorkflow,
    cancelAppliedWallpaper as cancelAppliedWallpaperWorkflow,
    canCancelWallpaper as canCancelWallpaperWorkflow,
    restoreOriginalWallpaper as restoreOriginalWallpaperWorkflow,
  } from "$lib/utils/wallpaperLifecycleWorkflow";
  import {
    importFailureMessage,
    type ImportSource,
  } from "$lib/utils/toastMessages";
  import { isViewName, type ViewName } from "$lib/utils/viewNavigation";

  const CAROUSEL_INTERVAL_MS = 7_000;
  type DiscoverSource = "wallhaven" | "motionbgs";

  let activeView = $state<ViewName>("home");
  let searchQuery = $state("");
  let discoverSource = $state<DiscoverSource>("wallhaven");
  let wallhavenQuery = $state("");
  let wallhavenCategory = $state("111");
  let wallhavenPurity = $state("100");
  let wallhavenSorting = $state("date_added");
  let wallhavenResolution = $state("");
  let wallhavenRatio = $state("");
  let wallhavenPage = $state(1);
  let motionBgsQuery = $state("");
  let motionBgsCategory = $state("latest");
  let motionBgsPage = $state(1);
  let detailIndex = $state<number | null>(null);
  let wallhavenDetailIndex = $state<number | null>(null);
  let motionBgsDetailIndex = $state<number | null>(null);
  let showSettings = $state(false);
  let scrollEl = $state<HTMLDivElement | null>(null);
  let carouselPaused = $state(false);
  let carouselTick = $state(0);
  let favoriteBusyId = $state<number | null>(null);
  let rotationBusy = $state(false);
  let rotationResetTick = $state(0);
  let applyTargetMonitorId = $state<string | null>(null);
  let applyingWallpaperPath = $state<string | null>(null);
  let dropActive = $state(false);
  let dropItemCount = $state(0);
  let thumbnailBusyId = $state<number | null>(null);
  let collectionBusyId = $state<number | null>(null);

  onMount(() => {
    initializeApp();
    const stopVisibilityObserver = appVisibility.start();

    let unlistenDrop: (() => void) | null = null;
    registerImportDropListener({
      onStateChange: ({ active, itemCount }) => {
        dropActive = active;
        dropItemCount = itemCount;
      },
      onDrop: (paths) => importSelectedPaths(paths, "dropped"),
    })
      .then((unlisten) => {
        unlistenDrop = unlisten;
      })
      .catch((error) => console.warn("File drop listener unavailable:", error));

    return () => {
      unlistenDrop?.();
      stopVisibilityObserver();
    };
  });

  let detailWallpaper = $derived(
    detailIndex !== null ? store.wallpapers[detailIndex] ?? null : null,
  );
  let wallhavenDetail = $derived(
    wallhavenDetailIndex !== null
      ? wallhavenToWallpaper(store.wallhavenResults[wallhavenDetailIndex] ?? null)
      : null,
  );
  let motionBgsDetail = $derived(
    motionBgsDetailIndex !== null
      ? motionBgsToWallpaper(store.motionBgsResults[motionBgsDetailIndex] ?? null)
      : null,
  );
  let ambientSrc = $derived(
    resolveWallpaperThumbnailSrc(motionBgsDetail) ??
      resolveWallpaperThumbnailSrc(wallhavenDetail) ??
      resolveWallpaperThumbnailSrc(detailWallpaper) ??
      resolveWallpaperThumbnailSrc(store.featured),
  );
  let favoriteRotationSignature = $derived(
    store.wallpapers
      .filter((wallpaper) => wallpaper.is_favorite)
      .map((wallpaper) => wallpaper.id)
      .join("|"),
  );
  let dropHint = $derived(
    dropItemCount === 1
      ? "Drop file to import"
      : dropItemCount > 1
        ? `Drop ${dropItemCount} items to import`
        : "Drop wallpapers to import",
  );
  let localSavedFilters = $derived(
    store.savedFilters.filter((filter) => filter.filter_type === "local"),
  );
  let wallhavenSavedFilters = $derived(
    store.savedFilters.filter((filter) => filter.filter_type === "wallhaven"),
  );

  $effect(() => {
    if (
      applyTargetMonitorId &&
      !store.monitors.some((monitor) => monitor.id === applyTargetMonitorId)
    ) {
      applyTargetMonitorId = null;
    }
  });

  $effect(() => {
    const carouselWallpapers = store.wallpapers.slice(0, 6);
    const itemCount = carouselWallpapers.length;
    const activeFeaturedId = store.featured?.id;
    const autoplayAvailable =
      activeView === "home" && !carouselPaused && itemCount > 1 && appVisibility.visible;

    if (!autoplayAvailable) return;

    const interval = window.setInterval(() => {
      const activeIndex = activeFeaturedId
        ? indexById(carouselWallpapers, activeFeaturedId) ?? -1
        : -1;
      const nextIndex = nextCircularIndex(activeIndex, itemCount, 1);
      store.featured = nextIndex === null ? null : carouselWallpapers[nextIndex] ?? null;
    }, CAROUSEL_INTERVAL_MS);

    carouselTick;

    return () => window.clearInterval(interval);
  });

  $effect(() => {
    const enabled = settingsStore.settings.favoriteRotationEnabled;
    const intervalMinutes = normalizeFavoriteRotationInterval(
      settingsStore.settings.favoriteRotationIntervalMinutes,
    );
    const hasFavorites = Boolean(favoriteRotationSignature);

    rotationResetTick;

    if (!enabled || !hasFavorites) return;

    const interval = window.setInterval(() => {
      rotateFavoriteWallpaper("scheduled");
    }, intervalMinutes * 60_000);

    return () => window.clearInterval(interval);
  });

  async function initializeApp() {
    store.loadMonitors();
    store.loadBackupStatus();
    store.loadPreviousWallpaperStatus();

    await Promise.all([
      store.loadWallpapers(),
      store.loadOrganization(),
      store.loadWallpaperHistory(),
      settingsStore.load(),
    ]);

    if (
      settingsStore.settings.favoriteRotationOnStartup &&
      store.wallpapers.some((wallpaper) => wallpaper.is_favorite)
    ) {
      await rotateFavoriteWallpaper("startup");
    }
  }

  async function handleApply(wallpaper: Wallpaper, monitorId: string | null = null) {
    if (!wallpaper.file_path || applyingWallpaperPath) return;

    applyingWallpaperPath = wallpaper.file_path;
    try {
      const result = await applyWallpaperWorkflow(wallpaper, monitorId, store.monitors, {
        apply: (wallpaper, monitorId) => store.apply(wallpaper, monitorId),
      });
      if (!result.ok) {
        toastStore.error(result.error);
        return;
      }

      toastStore.success(
        result.value.message,
        3000,
        resolveWallpaperThumbnailSrc(result.value.wallpaper) ?? undefined,
      );
    } finally {
      applyingWallpaperPath = null;
    }
  }

  async function rotateFavoriteWallpaper(reason: FavoriteRotationReason) {
    if (rotationBusy) return;

    rotationBusy = true;
    try {
      const outcome = await runFavoriteRotation(reason, store.wallpapers, {
        rotateRandomFavorite: () => store.rotateRandomFavorite(),
      });
      if (!outcome.ok) {
        if (outcome.shouldToast) {
          toastStore.error(outcome.error);
        }
        return;
      }

      if (!outcome.rotated) return;

      carouselTick += 1;

      if (outcome.shouldToast) {
        toastStore.success(
          outcome.message,
          3000,
          resolveWallpaperThumbnailSrc(outcome.wallpaper) ?? undefined,
        );
      }
    } finally {
      rotationBusy = false;
      rotationResetTick += 1;
    }
  }

  async function handleFavoriteChange(wallpaper: Wallpaper, isFavorite: boolean) {
    if (favoriteBusyId === wallpaper.id) return;

    favoriteBusyId = wallpaper.id;
    try {
      const result = await updateWallpaperFavorite(wallpaper, isFavorite, {
        setFavorite: (wallpaper, isFavorite) =>
          store.setFavorite(wallpaper, isFavorite),
      });
      if (!result.ok) {
        toastStore.error(result.error);
        return;
      }

      toastStore.success(
        result.value.message,
        2200,
        resolveWallpaperThumbnailSrc(result.value.wallpaper) ?? undefined,
      );
    } finally {
      favoriteBusyId = null;
    }
  }

  async function handleRandomFavorite() {
    await rotateFavoriteWallpaper("manual");
  }

  async function handleRotationEnabledChange(enabled: boolean) {
    const previous = settingsStore.settings.favoriteRotationEnabled;
    const result = await updateFavoriteRotationEnabledWorkflow(
      enabled,
      store.wallpapers.some((wallpaper) => wallpaper.is_favorite),
      previous,
      {
        updateSettings: (patch) => settingsStore.update(patch),
        settingsError: () => settingsStore.error,
      },
    );

    if (!result.ok) {
      toastStore.error(result.error);
    } else if (result.message) {
      toastStore.success(result.message);
    }
  }

  async function handleRotationIntervalChange(minutes: number) {
    const previous = settingsStore.settings.favoriteRotationIntervalMinutes;
    const result = await updateFavoriteRotationIntervalWorkflow(minutes, previous, {
      updateSettings: (patch) => settingsStore.update(patch),
      settingsError: () => settingsStore.error,
    });

    if (!result.ok) {
      toastStore.error(result.error);
    } else if (result.message) {
      toastStore.success(result.message);
    }
  }

  async function handleRemove(wallpaper: Wallpaper) {
    if (wallpaper.id <= 0) return;

    const confirmed = window.confirm(removeWallpaperConfirmationMessage(wallpaper));

    if (!confirmed) return;

    const result = await removeWallpaperFromLibrary(wallpaper, {
      removeWallpaper: (wallpaper) => store.removeWallpaper(wallpaper),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success(result.value.message);
    detailIndex = null;
  }

  async function handleReveal(wallpaper: Wallpaper) {
    const result = await revealWallpaperInExplorer(wallpaper, {
      revealInExplorer: (wallpaper) => store.revealInExplorer(wallpaper),
    });
    if (!result.ok) {
      toastStore.error(result.error);
    }
  }

  async function handleRegenerateThumbnail(wallpaper: Wallpaper) {
    if (thumbnailBusyId === wallpaper.id) return;

    thumbnailBusyId = wallpaper.id;
    try {
      const result = await regenerateWallpaperThumbnail(wallpaper, {
        regenerateThumbnail: (wallpaper, probe) =>
          store.regenerateThumbnail(wallpaper, probe),
      }, {
        setStatus: (status) => (store.importStatus = status),
      });
      if (!result.ok) {
        toastStore.error(result.error);
        return;
      }

      toastStore.success(
        result.value.message,
        2600,
        resolveWallpaperThumbnailSrc(result.value.wallpaper) ?? undefined,
      );
    } finally {
      thumbnailBusyId = null;
    }
  }

  function openDetail(wallpaper: Wallpaper) {
    detailIndex = indexById(store.wallpapers, wallpaper.id);
    wallhavenDetailIndex = null;
    motionBgsDetailIndex = null;
  }

  function openWallhavenDetail(wallpaper: WallhavenWallpaper) {
    detailIndex = null;
    wallhavenDetailIndex = indexById(store.wallhavenResults, wallpaper.id);
    motionBgsDetailIndex = null;
  }

  function openMotionBgsDetail(wallpaper: MotionBgsWallpaper) {
    detailIndex = null;
    wallhavenDetailIndex = null;
    motionBgsDetailIndex = indexById(store.motionBgsResults, wallpaper.id);
  }

  function navigateTo(view: string) {
    if (!isViewName(view) || activeView === view) return;

    activeView = view;
    if (view === "discover") {
      ensureDiscoverSourceLoaded(discoverSource);
    }

    requestAnimationFrame(() => {
      scrollEl?.scrollTo({ top: 0, behavior: "smooth" });
    });
  }

  function stepDetail(direction: number) {
    if (detailIndex === null) return;
    detailIndex = nextCircularIndex(detailIndex, store.wallpapers.length, direction);
  }

  function stepWallhavenDetail(direction: number) {
    if (wallhavenDetailIndex === null) return;
    wallhavenDetailIndex = nextCircularIndex(
      wallhavenDetailIndex,
      store.wallhavenResults.length,
      direction,
    );
  }

  function stepMotionBgsDetail(direction: number) {
    if (motionBgsDetailIndex === null) return;
    motionBgsDetailIndex = nextCircularIndex(
      motionBgsDetailIndex,
      store.motionBgsResults.length,
      direction,
    );
  }

  function selectFeaturedWallpaper(wallpaper: Wallpaper) {
    store.featured = wallpaper;
    carouselTick += 1;
  }

  function canCancelWallpaper(wallpaper: Wallpaper | null) {
    return canCancelWallpaperWorkflow(store.selected, store.previous, wallpaper);
  }

  async function handleRestore() {
    const result = await restoreOriginalWallpaperWorkflow({
      restoreOriginalWallpaper: () => store.restoreOriginalWallpaper(),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success(result.value.message);
  }

  async function handleUndoWallpaperHistory() {
    const result = await store.undoWallpaperHistory();
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success(`Restored "${result.value.title}" from history`);
  }

  async function handleCancelWallpaper(wallpaper: Wallpaper) {
    const result = await cancelAppliedWallpaperWorkflow(wallpaper, {
      cancelAppliedWallpaper: (wallpaper) => store.cancelAppliedWallpaper(wallpaper),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    detailIndex = null;
    wallhavenDetailIndex = null;
    motionBgsDetailIndex = null;
    toastStore.success(result.value.message);
  }

  function runSearch() {
    store.search(searchQuery);
  }

  async function handleSaveLocalFilter(name: string) {
    const result = await saveLocalSavedFilter(name, searchQuery, {
      saveFilter: (name, filterType, payload) =>
        store.saveFilter(name, filterType, payload),
    });

    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success("Filter saved");
  }

  async function handleApplyLocalFilter(filter: SavedFilter) {
    await applyLocalSavedFilter(filter, async (query) => {
      searchQuery = query;
      await store.search(query);
    });
    toastStore.success("Filter applied");
  }

  async function handleSaveWallhavenFilter(name: string) {
    const result = await saveWallhavenSavedFilter(name, currentWallhavenFields(), {
      saveFilter: (name, filterType, payload) =>
        store.saveFilter(name, filterType, payload),
    });

    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success("Wallhaven filter saved");
  }

  async function handleApplyWallhavenFilter(filter: SavedFilter) {
    await applyWallhavenSavedFilter(filter, async (fields) => {
      wallhavenQuery = fields.query;
      wallhavenCategory = fields.categories;
      wallhavenPurity = fields.purity;
      wallhavenSorting = fields.sorting;
      wallhavenResolution = fields.atleast;
      wallhavenRatio = fields.ratios;
      await searchWallhaven(1);
    });
    toastStore.success("Wallhaven filter applied");
  }

  async function handleDeleteSavedFilter(filter: SavedFilter) {
    const result = await deleteSavedFilterWorkflow(filter, {
      deleteSavedFilter: (filter) => store.deleteSavedFilter(filter),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success("Filter deleted");
  }

  function currentWallhavenFields(): WallhavenSearchFields {
    return {
      query: wallhavenQuery,
      categories: wallhavenCategory,
      purity: wallhavenPurity,
      sorting: wallhavenSorting,
      atleast: wallhavenResolution,
      ratios: wallhavenRatio,
    };
  }

  function wallhavenArgs(page = wallhavenPage) {
    return wallhavenSearchArgs(currentWallhavenFields(), page);
  }

  function currentMotionBgsFields(): MotionBgsSearchFields {
    return {
      query: motionBgsQuery,
      category: motionBgsCategory,
    };
  }

  function motionBgsArgs(page = motionBgsPage) {
    return motionBgsSearchArgs(currentMotionBgsFields(), page);
  }

  async function searchWallhaven(page = 1) {
    wallhavenSorting = effectiveWallhavenSorting(wallhavenQuery, wallhavenSorting);
    wallhavenPage = page;
    await store.searchWallhaven(wallhavenArgs(page));
  }

  async function searchMotionBgs(page = 1) {
    motionBgsPage = page;
    await store.searchMotionBgs(motionBgsArgs(page));
  }

  function handleDiscoverSourceChange(source: DiscoverSource) {
    discoverSource = source;
    ensureDiscoverSourceLoaded(source);
  }

  function ensureDiscoverSourceLoaded(source: DiscoverSource) {
    if (source === "wallhaven") {
      if (store.wallhavenMeta === null && !store.wallhavenLoading) {
        searchWallhaven(1);
      }
      return;
    }

    if (store.motionBgsMeta === null && !store.motionBgsLoading) {
      searchMotionBgs(1);
    }
  }

  function browseDiscoverSource(source: DiscoverSource) {
    discoverSource = source;
    ensureDiscoverSourceLoaded(source);
    navigateTo("discover");
  }

  async function downloadWallhaven(wallpaper: WallhavenWallpaper) {
    const result = await downloadWallhavenWallpaper(wallpaper, {
      downloadWallhaven: (wallpaper) => store.downloadWallhaven(wallpaper),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success(
      "Added to library",
      3000,
      resolveWallpaperThumbnailSrc(result.value.wallpaper) ??
        result.value.fallbackThumbnailSrc,
    );
  }

  async function downloadMotionBgs(wallpaper: MotionBgsWallpaper) {
    const result = await downloadMotionBgsWallpaper(wallpaper, {
      downloadMotionBgs: (wallpaper) => store.downloadMotionBgs(wallpaper),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    toastStore.success(
      "Added to library",
      3000,
      resolveWallpaperThumbnailSrc(result.value.wallpaper) ??
        result.value.fallbackThumbnailSrc,
    );
  }

  async function applyWallhavenDetail(wallpaper: Wallpaper, monitorId: string | null = null) {
    const imported = await downloadWallhavenDetailWallpaper(
      wallpaper,
      store.wallhavenResults,
      {
        downloadWallhaven: (wallpaper) => store.downloadWallhaven(wallpaper),
      },
    );
    if (!imported.ok) {
      toastStore.error(imported.error);
      return;
    }

    await handleApply(imported.value, monitorId);
  }

  async function applyMotionBgsDetail(wallpaper: Wallpaper, monitorId: string | null = null) {
    const imported = await downloadMotionBgsDetailWallpaper(
      wallpaper,
      store.motionBgsResults,
      {
        downloadMotionBgs: (wallpaper) => store.downloadMotionBgs(wallpaper),
      },
    );
    if (!imported.ok) {
      toastStore.error(imported.error);
      return;
    }

    await handleApply(imported.value, monitorId);
  }

  async function handleUpload() {
    await handleImportFiles();
  }

  async function handleImportFiles() {
    try {
      const paths = await selectWallpaperFiles();
      await importSelectedPaths(paths, "selected");
    } catch (error) {
      const errorMessage = importFailureMessage("selected", error);
      store.error = errorMessage;
      toastStore.error(errorMessage);
    }
  }

  async function handleImportFolder() {
    try {
      const paths = await selectWallpaperFolders();
      await importSelectedPaths(paths, "folder");
    } catch (error) {
      const errorMessage = importFailureMessage("folder", error);
      store.error = errorMessage;
      toastStore.error(errorMessage);
    }
  }

  async function importSelectedPaths(paths: string[], source: ImportSource) {
    if (!paths.length || store.importing) return;

    const outcome = await importSelectedWallpaperPaths(
      paths,
      source,
      {
        scanImportPaths: (paths) => store.scanImportPaths(paths),
        importWallpapers: (items) => store.importWallpapers(items),
      },
      {
        setStatus: (status) => (store.importStatus = status),
        setWarnings: (warnings) => (store.importWarnings = warnings),
      },
    );

    if (!outcome.ok) {
      if (outcome.kind === "unexpected") {
        store.error = outcome.error;
      }
      toastStore.error(outcome.error);
      return;
    }

    activeView = "library";
    toastStore.success(
      outcome.message,
      3600,
      resolveWallpaperThumbnailSrc(outcome.thumbnailWallpaper) ?? undefined,
    );
  }

  async function handleCheckFiles() {
    const result = await checkLibraryFilesWorkflow({
      detectBrokenWallpapers: () => store.detectBrokenWallpapers(),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return;
    }

    if (result.value.severity === "warning") {
      toastStore.warning(result.value.message);
    } else {
      toastStore.success(result.value.message);
    }
  }

  async function handleCreateCollection(name: string): Promise<Collection | null> {
    const result = await createCollectionWorkflow(name, {
      createCollection: (name) => store.createCollection(name),
    });
    if (!result.ok) {
      toastStore.error(result.error);
      return null;
    }

    toastStore.success(result.value.message);
    return result.value.value;
  }

  async function handleDeleteCollection(collection: Collection) {
    const confirmed = window.confirm(deleteCollectionConfirmationMessage(collection));

    if (!confirmed) return;

    collectionBusyId = collection.id;
    try {
      const result = await deleteCollectionWorkflow(collection, {
        deleteCollection: (collection) => store.deleteCollection(collection),
      });
      if (!result.ok) {
        toastStore.error(result.error);
        return;
      }

      toastStore.success(result.value.message);
    } finally {
      collectionBusyId = null;
    }
  }

  async function handleCollectionMembershipChange(
    collection: Collection,
    wallpaper: Wallpaper,
    inCollection: boolean,
  ) {
    collectionBusyId = collection.id;
    try {
      const result = await setCollectionMembershipWorkflow(
        collection,
        wallpaper,
        inCollection,
        {
          setCollectionMembership: (collection, wallpaper, inCollection) =>
            store.setCollectionMembership(collection, wallpaper, inCollection),
        },
      );
      if (!result.ok) {
        toastStore.error(result.error);
        return;
      }

      toastStore.success(result.value.message);
    } finally {
      collectionBusyId = null;
    }
  }

</script>

<div class="shell" class:home-active={activeView === "home"}>
  <AmbientBackground src={ambientSrc} />
  <WindowControls />

  {#if dropActive}
    <ImportDropOverlay hint={dropHint} />
  {/if}

  <div class="scroll" bind:this={scrollEl}>
    <NavigationBar
      {activeView}
      onNavigate={navigateTo}
      onUpload={handleUpload}
      onSettings={() => (showSettings = true)}
    />

    <AppViewPanel
      {activeView}
      bind:searchQuery
      {discoverSource}
      bind:wallhavenQuery
      bind:wallhavenCategory
      bind:wallhavenPurity
      bind:wallhavenSorting
      bind:wallhavenResolution
      bind:wallhavenRatio
      bind:motionBgsQuery
      bind:motionBgsCategory
      wallpapers={store.wallpapers}
      featured={store.featured}
      selected={store.selected}
      loading={store.loading}
      {favoriteBusyId}
      {wallhavenPage}
      wallhavenMeta={store.wallhavenMeta}
      wallhavenError={store.wallhavenError}
      wallhavenLoading={store.wallhavenLoading}
      wallhavenResults={store.wallhavenResults}
      {motionBgsPage}
      motionBgsMeta={store.motionBgsMeta}
      motionBgsError={store.motionBgsError}
      motionBgsLoading={store.motionBgsLoading}
      motionBgsResults={store.motionBgsResults}
      remoteDownloads={store.remoteDownloads}
      {localSavedFilters}
      {wallhavenSavedFilters}
      collections={store.collections}
      collectionMemberships={store.collectionMemberships}
      {collectionBusyId}
      importing={store.importing}
      {dropActive}
      rotationEnabled={settingsStore.settings.favoriteRotationEnabled}
      rotationIntervalMinutes={settingsStore.settings.favoriteRotationIntervalMinutes}
      {rotationBusy}
      onApply={handleApply}
      onOpen={openDetail}
      onSelectFeatured={selectFeaturedWallpaper}
      onFavoriteChange={handleFavoriteChange}
      onCarouselPausedChange={(paused) => (carouselPaused = paused)}
      onSearch={runSearch}
      onSaveLocalFilter={handleSaveLocalFilter}
      onApplyLocalFilter={handleApplyLocalFilter}
      onDeleteFilter={handleDeleteSavedFilter}
      onDiscoverSourceChange={handleDiscoverSourceChange}
      onSearchWallhaven={searchWallhaven}
      onSearchMotionBgs={searchMotionBgs}
      onSaveWallhavenFilter={handleSaveWallhavenFilter}
      onApplyWallhavenFilter={handleApplyWallhavenFilter}
      onOpenWallhaven={openWallhavenDetail}
      onOpenMotionBgs={openMotionBgsDetail}
      onDownloadWallhaven={downloadWallhaven}
      onDownloadMotionBgs={downloadMotionBgs}
      onBrowseWallhaven={() => browseDiscoverSource("wallhaven")}
      onBrowseMotionBgs={() => browseDiscoverSource("motionbgs")}
      onRandomFavorite={handleRandomFavorite}
      onImportFiles={handleImportFiles}
      onImportFolder={handleImportFolder}
      onCheckFiles={handleCheckFiles}
      onRotationEnabledChange={handleRotationEnabledChange}
      onRotationIntervalChange={handleRotationIntervalChange}
      onCreateCollection={handleCreateCollection}
      onDeleteCollection={handleDeleteCollection}
      onCollectionMembershipChange={handleCollectionMembershipChange}
    />

    <MonitorStatusBar
      monitors={store.monitors}
      backup={store.backup}
      history={store.history}
      error={store.error}
      importStatus={store.importStatus}
      importWarnings={store.importWarnings}
      onRestore={handleRestore}
      onUndoHistory={handleUndoWallpaperHistory}
    />
  </div>

  <AppDetailModals
    {detailWallpaper}
    {wallhavenDetail}
    {motionBgsDetail}
    monitors={store.monitors}
    {applyTargetMonitorId}
    {applyingWallpaperPath}
    {favoriteBusyId}
    {thumbnailBusyId}
    {canCancelWallpaper}
    onCloseDetail={() => (detailIndex = null)}
    onCloseWallhaven={() => (wallhavenDetailIndex = null)}
    onCloseMotionBgs={() => (motionBgsDetailIndex = null)}
    onApply={handleApply}
    onApplyWallhaven={applyWallhavenDetail}
    onApplyMotionBgs={applyMotionBgsDetail}
    onCancel={handleCancelWallpaper}
    onRemove={handleRemove}
    onReveal={handleReveal}
    onRegenerateThumbnail={handleRegenerateThumbnail}
    onApplyTargetChange={(monitorId) => (applyTargetMonitorId = monitorId)}
    onFavoriteChange={handleFavoriteChange}
    onPrevDetail={() => stepDetail(-1)}
    onNextDetail={() => stepDetail(1)}
    onPrevWallhaven={() => stepWallhavenDetail(-1)}
    onNextWallhaven={() => stepWallhavenDetail(1)}
    onPrevMotionBgs={() => stepMotionBgsDetail(-1)}
    onNextMotionBgs={() => stepMotionBgsDetail(1)}
  />

  {#if showSettings}
    <SettingsPanel onClose={() => (showSettings = false)} />
  {/if}

  <ToastContainer />
</div>
