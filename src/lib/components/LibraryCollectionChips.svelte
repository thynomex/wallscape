<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import type { Collection } from "$lib/types/wallpaper";

  let {
    collections = [],
    selectedCollectionId = null,
    onSelectCollection,
  }: {
    collections?: Collection[];
    selectedCollectionId?: number | null;
    onSelectCollection: (collectionId: number) => void;
  } = $props();
</script>

{#if collections.length}
  <div class="collection-chip-row" aria-label="Collections">
    {#each collections as collection (collection.id)}
      <button
        class="collection-chip"
        class:active={selectedCollectionId === collection.id}
        aria-pressed={selectedCollectionId === collection.id}
        onclick={() => onSelectCollection(collection.id)}
      >
        <Icon name="folder" size={15} />
        {collection.name}
        <span>{collection.wallpaper_count}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .collection-chip-row {
    display: flex;
    align-items: stretch;
    gap: 8px;
    flex-wrap: wrap;
  }

  .collection-chip {
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

  .collection-chip span {
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.12);
    font-size: 11px;
  }

  .collection-chip:hover {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .collection-chip.active {
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
    animation: segment-pop 260ms var(--ease-spring) both;
  }

  .collection-chip.active span {
    background: rgba(0, 0, 0, 0.1);
  }

  @keyframes segment-pop {
    0% {
      transform: scale(0.96);
    }
    100% {
      transform: scale(1);
    }
  }

  @media (max-width: 720px) {
    .collection-chip-row {
      align-items: stretch;
      width: 100%;
    }

    .collection-chip {
      width: 100%;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .collection-chip.active {
      animation: none !important;
    }
  }
</style>
