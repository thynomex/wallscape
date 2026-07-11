<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { appVisibility } from "$lib/stores/appVisibility.svelte";

  // Window controls for the borderless/transparent window.
  // Lazily import the Tauri API so the component still renders (and the
  // browser preview still works) outside the Tauri runtime.
  const inTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  async function loadAppWindow() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow();
  }

  let appWindowPromise: ReturnType<typeof loadAppWindow> | null = null;

  function appWindow() {
    appWindowPromise ??= loadAppWindow();
    return appWindowPromise;
  }

  async function minimize() {
    if (!inTauri) return;
    const w = await appWindow();
    // Respect the "minimize to tray" preference: hide instead of minimizing.
    appVisibility.markHidden();
    if (settingsStore.settings.minimizeToTray) {
      await w.hide();
    } else {
      await w.minimize();
    }
  }

  async function toggleMaximize() {
    if (!inTauri) return;
    const w = await appWindow();
    await w.toggleMaximize();
  }

  async function close() {
    if (!inTauri) return;
    if (settingsStore.settings.closeToTray) {
      appVisibility.markHidden();
    }
    (await appWindow()).close();
  }
</script>

<div class="window-controls" aria-label="Window controls">
  <button type="button" class="ctl ctl-minimize" aria-label="Minimize" onclick={minimize}>
    <span class="traffic-dot" aria-hidden="true">
      <span class="traffic-symbol traffic-symbol-minimize"></span>
    </span>
  </button>

  <button
    type="button"
    class="ctl ctl-maximize"
    aria-label="Toggle maximize"
    onclick={toggleMaximize}
  >
    <span class="traffic-dot" aria-hidden="true">
      <span class="traffic-symbol traffic-symbol-maximize"></span>
    </span>
  </button>

  <button type="button" class="ctl ctl-close" aria-label="Close" onclick={close}>
    <span class="traffic-dot" aria-hidden="true">
      <span class="traffic-symbol traffic-symbol-close"></span>
    </span>
  </button>
</div>

<style>
  .window-controls {
    position: absolute;
    top: 8px;
    right: 18px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    z-index: 300;
  }

  .ctl {
    position: relative;
    width: 22px;
    height: 34px;
    border: none;
    border-radius: 0;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    transition:
      transform var(--motion-fast) var(--ease-standard);
    -webkit-app-region: no-drag;
  }

  .ctl:hover {
    transform: translateY(-1px);
  }

  .ctl:active {
    transform: scale(0.96);
  }

  .ctl:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.85);
    outline-offset: 2px;
  }

  .ctl-close {
    width: 30px;
  }

  .traffic-dot {
    position: relative;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    box-shadow:
      inset 0 -1px 2px rgba(0, 0, 0, 0.2),
      inset 0 1px 1px rgba(255, 255, 255, 0.34),
      0 0 0 1px rgba(0, 0, 0, 0.16);
    transition:
      filter var(--motion-fast) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .ctl-minimize .traffic-dot {
    background: #ffbd2e;
  }

  .ctl-maximize .traffic-dot {
    background: #28c840;
  }

  .ctl-close .traffic-dot {
    background: #ff5f57;
  }

  .ctl:hover .traffic-dot {
    filter: saturate(1.08) brightness(1.05);
    transform: scale(1.04);
  }

  .traffic-symbol {
    position: relative;
    width: 8px;
    height: 8px;
    color: rgba(57, 42, 31, 0.62);
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-standard);
  }

  .ctl:hover .traffic-symbol {
    opacity: 1;
  }

  .traffic-symbol-minimize::before,
  .traffic-symbol-close::before,
  .traffic-symbol-close::after {
    content: "";
    position: absolute;
    left: 1px;
    right: 1px;
    top: 50%;
    height: 1.5px;
    border-radius: 1px;
    background: currentColor;
    transform: translateY(-50%);
  }

  .traffic-symbol-close::before {
    transform: translateY(-50%) rotate(45deg);
  }

  .traffic-symbol-close::after {
    transform: translateY(-50%) rotate(-45deg);
  }

  .traffic-symbol-maximize {
    border-radius: 2px;
    background:
      linear-gradient(135deg, transparent 0 48%, currentColor 49% 51%, transparent 52%) no-repeat,
      linear-gradient(135deg, currentColor 0 50%, transparent 51%) bottom left / 5px 5px no-repeat,
      linear-gradient(135deg, transparent 0 49%, currentColor 50%) top right / 5px 5px no-repeat;
  }

  .ctl-maximize .traffic-symbol {
    color: rgba(11, 73, 28, 0.66);
  }

  .ctl-close .traffic-symbol {
    color: rgba(111, 29, 26, 0.66);
  }

  @media (prefers-reduced-motion: reduce) {
    .ctl,
    .traffic-dot,
    .traffic-symbol {
      transition: none !important;
    }
  }

  @media (max-width: 720px) {
    .window-controls {
      top: 8px;
      right: 18px;
    }

    .ctl {
      width: 22px;
      height: 32px;
    }

    .ctl-close {
      width: 30px;
    }
  }
</style>
