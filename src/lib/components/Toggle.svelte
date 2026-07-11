<script lang="ts">
  // Accessible on/off switch matching the glassmorphic design system.
  let {
    checked = false,
    disabled = false,
    label,
    onChange,
  }: {
    checked?: boolean;
    disabled?: boolean;
    label?: string;
    onChange?: (value: boolean) => void;
  } = $props();

  function toggle() {
    if (disabled) return;
    onChange?.(!checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  class="toggle"
  class:on={checked}
  {disabled}
  onclick={toggle}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 44px;
    height: 26px;
    flex-shrink: 0;
    padding: 0;
    border-radius: var(--radius-full);
    border: 1px solid var(--glass-border);
    background: var(--glass-control);
    cursor: pointer;
    transition:
      background var(--motion-med) var(--ease-standard),
      border-color var(--motion-med) var(--ease-standard),
      transform var(--motion-fast) var(--ease-standard);
  }

  .toggle.on {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.24);
  }

  .toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    transition: transform var(--motion-med) var(--ease-spring);
  }

  .toggle.on .knob {
    transform: translateX(18px);
  }

  .toggle:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .toggle:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  @media (prefers-reduced-motion: reduce) {
    .toggle,
    .knob {
      transition: none !important;
    }
  }
</style>
