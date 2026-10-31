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
    remoteApplyPath = null,
    remoteApplyLabel = null,
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
    /** Remote URL of the Discover preview being downloaded and applied. */
    remoteApplyPath?: string | null;
    /** Progress text for that download. */
    remoteApplyLabel?: string | null;
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

  // Remote previews carry the remote URL, never the downloaded local path, so
  // they track their download-then-apply run by that URL.
  function remoteApplying(wallpaper: Wallpaper) {
    return remoteApplyPath !== null && remoteApplyPath === wallpaper.file_path;
  }
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
    applying={remoteApplying(wallhavenDetail)}
    applyLabel={remoteApplying(wallhavenDetail) ? remoteApplyLabel : null}
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
    applying={remoteApplying(motionBgsDetail)}
    applyLabel={remoteApplying(motionBgsDetail) ? remoteApplyLabel : null}
    {monitors}
    {applyTargetMonitorId}
    {onApplyTargetChange}
    onPrev={onPrevMotionBgs}
    onNext={onNextMotionBgs}
  />
{/if}
