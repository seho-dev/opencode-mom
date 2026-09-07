<script lang="ts">
  let { total, page = $bindable(1), pageSize = 5 }: { total: number; page?: number; pageSize?: number } = $props();
  const pages = $derived(Math.max(1, Math.ceil(total / pageSize)));
</script>

<nav class="pagination" aria-label="Pagination">
  <span class="pagination-info">Page {Math.min(page, pages)} of {pages} · {total} {total === 1 ? 'row' : 'rows'}</span>
  <div class="pagination-controls">
    <button
      type="button"
      class="pagination-btn"
      aria-label="Previous page"
      onclick={() => (page = Math.max(1, page - 1))}
      disabled={page <= 1}>←</button
    >
    <button
      type="button"
      class="pagination-btn"
      aria-label="Next page"
      onclick={() => (page = Math.min(pages, page + 1))}
      disabled={page >= pages}>→</button
    >
  </div>
</nav>
