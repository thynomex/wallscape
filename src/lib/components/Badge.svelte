<script lang="ts">
  type Variant = "new" | "featured" | "exclusive";

  let {
    variant = "new",
    children,
  }: { variant?: Variant; children?: import("svelte").Snippet } = $props();
</script>

<span class="badge badge-{variant}">
  {#if children}{@render children()}{:else}{variant}{/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    padding: 4px 12px;
    border-radius: var(--radius-md);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    line-height: 1;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.18);
    animation: badge-in 320ms var(--ease-spring) both;
  }

  .badge-new {
    background: var(--accent-blue);
    color: #ffffff;
  }

  .badge-featured {
    background: rgba(255, 215, 0, 0.2);
    color: #ffd700;
  }

  .badge-exclusive {
    background: rgba(138, 43, 226, 0.2);
    color: #a855f7;
  }

  @keyframes badge-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.9);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .badge {
      animation: none !important;
    }
  }
</style>
