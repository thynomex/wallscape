<script lang="ts">
  // Use the grid class of the cards being loaded, so the placeholders get the
  // same columns and gaps and nothing jumps when the real cards arrive.
  let {
    count = 6,
    layout = "card-grid",
  }: { count?: number; layout?: "card-grid" | "discover-grid" } = $props();
</script>

<div class="loading-grid {layout}" aria-hidden="true">
  {#each Array(count) as _, i}
    <div class="loading-card" style={`animation-delay:${i * 70}ms`}></div>
  {/each}
</div>
