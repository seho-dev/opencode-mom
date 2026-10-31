<script module lang="ts">
export type SettingsSearchTarget = {
  tab: 'general' | 'mcp-proxy' | 'behavior';
  target: string;
  fallback?: string;
};
</script>

<script lang="ts">
import { ArrowDownRight, Search, X } from '@lucide/svelte';
import { Button } from '$src/components/button/index.js';
import { Input } from '$src/components/input/index.js';
import { getI18n } from '$src/i18n/context.js';
import { en, type MessageKey } from '$src/i18n/dictionaries/en.js';
import { zh } from '$src/i18n/dictionaries/zh.js';

let { onselect }: { onselect: (target: SettingsSearchTarget) => void } = $props();
const i18n = getI18n();
let query = $state('');
let input = $state<HTMLInputElement | null>(null);

const entries: (SettingsSearchTarget & { keys: [MessageKey, MessageKey, ...MessageKey[]] })[] = [
  {
    tab: 'behavior',
    target: 'settings-behavior-title',
    keys: ['settings.behaviorTitle', 'settings.behaviorHint', 'settings.behaviorContent', 'settings.behaviorScope'],
  },
  { tab: 'general', target: 'settings-autostart', keys: ['settings.autostart', 'settings.autostartHint'] },
  {
    tab: 'general',
    target: 'settings-lid',
    keys: [
      'settings.lid',
      'settings.lidHint',
      'settings.lidDetails',
      'settings.lidAuthorization',
      'settings.lidScope',
      'settings.lidSafety',
      'settings.lidRestore',
    ],
  },
  { tab: 'general', target: 'settings-island', keys: ['settings.island', 'settings.islandHint'] },
  {
    tab: 'general',
    target: 'opencode-settings-title',
    keys: [
      'settings.opencodeUpdates',
      'settings.opencodeCheckHint',
      'settings.opencodeTitle',
      'settings.opencodeHint',
      'settings.opencodeUpgrade',
      'settings.opencodeVersion',
      'settings.opencodePath',
    ],
  },
  {
    tab: 'general',
    target: 'opencode-service-title',
    fallback: 'opencode-settings-title',
    keys: [
      'settings.serviceMode',
      'settings.serviceReadHint',
      'settings.serviceTitle',
      'settings.serviceShared',
      'settings.serviceSharedHint',
      'settings.serviceExplicit',
      'settings.serviceExplicitHint',
      'settings.servicePid',
      'settings.serviceVersion',
    ],
  },
  {
    tab: 'general',
    target: 'opencode-service-title',
    fallback: 'opencode-settings-title',
    keys: ['settings.serviceUrl', 'settings.serviceUrlHint', 'settings.serviceTitle'],
  },
  {
    tab: 'general',
    target: 'opencode-service-title',
    fallback: 'opencode-settings-title',
    keys: [
      'settings.servicePassword',
      'settings.servicePasswordHint',
      'settings.serviceClearPassword',
      'settings.serviceCredentialWarning',
      'settings.serviceTitle',
    ],
  },
  {
    tab: 'general',
    target: 'settings-app-updates',
    keys: [
      'settings.updates',
      'settings.updatesHint',
      'settings.checkUpdates',
      'settings.macInstallGuide',
      'settings.macSigning',
      'settings.releaseDownloads',
    ],
  },
  {
    tab: 'general',
    target: 'settings-support',
    keys: ['settings.supportProject', 'settings.starRequest', 'settings.starProject'],
  },
  {
    tab: 'general',
    target: 'settings-http-port',
    keys: ['settings.http.port', 'settings.http.portHint', 'settings.http.title', 'settings.http.description'],
  },
  {
    tab: 'general',
    target: 'settings-http-lan',
    keys: [
      'settings.http.allowLan',
      'settings.http.allowLanHint',
      'settings.http.allowLanWarning',
      'settings.http.title',
    ],
  },
  {
    tab: 'mcp-proxy',
    target: 'settings-proxy-timeout',
    keys: ['settings.proxy.idleTimeout', 'settings.proxy.idleTimeoutHint', 'settings.proxy.title'],
  },
  {
    tab: 'mcp-proxy',
    target: 'settings-proxy-capacity',
    keys: ['settings.proxy.maxUnits', 'settings.proxy.maxUnitsHint', 'settings.proxy.title'],
  },
  {
    tab: 'mcp-proxy',
    target: 'settings-proxy-token',
    keys: ['settings.proxy.rotateKey', 'settings.proxy.rotateKeyDescription', 'settings.proxy.title'],
  },
];

