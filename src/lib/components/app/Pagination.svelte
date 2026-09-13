<script lang="ts">
  import { getI18n } from '$lib/features/i18n/context.js';

  const i18n = getI18n();
  let { total, page = $bindable(1), pageSize = 5 }: { total: number; page?: number; pageSize?: number } = $props();
  const pages = $derived(Math.max(1, Math.ceil(total / pageSize)));
</script>

<nav class="pagination" aria-label={i18n.t('common.pagination')}>
  <span class="pagination-info"
    >{i18n.t('common.pageOf', { page: Math.min(page, pages), pages })} ·
    {i18n.t(total === 1 ? 'common.rowCountOne' : 'common.rowCountMany', { count: total })}</span
  >
  <div class="pagination-controls">
    <button
      type="button"
      class="pagination-btn"
      aria-label={i18n.t('common.previousPage')}
      onclick={() => (page = Math.max(1, page - 1))}
      disabled={page <= 1}>←</button
    >
    <button
      type="button"
      class="pagination-btn"
      aria-label={i18n.t('common.nextPage')}
      onclick={() => (page = Math.min(pages, page + 1))}
      disabled={page >= pages}>→</button
    >
  </div>
</nav>
