<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let replacement = $state('');
  let renaming = $state<string | null>(null);
  let newId = $state('');
  const models = $derived(
    config.models().filter((model) => `${model.ref} ${model.name ?? ''}`.toLowerCase().includes(query.toLowerCase())),
  );
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteModel(deleting as `${string}/${string}`);
      deleting = null;
    } catch {
      /* global feedback */
    }
  }
  async function replace() {
    if (!deleting || !replacement) return;
    try {
      await config.replaceReferences(deleting as `${string}/${string}`, replacement as `${string}/${string}`);
      deleting = null;
      replacement = '';
    } catch {
      /* global feedback */
    }
  }
  async function rename() {
    if (!renaming || !newId.trim()) return;
    try {
      await config.renameModel(renaming as `${string}/${string}`, newId.trim());
      renaming = null;
      newId = '';
    } catch {
      /* global feedback */
    }
  }
</script>

<svelte:head><title>Models · opencode-mom</title></svelte:head><PageHead eyebrow="CONFIG / MODELS" title="Models"
  >{#snippet children()}<Button href="/models/new"><Plus size={14} /> New model</Button>{/snippet}</PageHead
>
<div class="search-toolbar">
  <input aria-label="Search models" bind:value={query} placeholder="Search full model references" />
</div>
<DataTable label="Model directory"
  ><thead><tr><th>Full reference</th><th>Provider</th><th>Name</th><th>Context</th><th>Actions</th></tr></thead><tbody
    >{#if config.loading}<tr><td colspan="5" class="empty-table-row">Loading configuration...</td></tr
      >{:else if !models.length}<EmptyTableRow
        colspan={5}
        message="No models. Create a provider first."
      />{:else}{#each models as model}<tr
          ><td class="model-name">{model.ref}</td><td>{model.providerId}</td><td>{model.name ?? 'unnamed'}</td><td
            >{model.limit?.context ?? 'unset'}</td
          ><td class="row-actions"
            ><Button
              variant="ghost"
              size="icon-sm"
              title="Rename model"
              aria-label={`Rename ${model.ref}`}
              onclick={() => {
                renaming = model.ref;
                newId = model.id;
              }}><Pencil size={14} /></Button
            ><Button
              href={`/models/${encodeURIComponent(model.ref)}/edit`}
              variant="ghost"
              size="icon-sm"
              title="Edit model"
              aria-label={`Edit ${model.ref}`}><Pencil size={14} /></Button
            ><Button
              variant="ghost"
              size="icon-sm"
              title="Delete model"
              aria-label={`Delete ${model.ref}`}
              onclick={() => (deleting = model.ref)}><Trash2 size={14} /></Button
            ></td
          ></tr
        >{/each}{/if}</tbody
  ></DataTable
>
{#if renaming}<div class="confirm-strip" role="alert">
    New model ID <input aria-label="New model ID" bind:value={newId} /><Button
      size="sm"
      variant="outline"
      onclick={() => (renaming = null)}>Cancel</Button
    ><Button size="sm" onclick={rename}>Save rename</Button>
  </div>{/if}
{#if deleting}<div class="confirm-strip" role="alert">
    Delete <strong>{deleting}</strong>? Deletion is blocked while references exist.<select
      aria-label="Replace with model"
      bind:value={replacement}
      ><option value="">Do not replace references</option>{#each config
        .models()
        .filter((model) => model.ref !== deleting) as model}<option value={model.ref}>{model.ref}</option
        >{/each}</select
    ><Button size="sm" variant="outline" onclick={() => (deleting = null)}>Cancel</Button>{#if replacement}<Button
        size="sm"
        onclick={replace}>Replace references</Button
      >{/if}<Button size="sm" variant="destructive" onclick={remove}>Delete</Button>
  </div>{/if}
