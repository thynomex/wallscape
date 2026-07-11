<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(""),
    placeholder,
    class: className = "",
    onSearch,
  }: {
    value: string;
    placeholder: string;
    class?: string;
    onSearch: () => void;
  } = $props();
</script>

<div class={`search-bar ${className} ${value.trim().length > 0 ? "has-value" : ""}`.trim()}>
  <span class="search-icon"><Icon name="search" size={18} /></span>
  <input
    type="text"
    {placeholder}
    bind:value
    onkeydown={(event) => event.key === "Enter" && onSearch()}
  />
  <button class="search-btn" onclick={onSearch}>Search</button>
</div>
