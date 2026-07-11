<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";

  let {
    importing = false,
    loading = false,
    dropActive = false,
    onImportFiles,
    onImportFolder,
    onCheckFiles,
  }: {
    importing?: boolean;
    loading?: boolean;
    dropActive?: boolean;
    onImportFiles: () => void | Promise<void>;
    onImportFolder: () => void | Promise<void>;
    onCheckFiles: () => void | Promise<void>;
  } = $props();
</script>

<div class="import-panel" class:drop-active={dropActive}>
  <div>
    <h3>Import</h3>
    <p>Images, videos, and folders from disk.</p>
  </div>

  <div class="import-actions">
    <button disabled={importing} onclick={onImportFiles}>
      <Icon name="upload" size={16} />
      Files
    </button>
    <button disabled={importing} onclick={onImportFolder}>
      <Icon name="folder" size={16} />
      Folder
    </button>
    <button disabled={loading || importing} onclick={onCheckFiles}>
      <Icon name="alert-triangle" size={16} />
      Check Files
    </button>
  </div>
</div>

<style>
  .import-panel {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    margin: 4px 0 22px;
    padding: 16px 18px;
    border: 1px dashed rgba(255, 255, 255, 0.22);
    border-radius: var(--radius-md);
    background: rgba(0, 0, 0, 0.16);
    overflow: hidden;
    transition:
      border-color var(--motion-fast) var(--ease-standard),
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard);
  }

  .import-panel::before {
    content: "";
    position: absolute;
    inset: -1px;
    pointer-events: none;
    background: linear-gradient(
      110deg,
      transparent 0%,
      transparent 34%,
      rgba(255, 255, 255, 0.16) 50%,
      transparent 66%,
      transparent 100%
    );
    opacity: 0;
    transform: translateX(-115%);
  }

  .import-panel.drop-active {
    border-color: rgba(255, 255, 255, 0.72);
    background: rgba(255, 255, 255, 0.12);
    transform: translateY(-1px);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.16),
      0 18px 36px rgba(0, 0, 0, 0.2);
  }

  .import-panel.drop-active::before {
    opacity: 1;
    animation: drop-sweep 1150ms var(--ease-emphasized) infinite;
  }

  .import-panel h3 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 750;
    color: var(--text-primary);
  }

  .import-panel p {
    margin: 0;
    max-width: 560px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .import-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .import-actions button {
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

  .import-actions button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    transform: translateY(-1px);
  }

  .import-actions button:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  .import-actions button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @keyframes drop-sweep {
    from {
      transform: translateX(-115%);
    }
    to {
      transform: translateX(115%);
    }
  }

  @media (max-width: 720px) {
    .import-panel {
      width: 100%;
      align-items: stretch;
      flex-direction: column;
    }

    .import-actions {
      justify-content: stretch;
    }

    .import-actions button {
      flex: 1 1 120px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .import-panel.drop-active::before {
      animation: none !important;
    }
  }
</style>
