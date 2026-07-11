<script lang="ts">
  import WallpaperDetail from "$lib/components/WallpaperDetail.svelte";
  import type { Monitor, Wallpaper } from "$lib/types/wallpaper";

  let {
    detailWallpaper = null,
    wallhavenDetail = null,
    motionBgsDetail = null,
    monitors = [],
    applyTargetMonitorId = null,
    applyingWallpaperPath = null,
    favoriteBusyId = null,
    thumbnailBusyId = null,
    canCancelWallpaper,
    onCloseDetail,
    onCloseWallhaven,
    onCloseMotionBgs,
    onApply,
    onApplyWallhaven,
    onApplyMotionBgs,
    onCancel,
    onRemove,
    onReveal,
    onRegenerateThumbnail,
    onApplyTargetChange,
    onFavoriteChange,
    onPrevDetail,
    onNextDetail,
    onPrevWallhaven,
    onNextWallhaven,
    onPrevMotionBgs,
    onNextMotionBgs,
  }: {
    detailWallpaper?: Wallpaper | null;
    wallhavenDetail?: Wallpaper | null;
    motionBgsDetail?: Wallpaper | null;
    monitors?: Monitor[];
    applyTargetMonitorId?: string | null;
    applyingWallpaperPath?: string | null;
    favoriteBusyId?: number | null;
    thumbnailBusyId?: number | null;
    canCancelWallpaper: (wallpaper: Wallpaper | null) => boolean;
    onCloseDetail: () => void;
    onCloseWallhaven: () => void;
    onCloseMotionBgs: () => void;
    onApply: (wallpaper: Wallpaper, monitorId: string | null) => void | Promise<void>;
    onApplyWallhaven: (wallpaper: Wallpaper, monitorId: string | null) => void | Promise<void>;
    onApplyMotionBgs: (wallpaper: Wallpaper, monitorId: string | null) => void | Promise<void>;
    onCancel: (wallpaper: Wallpaper) => void | Promise<void>;
    onRemove: (wallpaper: Wallpaper) => void | Promise<void>;
    onReveal: (wallpaper: Wallpaper) => void | Promise<void>;
    onRegenerateThumbnail: (wallpaper: Wallpaper) => void | Promise<void>;
    onApplyTargetChange: (monitorId: string | null) => void;
    onFavoriteChange: (wallpaper: Wallpaper, isFavorite: boolean) => void | Promise<void>;
    onPrevDetail: () => void;
    onNextDetail: () => void;
    onPrevWallhaven: () => void;
    onNextWallhaven: () => void;
    onPrevMotionBgs: () => void;
    onNextMotionBgs: () => void;
  } = $props();
</script>

{#if detailWallpaper}
  <WallpaperDetail
    wallpaper={detailWallpaper}
    onClose={onCloseDetail}
    onApply={onApply}
    onCancel={onCancel}
    canCancel={canCancelWallpaper(detailWallpaper)}
    applying={applyingWallpaperPath === detailWallpaper.file_path}
    onRemove={onRemove}
    onReveal={onReveal}
    onRegenerateThumbnail={onRegenerateThumbnail}
    thumbnailBusy={thumbnailBusyId === detailWallpaper.id}
    {monitors}
    {applyTargetMonitorId}
    {onApplyTargetChange}
    onFavoriteChange={onFavoriteChange}
    favoriteBusy={favoriteBusyId === detailWallpaper.id}
    onPrev={onPrevDetail}
    onNext={onNextDetail}
  />
{/if}

{#if wallhavenDetail}
  <WallpaperDetail
    wallpaper={wallhavenDetail}
    onClose={onCloseWallhaven}
    onApply={onApplyWallhaven}
    onCancel={onCancel}
    canCancel={canCancelWallpaper(wallhavenDetail)}
    applying={applyingWallpaperPath === wallhavenDetail.file_path}
    {monitors}
    {applyTargetMonitorId}
    {onApplyTargetChange}
    onPrev={onPrevWallhaven}
    onNext={onNextWallhaven}
  />
{/if}

{#if motionBgsDetail}
  <WallpaperDetail
    wallpaper={motionBgsDetail}
    onClose={onCloseMotionBgs}
    onApply={onApplyMotionBgs}
    onCancel={onCancel}
    canCancel={canCancelWallpaper(motionBgsDetail)}
    applying={applyingWallpaperPath === motionBgsDetail.file_path}
    {monitors}
    {applyTargetMonitorId}
    {onApplyTargetChange}
    onPrev={onPrevMotionBgs}
    onNext={onNextMotionBgs}
  />
{/if}
