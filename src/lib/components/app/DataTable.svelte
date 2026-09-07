<script lang="ts">
  import type { Snippet } from 'svelte';
  import Pagination from './Pagination.svelte';

  let {
    label,
    children,
    total,
    page = $bindable(1),
    pageSize = 5,
  }: {
    label: string;
    children: Snippet;
    total: number;
    page?: number;
    pageSize?: number;
  } = $props();
</script>

<section class="panel table-wrap" aria-label={label}>
  <table class="data-table">{@render children()}</table>
</section>
{#if total > pageSize}<Pagination {total} bind:page {pageSize} />{/if}
