<script lang="ts">
  import Icon from "./Icon.svelte";
  import { reveal } from "$lib/actions/reveal";

  let {
    activeView = "home",
    onNavigate,
    onUpload,
    onSettings,
  }: {
    activeView?: string;
    onNavigate?: (view: string) => void;
    onUpload?: () => void;
    onSettings?: () => void;
  } = $props();

  const pills = [
    { id: "home", label: "Home" },
    { id: "discover", label: "Discover" },
    { id: "explore", label: "Explore" },
    { id: "library", label: "Library" },
  ];
</script>

<nav class="nav-bar" data-tauri-drag-region use:reveal={{ distance: "12px", threshold: 0.05 }}>
  <div class="nav-leading">
    <div class="logo" data-tauri-drag-region>
      <div class="logo-icon">
        <img src="/wallscape-logo.png" alt="" />
      </div>
      <span class="logo-text">Wallscape</span>
    </div>
  </div>

  <div class="nav-pills">
    {#each pills as pill (pill.id)}
      <button
        class="nav-pill"
        class:active={activeView === pill.id}
        onclick={() => onNavigate?.(pill.id)}
      >
        {pill.label}
      </button>
    {/each}
  </div>

  <div class="nav-actions">
    <button class="btn-settings" aria-label="Upload" onclick={() => onUpload?.()}>
      <span>Upload</span>
      <Icon name="upload" size={19} strokeWidth={1.9} />
    </button>
    <button class="btn-settings btn-icon-only" aria-label="Settings" onclick={() => onSettings?.()}>
      <Icon name="settings" size={20} />
    </button>
  </div>
</nav>

<style>
  .nav-bar {
    position: sticky;
    top: 0;
    min-height: 108px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 22px clamp(46px, 4vw, 76px) 18px;
    z-index: 100;
    /* Scrim so white logo/pills stay legible over bright hero imagery. */
    background: linear-gradient(
      to bottom,
      rgba(7, 11, 15, 0.62) 0%,
      rgba(7, 11, 15, 0.36) 52%,
      rgba(12, 19, 25, 0) 100%
    );
    animation: nav-drop var(--motion-slow) var(--ease-emphasized) both;
  }

  .nav-leading {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 230px;
    padding-left: 52px;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
    padding: 6px 14px 6px 6px;
    border-radius: var(--radius-full);
    background: rgba(20, 27, 34, 0.42);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 12px 24px rgba(0, 0, 0, 0.1);
  }

  .logo-icon {
    width: 32px;
    height: 32px;
    background: #08090a;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.13);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.08),
      0 10px 20px rgba(0, 0, 0, 0.18);
  }

  .logo-icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .logo-text {
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.025em;
    color: var(--text-primary);
  }

  .nav-pills {
    display: flex;
    align-items: center;
    gap: 0;
    background: rgba(34, 41, 48, 0.55);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid rgba(255, 255, 255, 0.11);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.14),
      0 16px 28px rgba(0, 0, 0, 0.12);
    padding: 5px 7px;
    border-radius: var(--radius-full);
  }

  .nav-pill {
    position: relative;
    min-width: 88px;
    height: 34px;
    padding: 0 20px;
    border-radius: var(--radius-full);
    font-size: 14px;
    font-weight: 600;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.82);
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard);
  }

  .nav-pill:not(:last-child)::after {
    content: "";
    position: absolute;
    right: -1px;
    top: 50%;
    width: 1px;
    height: 22px;
    transform: translateY(-50%);
    background: rgba(255, 255, 255, 0.12);
  }

  .nav-pill.active {
    background: rgba(255, 255, 255, 0.94);
    color: #171717;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.7),
      0 3px 10px rgba(0, 0, 0, 0.18);
    transform: translateY(-0.5px);
  }

  .nav-pill.active::after {
    opacity: 0;
  }

  .nav-pill:hover:not(.active) {
    color: var(--text-primary);
    transform: translateY(-1px);
  }

  .nav-actions {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 230px;
    justify-content: flex-end;
    padding-right: 128px;
  }

  .btn-settings {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 18px;
    height: 38px;
    background: rgba(34, 41, 48, 0.5);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 12px 24px rgba(0, 0, 0, 0.1);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 650;
    letter-spacing: -0.03em;
    transition:
      background var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard);
  }

  .btn-settings.btn-icon-only {
    width: 38px;
    padding: 0;
    border-radius: 50%;
  }

  .btn-settings:hover {
    background: var(--glass-control);
    transform: translateY(-1px);
  }

  @media (max-width: 720px) {
    .nav-bar {
      min-height: 112px;
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 10px 12px;
      padding: 36px var(--space-4) 14px;
    }
    .nav-leading,
    .nav-actions {
      min-width: auto;
    }
    .nav-leading {
      padding-left: 0;
    }
    .nav-actions {
      grid-column: 1 / -1;
      padding-right: 0;
      justify-content: flex-end;
    }
    .logo-text {
      display: none;
    }
    .nav-pills {
      padding: 5px;
      max-width: 100%;
      overflow-x: auto;
      scrollbar-width: none;
      justify-self: stretch;
      justify-content: flex-start;
    }
    .nav-pills::-webkit-scrollbar {
      display: none;
    }
    .nav-pill {
      min-width: 72px;
      height: 38px;
      padding: 0 14px;
      font-size: 14px;
    }
    .btn-settings {
      height: 40px;
      padding: 0 12px;
      font-size: 0;
    }
    .btn-settings.btn-icon-only {
      width: 40px;
    }
    .nav-actions {
      gap: 6px;
    }
  }

  @media (max-width: 440px) {
    .nav-bar {
      grid-template-columns: 1fr;
      padding-top: 36px;
    }
    .nav-leading {
      display: none;
    }
    .nav-actions {
      justify-content: stretch;
    }
    .btn-settings:not(.btn-icon-only) {
      flex: 1;
    }
  }

  @keyframes nav-drop {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
