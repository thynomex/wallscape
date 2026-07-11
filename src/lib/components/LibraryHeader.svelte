<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";

  type HeaderFilter = "all" | "favorites";

  let {
    activeFilter,
    wallpaperCount,
    favoriteCount,
    rotationEnabled = false,
    rotationIntervalMinutes = 30,
    rotationBusy = false,
    onFilterChange,
    onRandomFavorite,
    onRotationEnabledChange,
    onRotationIntervalChange,
  }: {
    activeFilter: string;
    wallpaperCount: number;
    favoriteCount: number;
    rotationEnabled?: boolean;
    rotationIntervalMinutes?: number;
    rotationBusy?: boolean;
    onFilterChange: (filter: HeaderFilter) => void;
    onRandomFavorite: () => void | Promise<void>;
    onRotationEnabledChange: (enabled: boolean) => void | Promise<void>;
    onRotationIntervalChange: (minutes: number) => void | Promise<void>;
  } = $props();

  const baseIntervalOptions = [5, 15, 30, 60, 120];

  let intervalOptions = $derived(
    baseIntervalOptions.includes(rotationIntervalMinutes)
      ? baseIntervalOptions
      : [...baseIntervalOptions, rotationIntervalMinutes].sort((a, b) => a - b),
  );
  let rotationDisabled = $derived(favoriteCount === 0 || rotationBusy);
</script>

<div class="section-header library-header">
  <div>
    <h2 class="section-title">Library</h2>
    <p class="section-description">Browse saved wallpapers and rotate through favorites.</p>
  </div>

  <div class="library-actions" aria-label="Library filters">
    <div class="segmented">
      <button
        class:active={activeFilter === "all"}
        aria-pressed={activeFilter === "all"}
        onclick={() => onFilterChange("all")}
      >
        All
        <span>{wallpaperCount}</span>
      </button>
      <button
        class:active={activeFilter === "favorites"}
        aria-pressed={activeFilter === "favorites"}
        onclick={() => onFilterChange("favorites")}
      >
        Favorites
        <span>{favoriteCount}</span>
      </button>
    </div>

    <button class="shuffle" disabled={rotationDisabled} onclick={onRandomFavorite}>
      <Icon name="shuffle" size={16} />
      Shuffle Favorite
    </button>

    <div class="rotation-controls">
      <button
        class="rotation-toggle"
        class:active={rotationEnabled}
        disabled={rotationDisabled}
        aria-pressed={rotationEnabled}
        onclick={() => onRotationEnabledChange(!rotationEnabled)}
      >
        <Icon name="rotate-ccw" size={16} />
        {rotationEnabled ? "Stop Rotation" : "Start Rotation"}
      </button>

      <label class="interval-select">
        <span>Every</span>
        <select
          value={rotationIntervalMinutes}
          disabled={rotationBusy}
          onchange={(event) =>
            onRotationIntervalChange(Number((event.currentTarget as HTMLSelectElement).value))}
        >
          {#each intervalOptions as minutes (minutes)}
            <option value={minutes}>{minutes} min</option>
          {/each}
        </select>
      </label>
    </div>
  </div>
</div>

<style>
  .library-header {
    gap: 18px;
  }

  .library-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .segmented {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 4px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: rgba(0, 0, 0, 0.22);
  }

  .segmented button,
  .shuffle,
  .rotation-toggle {
    border: none;
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
    font-weight: 650;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      opacity var(--motion-fast) var(--ease-standard);
  }

  .segmented button {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 13px;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
  }

  .segmented button:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
    transform: translateY(-1px);
  }

  .segmented button.active {
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.58),
      0 8px 18px rgba(0, 0, 0, 0.18);
    animation: segment-pop 260ms var(--ease-spring) both;
  }

  .segmented span {
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.12);
    font-size: 11px;
  }

  .segmented button.active span {
    background: rgba(0, 0, 0, 0.1);
  }

  .shuffle,
  .rotation-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 42px;
    padding: 0 16px;
    border-radius: var(--radius-full);
    background: var(--glass-control);
    border: 1px solid var(--glass-border);
  }

  .shuffle:hover:not(:disabled),
  .rotation-toggle:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .segmented button:active,
  .shuffle:active:not(:disabled),
  .rotation-toggle:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  .rotation-toggle.active {
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
    animation: segment-pop 260ms var(--ease-spring) both;
  }

  .shuffle:disabled,
  .rotation-toggle:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .rotation-controls {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 4px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: rgba(0, 0, 0, 0.18);
  }

  .interval-select {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding-left: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 650;
  }

  .interval-select select {
    height: 34px;
    min-width: 92px;
    padding: 0 10px;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    background: rgba(18, 25, 31, 0.86);
    font: inherit;
    font-size: 12px;
  }

  .interval-select select:focus {
    outline: none;
    border-color: var(--accent-blue);
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
    .library-actions {
      width: 100%;
      justify-content: stretch;
    }

    .segmented,
    .shuffle,
    .rotation-controls {
      width: 100%;
    }

    .rotation-controls {
      flex-wrap: wrap;
    }

    .rotation-toggle,
    .interval-select {
      flex: 1 1 160px;
      justify-content: center;
    }

    .segmented button {
      flex: 1;
      justify-content: center;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .segmented button.active,
    .rotation-toggle.active {
      animation: none !important;
    }
  }
</style>
