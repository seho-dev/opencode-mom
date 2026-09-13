<script lang="ts">
  import { Search } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { getI18n } from '$lib/features/i18n/context.js';

  const i18n = getI18n();
  let {
    query = $bindable(''),
    selected = $bindable('all'),
    searchLabel = i18n.t('common.searchByName'),
    filterLabel = i18n.t('common.filterByType'),
    children,
  }: { query?: string; selected?: string; searchLabel?: string; filterLabel?: string; children: Snippet } = $props();
</script>

<div class="search-toolbar">
  <div class="search-field">
    <Search size={14} aria-hidden="true" />
    <input aria-label={searchLabel} placeholder={i18n.t('common.searchByName')} bind:value={query} />
  </div>
  <select aria-label={filterLabel} bind:value={selected}>{@render children()}</select>
</div>
