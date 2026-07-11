<script lang="ts">
  let { count = 6 }: { count?: number } = $props();
</script>

<div class="loading-grid" aria-hidden="true">
  {#each Array(count) as _, i}
    <div class="loading-card" style={`animation-delay:${i * 70}ms`}></div>
  {/each}
</div>
