<script lang="ts">
  import { Search } from '@lucide/svelte';
  import type { Snippet } from 'svelte';

  let {
    query = $bindable(''),
    selected = $bindable('all'),
    searchLabel = 'Search by name',
    filterLabel = 'Filter by type',
    children,
  }: { query?: string; selected?: string; searchLabel?: string; filterLabel?: string; children: Snippet } = $props();
</script>

<div class="search-toolbar">
  <div class="search-field">
    <Search size={14} aria-hidden="true" />
    <input aria-label={searchLabel} placeholder="Search by name" bind:value={query} />
  </div>
  <select aria-label={filterLabel} bind:value={selected}>{@render children()}</select>
</div>
