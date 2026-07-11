<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";

  let { hint }: { hint: string } = $props();
</script>

<div class="drop-overlay" aria-live="polite">
  <div class="drop-target">
    <div class="drop-icon">
      <Icon name="upload" size={28} strokeWidth={1.9} />
    </div>
    <div>
      <div class="drop-title">{hint}</div>
      <div class="drop-subtitle">Images, videos, and folders</div>
    </div>
  </div>
</div>
