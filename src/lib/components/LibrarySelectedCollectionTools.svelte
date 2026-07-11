<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import type { Collection } from "$lib/types/wallpaper";

  let {
    selectedCollection,
    collectionBusyId = null,
    onDeleteCollection,
    onDeleted,
  }: {
    selectedCollection: Collection;
    collectionBusyId?: number | null;
    onDeleteCollection: (collection: Collection) => void | Promise<void>;
    onDeleted: () => void;
  } = $props();

  async function deleteSelectedCollection() {
    await onDeleteCollection(selectedCollection);
    onDeleted();
  }
</script>

<div class="collection-tools collection-tools-top">
  <div class="collection-selected-meta">
    <span>{selectedCollection.name}</span>
    <strong>{selectedCollection.wallpaper_count}</strong>
  </div>
  <button
    class="collection-tool danger"
    disabled={collectionBusyId === selectedCollection.id}
    onclick={deleteSelectedCollection}
  >
    <Icon name="trash" size={16} />
    Delete
  </button>
</div>

<style>
  .collection-tools {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
  }

  .collection-tools-top {
    padding-top: 2px;
  }

  .collection-tool {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 38px;
    padding: 0 13px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    background: var(--glass-control);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .collection-tool:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .collection-tool.danger:hover:not(:disabled) {
    color: #ffb4ab;
  }

  .collection-tool:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .collection-selected-meta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
    padding: 0 12px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: rgba(0, 0, 0, 0.18);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 700;
  }

  .collection-selected-meta span {
    max-width: min(280px, 48vw);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
  }

  .collection-selected-meta strong {
    min-width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.12);
    color: var(--text-primary);
    font-size: 11px;
  }

  @media (max-width: 720px) {
    .collection-tools {
      align-items: stretch;
      width: 100%;
    }

    .collection-tool,
    .collection-selected-meta {
      width: 100%;
    }

    .collection-selected-meta span {
      max-width: none;
    }
  }
</style>
