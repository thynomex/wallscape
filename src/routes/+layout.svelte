<script lang="ts">
  import "$lib/styles/global.css";

  let { children }: { children: import("svelte").Snippet } = $props();
</script>

{@render children()}
