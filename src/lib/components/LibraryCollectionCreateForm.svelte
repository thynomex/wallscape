<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import type { Collection } from "$lib/types/wallpaper";

  let {
    disabled = false,
    onCreateCollection,
    onCollectionCreated,
  }: {
    disabled?: boolean;
    onCreateCollection: (
      name: string,
    ) => Collection | null | void | Promise<Collection | null | void>;
    onCollectionCreated: (collection: Collection) => void;
  } = $props();

  let newCollectionName = $state("");

  async function createCollection(event: SubmitEvent) {
    event.preventDefault();
    const name = newCollectionName.trim();
    if (!name) return;

    const created = await onCreateCollection(name);
    newCollectionName = "";

    if (created) {
      onCollectionCreated(created);
    }
  }
</script>

<form class="collection-create" onsubmit={createCollection}>
  <input
    bind:value={newCollectionName}
    maxlength="64"
    placeholder="Collection name"
    disabled={disabled}
  />
  <button type="submit" disabled={disabled || !newCollectionName.trim()}>
    <Icon name="plus" size={16} />
    Create
  </button>
</form>

<style>
  .collection-create {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .collection-create input {
    min-width: 0;
    width: min(230px, 44vw);
    height: 38px;
    padding: 0 13px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    background: rgba(18, 25, 31, 0.86);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
  }

  .collection-create input:focus {
    outline: none;
    border-color: var(--accent-blue);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.16);
  }

  .collection-create button {
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

  .collection-create button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .collection-create button:disabled,
  .collection-create input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @media (max-width: 720px) {
    .collection-create {
      align-items: stretch;
      width: 100%;
    }

    .collection-create input,
    .collection-create button {
      width: 100%;
    }
  }
</style>
