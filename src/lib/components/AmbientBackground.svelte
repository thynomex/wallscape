<script lang="ts">
  // Heavily blurred, darkened copy of the current wallpaper, fixed behind
  // the whole app. This is what the frosted-glass panels refract against —
  // it's what makes the glassmorphism actually read. Crossfades on change.
  let { src }: { src?: string | null } = $props();
</script>

<div class="ambient" aria-hidden="true">
  {#key src}
    {#if src}
      <img
        class="ambient-img"
        {src}
        alt=""
        draggable={false}
        ondragstart={(event) => event.preventDefault()}
      />
    {/if}
  {/key}
  <div class="ambient-veil"></div>
</div>

<style>
  .ambient {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    background: var(--bg-base);
  }

  .ambient-img {
    position: absolute;
    inset: -8%;
    width: 116%;
    height: 116%;
    object-fit: cover;
    filter: blur(60px) saturate(1.4) brightness(0.55);
    transform: scale(1.1);
    animation: ambient-in 0.8s ease both;
  }

  /* Vignette + top-light gradient for depth under the glass shell. */
  .ambient-veil {
    position: absolute;
    inset: 0;
    background: radial-gradient(
        120% 100% at 50% 0%,
        rgba(9, 12, 17, 0.25) 0%,
        rgba(9, 12, 17, 0.55) 55%,
        rgba(9, 12, 17, 0.8) 100%
      );
  }

  @keyframes ambient-in {
    from {
      opacity: 0;
      transform: scale(1.18);
    }
    to {
      opacity: 1;
      transform: scale(1.1);
    }
  }
</style>
