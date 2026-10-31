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
    applying = false,
    canApplyHistory = () => false,
    onRestore,
    onUndoHistory,
    onApplyHistory,
    onDismissError,
  }: {
    monitors: Monitor[];
    backup: WallpaperBackupStatus;
    history?: WallpaperHistoryEntry[];
    error: string | null;
    importStatus: string | null;
    importWarnings: string[];
    /** A wallpaper is being applied; history entries wait for it. */
    applying?: boolean;
    /** The entry's wallpaper is still in the library. */
    canApplyHistory?: (entry: WallpaperHistoryEntry) => boolean;
    onRestore: () => void;
    onUndoHistory?: () => void;
    onApplyHistory?: (entry: WallpaperHistoryEntry) => void;
    onDismissError?: () => void;
  } = $props();

  // Reads the element's own `open` state: the native toggle event that would
  // update a mirrored copy fires asynchronously.
  let historyMenuEl = $state<HTMLDetailsElement | null>(null);
  let recentHistory = $derived(history.slice(0, 5));
  let canUndoHistory = $derived(history.length > 1);
  let warningText = $derived(importWarnings.join(" "));

  function closeHistory(returnFocus = false) {
    if (!historyMenuEl?.open) return;
    historyMenuEl.open = false;
    if (returnFocus) historyMenuEl.querySelector("summary")?.focus();
  }

  function handleWindowClick(event: MouseEvent) {
    if (historyMenuEl?.open && !historyMenuEl.contains(event.target as Node)) closeHistory();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && historyMenuEl?.open) {
      event.preventDefault();
      closeHistory(true);
    }
  }

  function applyHistoryEntry(entry: WallpaperHistoryEntry) {
    closeHistory(true);
    onApplyHistory?.(entry);
  }

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

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

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
      <span class="footer-message footer-error" title={error}>{error}</span>
      {#if onDismissError}
        <button
          type="button"
          class="footer-dismiss"
          aria-label="Dismiss error"
          onclick={onDismissError}
        >
          <Icon name="x" size={13} />
        </button>
      {/if}
    {/if}

    {#if importStatus}
      <span class="footer-message footer-status" title={importStatus}>{importStatus}</span>
    {/if}

    {#if importWarnings.length}
      <span class="footer-message footer-warning" title={warningText}>{warningText}</span>
    {/if}

    <div class="footer-actions">
      {#if history.length}
        <details class="history-menu" bind:this={historyMenuEl}>
          <summary>
            <Icon name="history" size={14} />
            History
          </summary>
          <div class="history-popover">
            {#each recentHistory as item (item.id)}
              {@const available = canApplyHistory(item)}
              <button
                type="button"
                class="history-item"
                disabled={!available || applying}
                onclick={() => applyHistoryEntry(item)}
              >
                <span class="history-title">{item.title}</span>
                <span class="history-meta">
                  {sourceLabel(item.apply_source)}
                  {#if item.target_monitor_id}
                    · Display
                  {/if}
                  {#if formatAppliedAt(item.applied_at)}
                    · {formatAppliedAt(item.applied_at)}
                  {/if}
                  {#if !available}
                    · Not in library
                  {/if}
                </span>
              </button>
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

<style>
  .monitor-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-12);
    font-size: 13px;
    color: var(--text-tertiary);
    border-top: 1px solid var(--glass-border);
    flex-wrap: wrap;
    row-gap: 6px;
  }

  /* min-width: 0 lets a long message shrink to an ellipsis instead of being
     cut off by the window edge; the title shows the full text. */
  .footer-message {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .footer-error {
    color: #ffb4ab;
  }

  .footer-status {
    color: var(--text-secondary);
  }

  .footer-warning {
    color: #ffd38a;
  }

  .footer-dismiss {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    color: #ffb4ab;
    background: transparent;
    transition: background var(--motion-fast) var(--ease-standard);
  }

  .footer-dismiss:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .footer-actions {
    margin-left: auto;
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .restore-btn,
  .history-menu summary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    height: 32px;
    padding: 0 var(--space-4);
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: var(--glass-control);
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard);
  }

  .history-menu {
    position: relative;
  }

  .history-menu summary {
    list-style: none;
    cursor: pointer;
  }

  .history-menu summary::-webkit-details-marker {
    display: none;
  }

  .restore-btn:hover,
  .history-menu summary:hover {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .restore-btn:active,
  .history-menu summary:active {
    transform: translateY(0) scale(0.98);
  }

  .history-popover {
    position: absolute;
    right: 0;
    bottom: calc(100% + 8px);
    z-index: 40;
    width: min(320px, calc(100vw - 32px));
    display: grid;
    gap: 6px;
    padding: var(--space-3);
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
    background: rgba(14, 18, 24, 0.96);
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.34), inset 0 1px 0 var(--glass-highlight);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .history-item {
    width: 100%;
    min-width: 0;
    display: grid;
    gap: 2px;
    padding: 8px 9px;
    border: none;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.06);
    font: inherit;
    font-size: 13px;
    text-align: left;
    transition: background var(--motion-fast) var(--ease-standard);
  }

  .history-item:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .history-item:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .history-title {
    overflow: hidden;
    color: var(--text-primary);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-meta {
    overflow: hidden;
    color: var(--text-tertiary);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
