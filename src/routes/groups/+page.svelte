<script lang="ts">
import { Pencil, Play, Plus, RefreshCw, Trash2 } from '@lucide/svelte';
import { Button } from '$src/components/button/index.js';
import DataTable from '$src/components/DataTable.svelte';
import * as Dialog from '$src/components/dialog/index.js';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import {
  GROUP_TYPE_LABELS,
  GROUP_TYPE_NATIVE,
  GROUP_TYPE_OMO,
  GROUP_TYPE_SLIM,
  type GroupType,
} from '$src/utils/constants.js';

const tabOrder: GroupType[] = [GROUP_TYPE_NATIVE, GROUP_TYPE_SLIM, GROUP_TYPE_OMO];
const config = getConfig();
const i18n = getI18n();
let query = $state('');
let deleting = $state<string | null>(null);
let refreshing = $state(false);
let changingGroup = $state<string | null>(null);
let switchedGroup = $state<string | null>(null);
let activeTab = $state<GroupType>(GROUP_TYPE_NATIVE);
let nativeTabEl = $state<HTMLButtonElement | null>(null);
let slimTabEl = $state<HTMLButtonElement | null>(null);
let omoTabEl = $state<HTMLButtonElement | null>(null);
const pageSize = 5;
let page = $state(1);
const nativeCount = $derived(config.groups.filter((group) => group.type === GROUP_TYPE_NATIVE).length);
const slimCount = $derived(config.groups.filter((group) => group.type === GROUP_TYPE_SLIM).length);
const omoCount = $derived(config.groups.filter((group) => group.type === GROUP_TYPE_OMO).length);
const groups = $derived(
  config.groups.filter((group) => group.type === activeTab && group.name.toLowerCase().includes(query.toLowerCase())),
);
const maxPage = $derived(Math.max(1, Math.ceil(groups.length / pageSize)));
const currentPage = $derived(Math.min(page, maxPage));
const pagedGroups = $derived(groups.slice((currentPage - 1) * pageSize, currentPage * pageSize));
const busy = $derived(
  refreshing || changingGroup !== null || config.loading || config.saving || config.switching || config.reloading,
);
$effect(() => {
  // Reset to the first page whenever the filtered row count changes (search or delete).
  void groups.length;
  page = 1;
});
function selectTab(tab: GroupType) {
  activeTab = tab;
  page = 1;
}
function tabElement(tab: GroupType) {
  return tab === GROUP_TYPE_NATIVE ? nativeTabEl : tab === GROUP_TYPE_SLIM ? slimTabEl : omoTabEl;
}
function onTabKeys(event: KeyboardEvent) {
  const index = tabOrder.indexOf(activeTab);
  let next: GroupType | undefined;
  if (event.key === 'ArrowRight') next = tabOrder[(index + 1) % tabOrder.length];
  else if (event.key === 'ArrowLeft') next = tabOrder[(index - 1 + tabOrder.length) % tabOrder.length];
  else if (event.key === 'Home') next = tabOrder[0];
  else if (event.key === 'End') next = tabOrder[tabOrder.length - 1];
  else return;
  if (!next) return;
  event.preventDefault();
  selectTab(next);
  tabElement(next)?.focus();
}
async function refresh() {
  if (busy) return;
  refreshing = true;
  try {
    await config.refresh(true);
  } finally {
    refreshing = false;
  }
}
async function toggleEnabled(id: string) {
  if (busy) return;
  const group = config.groups.find((group) => group.id === id);
  if (!group || group.id === config.selectedGroupId) return;
  changingGroup = id;
  try {
    await config.saveGroup({ ...$state.snapshot(group), isEnabled: !group.isEnabled });
  } catch {
    /* global feedback; retain the confirmed state */
  } finally {
    changingGroup = null;
  }
}
async function remove() {
  if (!deleting || busy) return;
  changingGroup = deleting;
  try {
    await config.deleteGroup(deleting);
    deleting = null;
  } catch {
    /* global feedback */
  } finally {
    changingGroup = null;
  }
}
async function activate(id: string, name: string, type: GroupType) {
  if (busy || !config.groups.find((group) => group.id === id)?.isEnabled) return;
  changingGroup = id;
  try {
    await config.switchGroup(id);
    if (type === GROUP_TYPE_SLIM || type === GROUP_TYPE_OMO) switchedGroup = name;
  } catch {
    /* global feedback */
  } finally {
    changingGroup = null;
  }
}
async function reloadNow() {
  if (busy) return;
  try {
    await config.reloadOpencode();
    switchedGroup = null;
  } catch {
    /* global feedback; keep the prompt open for retry */
  }
}
</script>

