<script lang="ts">
  import type { SavedFilter } from "$lib/types/wallpaper";
  import Icon from "./Icon.svelte";

  let {
    filters,
    disabled = false,
    placeholder = "Filter name",
    onSave,
    onApply,
    onDelete,
  }: {
    filters: SavedFilter[];
    disabled?: boolean;
    placeholder?: string;
    onSave: (name: string) => void | Promise<void>;
    onApply: (filter: SavedFilter) => void | Promise<void>;
    onDelete: (filter: SavedFilter) => void | Promise<void>;
  } = $props();

  let selectedId = $state("");
  let name = $state("");
  let selectedFilter = $derived(
    filters.find((filter) => String(filter.id) === selectedId) ?? null,
  );

  $effect(() => {
    if (selectedId && !filters.some((filter) => String(filter.id) === selectedId)) {
      selectedId = "";
    }
  });

  async function saveFilter(event: SubmitEvent) {
    event.preventDefault();
    const nextName = name.trim();
    if (!nextName || disabled) return;

    await onSave(nextName);
    name = "";
  }
</script>

<div class="saved-filter-controls">
  <div class="saved-filter-picker">
    <Icon name="filter" size={16} />
    <select
      value={selectedId}
      disabled={disabled || filters.length === 0}
      aria-label="Saved filters"
      onchange={(event) => {
        selectedId = (event.currentTarget as HTMLSelectElement).value;
      }}
    >
      <option value="">Saved filters</option>
      {#each filters as filter (filter.id)}
        <option value={String(filter.id)}>{filter.name}</option>
      {/each}
    </select>
    <button
      type="button"
      disabled={disabled || !selectedFilter}
      aria-label="Apply saved filter"
      onclick={() => selectedFilter && onApply(selectedFilter)}
    >
      <Icon name="check" size={16} />
    </button>
    <button
      type="button"
      disabled={disabled || !selectedFilter}
      aria-label="Delete saved filter"
      onclick={() => selectedFilter && onDelete(selectedFilter)}
    >
      <Icon name="trash" size={16} />
    </button>
  </div>

  <form class="saved-filter-save" onsubmit={saveFilter}>
    <input bind:value={name} {placeholder} maxlength="64" disabled={disabled} />
    <button type="submit" disabled={disabled || !name.trim()}>
      <Icon name="plus" size={16} />
      Save
    </button>
  </form>
</div>

<style>
  .saved-filter-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .saved-filter-picker,
  .saved-filter-save {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .saved-filter-picker {
    padding: 4px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    color: var(--text-secondary);
    background: rgba(0, 0, 0, 0.18);
  }

  .saved-filter-picker select,
  .saved-filter-save input {
    height: 34px;
    min-width: 0;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    background: rgba(18, 25, 31, 0.86);
    font: inherit;
    font-size: 12px;
    font-weight: 650;
  }

  .saved-filter-picker select {
    width: min(220px, 44vw);
    padding: 0 10px;
  }

  .saved-filter-save input {
    width: min(190px, 40vw);
    padding: 0 12px;
  }

  .saved-filter-picker select:focus,
  .saved-filter-save input:focus {
    outline: none;
    border-color: var(--accent-blue);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.16);
  }

  .saved-filter-picker button,
  .saved-filter-save button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    min-width: 34px;
    height: 34px;
    padding: 0 12px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    background: var(--glass-control);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .saved-filter-picker button:hover:not(:disabled),
  .saved-filter-save button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .saved-filter-picker button:active:not(:disabled),
  .saved-filter-save button:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  .saved-filter-picker button:disabled,
  .saved-filter-save button:disabled,
  .saved-filter-picker select:disabled,
  .saved-filter-save input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @media (max-width: 720px) {
    .saved-filter-controls,
    .saved-filter-picker,
    .saved-filter-save {
      width: 100%;
    }

    .saved-filter-picker select,
    .saved-filter-save input {
      width: 100%;
      flex: 1;
    }

    .saved-filter-save button {
      flex: 0 0 auto;
    }
  }
</style>
