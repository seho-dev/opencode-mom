<script lang="ts">
  import { AlertTriangle, Check, RefreshCw, Wrench } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { getConfig } from '$lib/features/config/context.js';
  const config = getConfig();
</script>

{#if config.error}
  <div class="state-banner error" role="alert">
    <AlertTriangle size={15} /><span>{config.error.message}</span>
    {#if config.error.code === 'conflict'}
      <span class="muted">The local draft is kept in the current form and was not overwritten.</span>
      <Button size="sm" variant="outline" onclick={() => config.reloadKeepingDraft()}
        ><RefreshCw size={13} /> Keep draft and reload</Button
      >
      <Button size="sm" variant="ghost" onclick={() => config.continueEditing()}>Review and merge manually</Button>
      <Button size="sm" variant="destructive" onclick={() => config.discardDraftAndRefresh()}
        >Discard draft and refresh</Button
      >
    {:else if config.error.code === 'busy'}
      <span class="muted"
        >Another configuration operation is still running. The local draft is kept; retry in the original form later.</span
      >
      <Button size="sm" variant="ghost" onclick={() => config.continueEditing()}>Continue editing</Button>
    {:else if config.error.code === 'references_blocked'}
      <span class="muted">Replace or remove the references first.</span><Button
        size="sm"
        variant="ghost"
        onclick={() => config.continueEditing()}>Back to draft</Button
      >
    {:else}
      <span class="muted"
        >{config.draftRecovery
          ? 'The request content is preserved; no automatic merge was performed. Review it in the original form and retry.'
          : 'Review the current page and retry.'}</span
      ><Button size="sm" variant="ghost" onclick={() => config.continueEditing()}>Continue editing</Button>
    {/if}
  </div>
{:else if config.notice}
  <div class="state-banner success" role="status">
    <Check size={15} /><span>{config.notice}</span><Button
      size="sm"
      variant="ghost"
      onclick={() => config.clearNotice()}>Close</Button
    >
  </div>
{/if}
{#if config.diagnostics.length}
  <details class="diagnostics" open>
    <summary><Wrench size={14} /> Config diagnostics ({config.diagnostics.length})</summary>
    <ul>
      {#each config.diagnostics as diagnostic}<li>
          {diagnostic}{#if /legacy|omo/i.test(diagnostic)}
            <span class="muted">Legacy OMO configuration is currently ignored.</span>{/if}
        </li>{/each}
    </ul>
  </details>
{/if}
