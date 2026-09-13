<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import type { AgentStorage } from '$lib/features/config/types.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  const config = getConfig();
  const i18n = getI18n();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let storage = $state<AgentStorage>('inline');
  const pageSize = 5;
  let page = $state(1);
  const agents = $derived(
    config.agents.filter((agent) =>
      `${agent.id} ${agent.description ?? ''}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
  const maxPage = $derived(Math.max(1, Math.ceil(agents.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedAgents = $derived(agents.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the filtered row count changes (search or delete).
    void agents.length;
    page = 1;
  });
  const canSelect = $derived(!!deleting && config.agents.find((agent) => agent.id === deleting)?.source === 'both');
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteAgent(deleting, canSelect ? storage : undefined);
      deleting = null;
    } catch {
      /* ConfigFeedback renders the command error. */
    }
  }
</script>

<svelte:head><title>{i18n.t('agents.metaTitle')}</title></svelte:head><PageHead
  eyebrow={i18n.t('agents.eyebrow')}
  title={i18n.t('agents.title')}
  >{#snippet children()}<Button href="/agents/new"><Plus size={14} /> {i18n.t('agents.new')}</Button
    >{/snippet}</PageHead
>
<div class="search-toolbar">
  <input
    aria-label={i18n.t('agents.searchLabel')}
    bind:value={query}
    placeholder={i18n.t('agents.searchPlaceholder')}
  />
</div>
<DataTable label={i18n.t('agents.tableLabel')} total={agents.length} bind:page {pageSize}
  ><thead
    ><tr
      ><th>{i18n.t('agents.colId')}</th><th>{i18n.t('agents.colMode')}</th><th>{i18n.t('agents.colSource')}</th><th
        >{i18n.t('agents.colModel')}</th
      ><th class="th-actions">{i18n.t('agents.colActions')}</th></tr
    ></thead
  ><tbody
    >{#if config.loading}<tr><td colspan="5" class="empty-table-row">{i18n.t('agents.loading')}</td></tr
      >{:else if !agents.length}<EmptyTableRow
        colspan={5}
        message={i18n.t('empty.agents')}
      />{:else}{#each pagedAgents as agent}<tr
          ><td class="model-name">{agent.id}</td><td>{agent.mode ?? i18n.t('common.unset')}</td><td>{agent.source}</td
          ><td>{agent.modelRef ?? i18n.t('common.inherited')}</td><td class="row-actions"
            ><Button
              href={`/agents/${encodeURIComponent(agent.id)}/edit`}
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('agents.editAria', { id: agent.id })}><Pencil size={14} /></Button
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('agents.deleteAria', { id: agent.id })}
              onclick={() => (deleting = agent.id)}><Trash2 size={14} /></Button
            ></td
          ></tr
        >{/each}{/if}</tbody
  ></DataTable
>
<Dialog.Root
  open={deleting !== null}
  onOpenChange={(open) => {
    if (!open) deleting = null;
  }}
>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>{i18n.t('agents.deleteTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('agents.deleteConfirm', { name: deleting ?? '' })}</Dialog.Description>
    </Dialog.Header>
    {#if canSelect}<div class="dialog-field">
        <label for="source-to-delete">{i18n.t('agents.sourceToDelete')}</label><select
          id="source-to-delete"
          class="dialog-select"
          bind:value={storage}
          ><option value="inline">{i18n.t('agents.sourceInline')}</option><option value="global_markdown"
            >{i18n.t('agents.sourceGlobalMarkdown')}</option
          ><option value="project_markdown">{i18n.t('agents.sourceProjectMarkdown')}</option></select
        >
        <p class="text-xs text-muted-foreground">{i18n.t('agents.sourceDeleteHint')}</p>
      </div>{/if}
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="destructive" onclick={remove}>{i18n.t('common.delete')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
