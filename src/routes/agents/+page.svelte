<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import Select from '$lib/components/app/Select.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { BUILTIN_AGENTS } from '$lib/features/config/builtinAgents.js';
  import { STORAGE_OPTIONS } from '$lib/features/config/constants.js';
  import type { AgentStorage } from '$lib/features/config/types.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  type AgentTab = 'custom' | 'builtin';
  const tabOrder: AgentTab[] = ['custom', 'builtin'];
  const config = getConfig();
  const i18n = getI18n();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let storage = $state<AgentStorage>('inline');
  let activeTab = $state<AgentTab>('custom');
  let customTabEl = $state<HTMLButtonElement | null>(null);
  let builtinTabEl = $state<HTMLButtonElement | null>(null);
  const pageSize = 5;
  let page = $state(1);
  const agents = $derived(
    config.agents.filter((agent) =>
      `${agent.id} ${agent.description ?? ''}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
  const slimAgents = $derived(BUILTIN_AGENTS.filter((agent) => agent.system === 'slim'));
  const omoAgents = $derived(BUILTIN_AGENTS.filter((agent) => agent.system === 'oh-my-openagent'));
  const maxPage = $derived(Math.max(1, Math.ceil(agents.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedAgents = $derived(agents.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the filtered row count changes (search or delete).
    void agents.length;
    page = 1;
  });
  function selectTab(tab: AgentTab) {
    activeTab = tab;
    query = '';
    page = 1;
  }
  function onTabKeys(event: KeyboardEvent) {
    const index = tabOrder.indexOf(activeTab);
    let next: AgentTab;
    if (event.key === 'ArrowRight') next = tabOrder[(index + 1) % tabOrder.length]!;
    else if (event.key === 'ArrowLeft') next = tabOrder[(index - 1 + tabOrder.length) % tabOrder.length]!;
    else if (event.key === 'Home') next = tabOrder[0]!;
    else if (event.key === 'End') next = tabOrder[tabOrder.length - 1]!;
    else return;
    event.preventDefault();
    selectTab(next);
    (next === 'custom' ? customTabEl : builtinTabEl)?.focus();
  }
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

<svelte:head><title>{i18n.t('agents.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('agents.eyebrow')} title={i18n.t('agents.title')}
  >{#snippet children()}{#if activeTab === 'custom'}<Button href="/agents/new"
        ><Plus size={14} /> {i18n.t('agents.new')}</Button
      >{/if}{/snippet}</PageHead
>
<div class="flex gap-2 mb-3" role="tablist" aria-label={i18n.t('agents.sourceTabLabel')}>
  <Button
    bind:ref={customTabEl}
    id="agents-tab-custom"
    role="tab"
    variant={activeTab === 'custom' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'custom'}
    aria-controls="agents-tabpanel-custom"
    tabindex={activeTab === 'custom' ? 0 : -1}
    onclick={() => selectTab('custom')}
    onkeydown={onTabKeys}>{i18n.t('agents.tabCustom', { count: config.agents.length })}</Button
  >
  <Button
    bind:ref={builtinTabEl}
    id="agents-tab-builtin"
    role="tab"
    variant={activeTab === 'builtin' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'builtin'}
    aria-controls="agents-tabpanel-builtin"
    tabindex={activeTab === 'builtin' ? 0 : -1}
    onclick={() => selectTab('builtin')}
    onkeydown={onTabKeys}>{i18n.t('agents.tabBuiltin', { count: BUILTIN_AGENTS.length })}</Button
  >
</div>
{#if activeTab === 'custom'}
  <div id="agents-tabpanel-custom" role="tabpanel" aria-labelledby="agents-tab-custom" tabindex="0">
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
              ><td class="model-name">{agent.id}</td><td>{agent.mode ?? i18n.t('common.unset')}</td><td
                >{agent.source}</td
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
  </div>
{:else}
  <div
    id="agents-tabpanel-builtin"
    role="tabpanel"
    aria-labelledby="agents-tab-builtin"
    tabindex="0"
    class="flex flex-col gap-6"
  >
    <section aria-label={i18n.t('agents.slimBuiltin')}>
      <p class="group-title mb-2">{i18n.t('groupForm.typeSlim')}</p>
      <DataTable label={i18n.t('agents.slimBuiltin')} total={slimAgents.length} pageSize={slimAgents.length}
        ><thead
          ><tr
            ><th>{i18n.t('agents.colId')}</th><th>{i18n.t('common.type')}</th><th>{i18n.t('common.description')}</th
            ></tr
          ></thead
        ><tbody
          >{#each slimAgents as agent}<tr
              ><td class="model-name">{agent.id}</td><td>{agent.type}</td><td>{agent.description}</td></tr
            >{/each}</tbody
        ></DataTable
      >
    </section>
    <section aria-label={i18n.t('agents.omoBuiltin')}>
      <p class="group-title mb-2">{i18n.t('groupForm.typeOmo')}</p>
      <DataTable label={i18n.t('agents.omoBuiltin')} total={omoAgents.length} pageSize={omoAgents.length}
        ><thead
          ><tr
            ><th>{i18n.t('agents.colId')}</th><th>{i18n.t('common.type')}</th><th>{i18n.t('common.description')}</th
            ></tr
          ></thead
        ><tbody
          >{#each omoAgents as agent}<tr
              ><td class="model-name">{agent.id}</td><td>{agent.type}</td><td>{agent.description}</td></tr
            >{/each}</tbody
        ></DataTable
      >
    </section>
  </div>
{/if}
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
        <label for="source-to-delete">{i18n.t('agents.sourceToDelete')}</label><Select
          id="source-to-delete"
          bind:value={storage}
          options={STORAGE_OPTIONS.map((option) => ({ value: option.value, label: i18n.t(option.label) }))}
        />
        <p class="text-xs text-muted-foreground">{i18n.t('agents.sourceDeleteHint')}</p>
      </div>{/if}
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="destructive" onclick={remove}>{i18n.t('common.delete')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
