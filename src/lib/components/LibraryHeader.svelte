<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";

  let {
    favoriteCount,
    rotationEnabled = false,
    rotationIntervalMinutes = 30,
    rotationBusy = false,
    onRandomFavorite,
    onRotationEnabledChange,
    onRotationIntervalChange,
  }: {
    favoriteCount: number;
    rotationEnabled?: boolean;
    rotationIntervalMinutes?: number;
    rotationBusy?: boolean;
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

  <div class="library-actions" role="group" aria-label="Favorite rotation">
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

  .shuffle,
  .rotation-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 42px;
    padding: 0 16px;
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

  .shuffle:hover:not(:disabled),
  .rotation-toggle:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .shuffle:active:not(:disabled),
  .rotation-toggle:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  .rotation-toggle.active {
    color: #111820;
    background: rgba(255, 255, 255, 0.92);
    animation: segment-pop 260ms var(--ease-spring) backwards;
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

  @media (prefers-reduced-motion: reduce) {
    .rotation-toggle.active {
      animation: none !important;
    }
  }
</style>