<svelte:head><title>{i18n.t('groups.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('groups.eyebrow')} title={i18n.t('groups.title')}
  >{#snippet children()}
    <div class="flex flex-wrap justify-end gap-2">
      <Button variant="outline" size="sm" disabled={busy} aria-busy={refreshing} onclick={refresh}>
        <RefreshCw size={14} class={refreshing ? 'animate-spin motion-reduce:animate-none' : ''} aria-hidden="true" />
        {i18n.t('configuration.refresh')}
      </Button>
      <Button size="sm" href="/groups/new" disabled={busy}><Plus size={14} /> {i18n.t('groups.new')}</Button>
    </div>
  {/snippet}</PageHead
>
<div class="search-toolbar">
  <input aria-label={i18n.t('groups.searchLabel')} bind:value={query} placeholder={i18n.t('groups.searchPlaceholder')}>
</div>
<div class="flex gap-2 mb-3" role="tablist" aria-label={i18n.t('groups.title')}>
  <Button
    bind:ref={nativeTabEl}
    id="groups-tab-native"
    role="tab"
    variant={activeTab === GROUP_TYPE_NATIVE ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === GROUP_TYPE_NATIVE}
    aria-controls="groups-tabpanel-native"
    tabindex={activeTab === GROUP_TYPE_NATIVE ? 0 : -1}
    onclick={() => selectTab(GROUP_TYPE_NATIVE)}
    onkeydown={onTabKeys}
    >{i18n.t('groups.tabNative', { count: nativeCount })}</Button
  >
  <Button
    bind:ref={slimTabEl}
    id="groups-tab-slim"
    role="tab"
    variant={activeTab === GROUP_TYPE_SLIM ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === GROUP_TYPE_SLIM}
    aria-controls="groups-tabpanel-slim"
    tabindex={activeTab === GROUP_TYPE_SLIM ? 0 : -1}
    onclick={() => selectTab(GROUP_TYPE_SLIM)}
    onkeydown={onTabKeys}
    >{i18n.t('groups.tabSlim', { count: slimCount })}</Button
  >
  <Button
    bind:ref={omoTabEl}
    id="groups-tab-omo"
    role="tab"
    variant={activeTab === GROUP_TYPE_OMO ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === GROUP_TYPE_OMO}
    aria-controls="groups-tabpanel-omo"
    tabindex={activeTab === GROUP_TYPE_OMO ? 0 : -1}
    onclick={() => selectTab(GROUP_TYPE_OMO)}
    onkeydown={onTabKeys}
    >{i18n.t('groups.tabOmo', { count: omoCount })}</Button
  >
</div>
<div id={`groups-tabpanel-${activeTab}`} role="tabpanel" aria-labelledby={`groups-tab-${activeTab}`}>
  <DataTable label={i18n.t('groups.tableLabel')} total={groups.length} bind:page {pageSize}
    ><thead>
      <tr>
        <th>{i18n.t('groups.colName')}</th>
        <th>{i18n.t('groups.colType')}</th>
        <th>{i18n.t('groups.colMappings')}</th>
        <th>{i18n.t('groups.colStatus')}</th>
        <th class="th-actions">{i18n.t('groups.colActions')}</th>
      </tr>
    </thead>
    <tbody>
      {#if config.loading && !groups.length}
        <EmptyTableRow colspan={5} message={i18n.t('groups.loading')} />
      {:else if !groups.length}
        <EmptyTableRow colspan={5} message={i18n.t('empty.groups')} />
      {:else}
        {#each pagedGroups as group (group.id)}
          <tr>
            <td class="model-name">
              {group.name}
              <div class="muted">{group.description}</div>
            </td>
            <td>{i18n.t(GROUP_TYPE_LABELS[group.type])}</td>
            <td>
              {group.openCodeAgentOverrides.length +
                (group.slimAgentOverrides?.length ?? 0) +
                (group.omoAgentOverrides?.length ?? 0) +
                (group.omoCategoryMappings?.length ?? 0)}
            </td>
            <td>
              <div class="flex items-center">
                <Switch
                  checked={group.isEnabled}
                  disabled={busy || group.id === config.selectedGroupId}
                  aria-label={i18n.t(group.isEnabled ? 'groups.disableAria' : 'groups.enableAria', { name: group.name })}
                  aria-busy={changingGroup === group.id}
                  title={group.id === config.selectedGroupId ? i18n.t('dashboard.selected') : undefined}
                  onclick={(event) => {
                    event.preventDefault();
                    void toggleEnabled(group.id);
                  }}
                />
              </div>
            </td>
            <td class="row-actions">
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={i18n.t('groups.switchTo', { name: group.name })}
                title={i18n.t('groups.switchTitle')}
                onclick={() => activate(group.id, group.name, group.type)}
                disabled={busy || !group.isEnabled}
                ><Play size={14} /></Button
              ><Button
                href={`/groups/${group.id}/edit`}
                variant="ghost"
                size="icon-sm"
                disabled={busy}
                aria-label={i18n.t('groups.editAria', { name: group.name })}
                ><Pencil size={14} /></Button
              ><Button
                variant="ghost"
                size="icon-sm"
                disabled={busy}
                aria-label={i18n.t('groups.deleteAria', { name: group.name })}
                onclick={() => { if (!busy) deleting = group.id; }}
                ><Trash2 size={14} /></Button
              >
            </td>
          </tr>
        {/each}
      {/if}
    </tbody></DataTable
  >
</div>
<Dialog.Root
  open={deleting !== null}
  onOpenChange={(open) => {
    if (!open && !busy) deleting = null;
  }}
>
  <Dialog.Content
    aria-busy={busy}
    onEscapeKeydown={(event) => { if (busy) event.preventDefault(); }}
    onInteractOutside={(event) => { if (busy) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('groups.deleteTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('groups.deleteConfirm', { name: deleting ?? '' })}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" disabled={busy} onclick={() => (deleting = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="destructive" disabled={busy} onclick={remove}>{i18n.t('common.delete')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
<Dialog.Root
  open={switchedGroup !== null}
  onOpenChange={(open) => {
    if (!open && !busy) switchedGroup = null;
  }}
>
  <Dialog.Content
    aria-busy={config.reloading}
    onEscapeKeydown={(event) => { if (busy) event.preventDefault(); }}
    onInteractOutside={(event) => { if (busy) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('groups.reloadTitle')}</Dialog.Title>
      <Dialog.Description>
        <span class="block">{i18n.t('groups.reloadDescription', { name: switchedGroup ?? '' })}</span>
        <span class="mt-2 block">{i18n.t('groups.reloadWarning')}</span>
      </Dialog.Description>
      {#if config.reloading}
        <span class="sr-only" role="status">{i18n.t('groups.reloading')}</span>
      {/if}
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="ghost" disabled={busy} onclick={() => (switchedGroup = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="outline" disabled={busy} onclick={() => (switchedGroup = null)}
        >{i18n.t('groups.reloadLater')}</Button
      >
      <Button disabled={busy} onclick={reloadNow}
        >{i18n.t(config.reloading ? 'groups.reloading' : 'groups.reloadNow')}</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