const index = entries.map((entry) => ({
  ...entry,
  text: [
    en[
      entry.tab === 'general'
        ? 'settings.tabGeneral'
        : entry.tab === 'behavior'
          ? 'settings.behaviorTab'
          : 'settings.tabMcpProxy'
    ],
    zh[
      entry.tab === 'general'
        ? 'settings.tabGeneral'
        : entry.tab === 'behavior'
          ? 'settings.behaviorTab'
          : 'settings.tabMcpProxy'
    ],
    ...entry.keys.flatMap((key) => [key, en[key], zh[key]]),
  ]
    .join(' ')
    .normalize('NFKC')
    .toLowerCase(),
}));
const terms = $derived(query.normalize('NFKC').trim().toLowerCase().split(/\s+/).filter(Boolean));
const results = $derived(terms.length ? index.filter((entry) => terms.every((term) => entry.text.includes(term))) : []);

function clear() {
  query = '';
  input?.focus();
}
</script>

<search
  aria-label={i18n.t('settings.searchLabel')}
  class="relative sticky -top-8 max-[760px]:-top-5 z-20 mb-5 max-w-3xl bg-[var(--surface-app)] py-2"
>
  <div class="relative">
    <Search size={15} class="pointer-events-none absolute top-2.5 left-3 text-[var(--text-muted)]" aria-hidden="true" />
    <Input
      bind:ref={input}
      type="search"
      name="settings-search"
      bind:value={query}
      autocomplete="off"
      spellcheck={false}
      aria-label={i18n.t('settings.searchLabel')}
      aria-describedby="settings-search-hint"
      aria-controls={terms.length ? 'settings-search-results' : undefined}
      placeholder={i18n.t('settings.searchPlaceholder')}
      class="h-9 pl-9 pr-10 [&::-webkit-search-cancel-button]:appearance-none"
      onkeydown={(event) => { if (event.key === 'Escape' && !event.isComposing) { event.preventDefault(); clear(); } }}
    />
    {#if query}
      <Button
        variant="ghost"
        size="icon"
        class="absolute top-0.5 right-0.5"
        aria-label={i18n.t('settings.searchClear')}
        title={i18n.t('settings.searchClear')}
        onclick={clear}
      >
        <X size={14} aria-hidden="true" />
      </Button>
    {/if}
  </div>
  <p id="settings-search-hint" class="field-hint mt-2">
    {i18n.t('settings.searchHint')}
  </p>
  {#if terms.length}
    <div
      class="absolute top-full left-0 z-30 mt-2 w-full rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-3 shadow-lg"
    >
      <p class="mt-0 mb-2 text-xs leading-5 text-[var(--text-secondary)]" role="status" aria-atomic="true">
        {i18n.t(results.length ? 'settings.searchCount' : 'settings.searchEmpty', { count: results.length })}
      </p>
      {#if results.length}
        <ul
          id="settings-search-results"
          class="m-0 max-h-72 list-none overflow-y-auto p-0 divide-y divide-[var(--border-subtle)]"
          aria-label={i18n.t('settings.searchResults')}
        >
          {#each results as entry (entry.keys[0])}
            <li>
              <button
                type="button"
                class="flex w-full items-center gap-3 rounded-[2px] px-3 py-3 text-left hover:bg-[var(--surface-hover)] focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-[var(--focus-ring)]"
                onclick={() => onselect(entry)}
              >
                <span class="min-w-0 flex-1">
                  <span class="mb-0.5 block text-[10px] leading-4 text-[var(--text-muted)]"
                    >{i18n.t(entry.tab === 'general' ? 'settings.tabGeneral' : entry.tab === 'behavior' ? 'settings.behaviorTab' : 'settings.tabMcpProxy')}</span
                  >
                  <span class="block text-xs leading-5 font-semibold text-[var(--text-primary)]"
                    >{i18n.t(entry.keys[0])}</span
                  >
                  <span class="field-hint line-clamp-2">{i18n.t(entry.keys[1])}</span>
                </span>
                <ArrowDownRight size={15} class="shrink-0 text-[var(--text-muted)]" aria-hidden="true" />
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <Button variant="outline" size="sm" onclick={clear}>{i18n.t('settings.searchClear')}</Button>
      {/if}
    </div>
  {/if}
</search>
