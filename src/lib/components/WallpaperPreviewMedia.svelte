<script lang="ts">
  import type { Wallpaper } from "$lib/types/wallpaper";
  import {
    isGifSrc,
    resolveMediaSrc,
    resolveWallpaperPreviewSrc,
    resolveWallpaperThumbnailSrc,
  } from "$lib/utils/media";
  import { appVisibility } from "$lib/stores/appVisibility.svelte";
  import Icon from "./Icon.svelte";

  let {
    wallpaper,
    mode = "thumbnail",
    showVideo = true,
    imageClass,
    videoClass,
    fallbackClass,
    posterClass = imageClass,
    alt = wallpaper.title,
    loading,
    fallbackIconName,
    fallbackIconSize = 40,
  }: {
    wallpaper: Wallpaper;
    mode?: "thumbnail" | "detail";
    showVideo?: boolean;
    imageClass: string;
    videoClass: string;
    fallbackClass: string;
    posterClass?: string;
    alt?: string;
    loading?: "lazy" | "eager";
    fallbackIconName?: "play";
    fallbackIconSize?: number;
  } = $props();

  let thumbnailSrc = $derived(resolveWallpaperThumbnailSrc(wallpaper));
  let imageSrc = $derived(
    mode === "detail"
      ? resolveWallpaperPreviewSrc(wallpaper)
      : thumbnailSrc,
  );
  let videoSrc = $derived(
    wallpaper.media_type === "video" && wallpaper.file_path && !isGifSrc(wallpaper.file_path)
      ? resolveMediaSrc(wallpaper.file_path)
      : null,
  );
  let effectiveShowVideo = $derived(showVideo && appVisibility.visible);
  let videoReady = $state(false);

  $effect(() => {
    videoSrc;
    effectiveShowVideo;
    videoReady = false;
  });

  function preventMediaDrag(event: DragEvent) {
    event.preventDefault();
  }
</script>

{#if imageSrc}
  <img
    class={effectiveShowVideo && videoSrc ? posterClass : imageClass}
    src={imageSrc}
    {alt}
    {loading}
    draggable={false}
    ondragstart={preventMediaDrag}
  />
{:else}
  <div class={fallbackClass}>
    {#if fallbackIconName}
      <Icon name={fallbackIconName} size={fallbackIconSize} />
    {/if}
  </div>
{/if}

{#if effectiveShowVideo && videoSrc}
  <video
    class={videoClass}
    class:ready={videoReady}
    src={videoSrc}
    poster={imageSrc ?? undefined}
    muted
    loop
    playsinline
    preload="metadata"
    autoplay
    aria-hidden="true"
    draggable={false}
    ondragstart={preventMediaDrag}
    onloadeddata={() => (videoReady = true)}
  ></video>
{/if}
