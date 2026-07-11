<script lang="ts">
  import type {
    Monitor,
    WallpaperBackupStatus,
    WallpaperHistoryEntry,
  } from "$lib/types/wallpaper";
  import Icon from "./Icon.svelte";

  let {
    monitors,
    backup,
    history = [],
    error,
    importStatus,
    importWarnings,
    onRestore,
    onUndoHistory,
  }: {
    monitors: Monitor[];
    backup: WallpaperBackupStatus;
    history?: WallpaperHistoryEntry[];
    error: string | null;
    importStatus: string | null;
    importWarnings: string[];
    onRestore: () => void;
    onUndoHistory?: () => void;
  } = $props();

  let recentHistory = $derived(history.slice(0, 5));
  let canUndoHistory = $derived(history.length > 1);

  function sourceLabel(source: string) {
    switch (source) {
      case "rotation":
        return "Rotation";
      case "history_undo":
        return "Undo";
      case "restore_original":
        return "Original";
      case "restore_previous":
        return "Previous";
      default:
        return "Manual";
    }
  }

  function formatAppliedAt(value: number) {
    const date = new Date(value * 1000);
    if (Number.isNaN(date.getTime())) return "";

    return date.toLocaleString([], {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }
</script>

{#if monitors.length || backup.can_restore || history.length || error || importStatus || importWarnings.length}
  <footer class="monitor-bar">
    {#if monitors.length}
      <Icon name="monitor" size={15} />
      <span>
        {monitors.length} display{monitors.length > 1 ? "s" : ""} ·
        {monitors.map((monitor) => `${monitor.width}×${monitor.height}`).join(", ")}
      </span>
    {/if}

    {#if error}
      <span class="footer-error">{error}</span>
    {/if}

    {#if importStatus}
      <span class="footer-status">{importStatus}</span>
    {/if}

    {#if importWarnings.length}
      <span class="footer-warning">{importWarnings.join(" ")}</span>
    {/if}

    <div class="footer-actions">
      {#if history.length}
        <details class="history-menu">
          <summary>
            <Icon name="history" size={14} />
            History
          </summary>
          <div class="history-popover">
            {#each recentHistory as item (item.id)}
              <div class="history-item">
                <span class="history-title">{item.title}</span>
                <span class="history-meta">
                  {sourceLabel(item.apply_source)}
                  {#if item.target_monitor_id}
                    · Display
                  {/if}
                  {#if formatAppliedAt(item.applied_at)}
                    · {formatAppliedAt(item.applied_at)}
                  {/if}
                </span>
              </div>
            {/each}
          </div>
        </details>
      {/if}

      {#if canUndoHistory && onUndoHistory}
        <button class="restore-btn" onclick={onUndoHistory}>
          <Icon name="rotate-ccw" size={14} />
          Undo wallpaper
        </button>
      {/if}

      {#if backup.can_restore}
        <button class="restore-btn" onclick={onRestore}>
          <Icon name="rotate-ccw" size={14} />
          Restore original
        </button>
      {/if}
    </div>
  </footer>
{/if}
