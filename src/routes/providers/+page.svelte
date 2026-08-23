<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Eye, Pencil, Plus, RefreshCw, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import StatusBadge from '$lib/components/app/StatusBadge.svelte';
  import { getConfig } from '$lib/features/config/context.js';

  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let renaming = $state<string | null>(null);
  let newId = $state('');
  let revealed = $state<{ providerId: string; key: string; value: string } | null>(null);
  let revealTimer: ReturnType<typeof setTimeout> | undefined;
  const filtered = $derived(
    config.providers.filter((provider) =>
      `${provider.id} ${provider.name ?? ''}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
  const isMasked = (value: unknown) =>
    typeof value === 'object' &&
    value !== null &&
    'configured' in value &&
    (value as { configured?: unknown }).configured === true;
  const isSensitive = (key: string, value: unknown) =>
    /(api.?key|token|secret|password|credential)/i.test(key) || isMasked(value);

  function hide() {
    if (revealTimer) clearTimeout(revealTimer);
    revealTimer = undefined;
    revealed = null;
  }
  async function reveal(providerId: string, key: string) {
    hide();
    try {
      const value = await config.revealProviderSecret(providerId, key);
      revealed = { providerId, key, value: typeof value === 'string' ? value : JSON.stringify(value) };
      revealTimer = setTimeout(hide, 10_000);
    } catch {
      /* ConfigFeedback renders the command error. */
    }
  }
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteProvider(deleting);
      deleting = null;
    } catch {
      /* ConfigFeedback renders the command error. */
    }
  }
  async function rename() {
    if (!renaming) return;
    try {
      await config.renameProvider(renaming, newId);
      renaming = null;
      newId = '';
    } catch {
      /* ConfigFeedback renders the command error. */
    }
  }
  onDestroy(hide);
</script>

<svelte:head><title>Providers · opencode-mom</title></svelte:head>
<PageHead eyebrow="CONFIG / PROVIDERS" title="Provider Registry"
  >{#snippet children()}<Button href="/providers/new"><Plus size={14} /> New provider</Button>{/snippet}</PageHead
>
{#if config.error}<div class="state-banner error" role="alert">
    {config.error.message}<Button size="sm" variant="ghost" onclick={() => config.refresh()}
      ><RefreshCw size={13} /> Retry</Button
    >
  </div>{/if}
<div class="search-toolbar">
  <input aria-label="Search providers" bind:value={query} placeholder="Search provider ID / name" />
</div>
<DataTable label="Provider Registry"
  ><thead><tr><th>ID</th><th>Name</th><th>Models</th><th>Options</th><th>Status</th><th>Actions</th></tr></thead><tbody>
    {#if config.loading}<tr><td colspan="6" class="empty-table-row">Loading configuration...</td></tr>
    {:else if !filtered.length}<EmptyTableRow
        colspan={6}
        message="No providers. Models must be attached to a provider first."
      />
    {:else}{#each filtered as provider}<tr
          ><td class="model-name">{provider.id}</td><td>{provider.name ?? 'unnamed'}</td><td
            >{Object.keys(provider.models).length}</td
          ><td
            >{#each Object.entries(provider.options ?? {}) as [key, value]}<div>
                {key}:
                <code>{isSensitive(key, value) ? '••••' : String(value)}</code>{#if isSensitive(key, value)}<Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label={`Reveal ${key}`}
                    title="Temporarily reveal sensitive option"
                    onclick={() => reveal(provider.id, key)}><Eye size={13} /></Button
                  >{/if}
              </div>{/each}</td
          ><td><StatusBadge variant="success">Loaded</StatusBadge></td><td class="row-actions"
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Rename ${provider.id}`}
              onclick={() => {
                renaming = provider.id;
                newId = provider.id;
              }}><Pencil size={14} /></Button
            ><Button
              href={`/providers/${provider.id}/edit`}
              variant="ghost"
              size="icon-sm"
              aria-label={`Edit ${provider.id}`}><Pencil size={14} /></Button
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Delete ${provider.id}`}
              onclick={() => (deleting = provider.id)}><Trash2 size={14} /></Button
            ></td
          ></tr
        >{/each}{/if}
  </tbody></DataTable
>
{#if renaming}<div class="confirm-strip" role="alert">
    New provider ID <input aria-label="New provider ID" bind:value={newId} /><Button
      size="sm"
      variant="outline"
      onclick={() => (renaming = null)}>Cancel</Button
    ><Button size="sm" onclick={rename}>Save rename</Button>
  </div>{/if}
{#if deleting}<div class="confirm-strip" role="alert">
    Delete provider <strong>{deleting}</strong>?<Button size="sm" variant="outline" onclick={() => (deleting = null)}
      >Cancel</Button
    ><Button size="sm" variant="destructive" onclick={remove}>Delete</Button>
  </div>{/if}
{#if revealed}<div class="state-banner" role="status">
    Temporarily revealed <code>{revealed.providerId}.{revealed.key}: {revealed.value}</code><Button
      size="sm"
      variant="ghost"
      onclick={hide}>Hide</Button
    >
  </div>{/if}
