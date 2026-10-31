<script lang="ts">
  import Icon from "./Icon.svelte";
  import { reveal } from "$lib/actions/reveal";

  let {
    activeView = "home",
    importing = false,
    onNavigate,
    onImport,
    onSettings,
  }: {
    activeView?: string;
    importing?: boolean;
    onNavigate?: (view: string) => void;
    onImport?: () => void;
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
  <div class="nav-leading" data-tauri-drag-region>
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
        aria-current={activeView === pill.id ? "page" : undefined}
        onclick={() => onNavigate?.(pill.id)}
      >
        {pill.label}
      </button>
    {/each}
  </div>

  <div class="nav-actions" data-tauri-drag-region>
    <!-- Not disabled while importing: a disabled button would drop keyboard
         focus. The page ignores repeat imports while one runs. -->
    <button
      class="btn-settings btn-import"
      aria-label={importing ? "Importing wallpapers" : "Import wallpapers"}
      aria-busy={importing}
      title="Import files from this PC"
      onclick={() => onImport?.()}
    >
      <span class="btn-label">{importing ? "Importing" : "Import"}</span>
      <Icon
        name={importing ? "refresh" : "upload"}
        size={19}
        strokeWidth={1.9}
        spin={importing}
      />
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
    /* Equal side tracks keep the pills centered whatever the logo and
       action group widths are. */
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: var(--space-4);
    /* The logo starts on the shared page edge, level with the hero copy. */
    padding: 22px var(--page-inset) 18px;
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
    min-width: 0;
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
    min-width: 0;
    justify-content: flex-end;
    /* Clear the window controls in the shell's top-right corner (about 92px
       from its edge). The nav's own padding is at least --page-gutter. */
    margin-right: max(0px, calc(104px - var(--page-gutter)));
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
