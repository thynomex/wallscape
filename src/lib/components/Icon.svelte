<script lang="ts">
  // Outline / stroke icon set used across the app.
  // Stroke width 2px and currentColor by default.
  type IconName =
    | "sparkle"
    | "upload"
    | "settings"
    | "heart"
    | "heart-filled"
    | "external"
    | "play"
    | "pause"
    | "search"
    | "filter"
    | "download"
    | "folder"
    | "refresh"
    | "shuffle"
    | "trash"
    | "rotate-ccw"
    | "chevron-left"
    | "chevron-right"
    | "close"
    | "plus"
    | "monitor"
    | "check"
    | "x"
    | "alert-triangle"
    | "info"
    | "history"
    | "gauge"
    | "maximize"
    | "maximize-2"
    | "move";

  let {
    name,
    size = 20,
    strokeWidth = 2,
  }: { name: IconName; size?: number; strokeWidth?: number } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "sparkle"}
    <path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3z" />
    <path d="M19 3v4M21 5h-4" />
  {:else if name === "upload"}
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <path d="M17 8l-5-5-5 5" />
    <path d="M12 3v12" />
  {:else if name === "settings"}
    <circle cx="12" cy="12" r="3" />
    <path
      d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
    />
  {:else if name === "heart"}
    <path
      d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78L12 21l8.84-8.61a5.5 5.5 0 0 0 0-7.78z"
    />
  {:else if name === "heart-filled"}
    <path
      fill="currentColor"
      stroke="none"
      d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78L12 21l8.84-8.61a5.5 5.5 0 0 0 0-7.78z"
    />
  {:else if name === "external"}
    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    <path d="M15 3h6v6" />
    <path d="M10 14L21 3" />
  {:else if name === "play"}
    <path fill="currentColor" stroke="none" d="M8 5v14l11-7z" />
  {:else if name === "pause"}
    <rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor" stroke="none" />
    <rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor" stroke="none" />
  {:else if name === "search"}
    <circle cx="11" cy="11" r="8" />
    <path d="M21 21l-4.35-4.35" />
  {:else if name === "filter"}
    <path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z" />
  {:else if name === "download"}
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <path d="M7 10l5 5 5-5" />
    <path d="M12 15V3" />
  {:else if name === "folder"}
    <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  {:else if name === "refresh"}
    <path d="M21 12a9 9 0 0 1-15.3 6.4" />
    <path d="M3 12A9 9 0 0 1 18.3 5.6" />
    <path d="M18 2v4h-4" />
    <path d="M6 22v-4h4" />
  {:else if name === "shuffle"}
    <path d="M16 3h5v5" />
    <path d="M4 20L21 3" />
    <path d="M21 16v5h-5" />
    <path d="M15 15l6 6" />
    <path d="M4 4l5 5" />
  {:else if name === "trash"}
    <path d="M3 6h18" />
    <path d="M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2" />
    <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
    <path d="M10 11v6M14 11v6" />
  {:else if name === "rotate-ccw"}
    <path d="M3 12a9 9 0 1 0 3-6.7" />
    <path d="M3 3v6h6" />
  {:else if name === "chevron-left"}
    <path d="M15 18l-6-6 6-6" />
  {:else if name === "chevron-right"}
    <path d="M9 18l6-6-6-6" />
  {:else if name === "close"}
    <path d="M18 6 6 18M6 6l12 12" />
  {:else if name === "plus"}
    <path d="M12 5v14M5 12h14" />
  {:else if name === "monitor"}
    <rect x="2" y="3" width="20" height="14" rx="2" />
    <path d="M8 21h8M12 17v4" />
  {:else if name === "check"}
    <path d="M20 6L9 17l-5-5" />
  {:else if name === "x"}
    <path d="M18 6L6 18M6 6l12 12" />
  {:else if name === "alert-triangle"}
    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
    <path d="M12 9v4M12 17h.01" />
  {:else if name === "info"}
    <circle cx="12" cy="12" r="10" />
    <path d="M12 16v-4M12 8h.01" />
  {:else if name === "history"}
    <path d="M3 12a9 9 0 1 0 3-6.7" />
    <path d="M3 3v6h6" />
    <path d="M12 7v5l3 2" />
  {:else if name === "gauge"}
    <path d="M12 2v4M6 6l2 2M2 12h4M6 18l2-2M12 18v4M18 18l-2-2M22 12h-4M18 6l-2 2" />
    <circle cx="12" cy="12" r="3" />
  {:else if name === "maximize"}
    <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
  {:else if name === "maximize-2"}
    <path d="M15 3h6v6M14 10l6.1-6.1M9 21H3v-6M10 14l-6.1 6.1" />
  {:else if name === "move"}
    <path d="M12 2v20M2 12h20M5 9l-3 3 3 3M19 9l3 3-3 3M9 5l3-3 3 3M9 19l3 3 3-3" />
  {/if}
</svg>

<style>
  .icon {
    display: block;
    flex-shrink: 0;
  }
</style>
