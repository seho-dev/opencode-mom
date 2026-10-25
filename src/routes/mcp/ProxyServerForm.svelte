<script lang="ts">
import { Download } from '@lucide/svelte';
import { onDestroy } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import * as Dialog from '$src/components/dialog/index.js';
import FormActions from '$src/components/FormActions.svelte';
import { Input } from '$src/components/input/index.js';
import Select from '$src/components/Select.svelte';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpServer } from '$src/types/mcp.js';
import type { McpProxyProtocol, McpProxyServerDto } from '$src/types/mcp-proxy.js';

interface Props {
  open?: boolean;
  server?: McpProxyServerDto | null;
  expectedRevision?: number;
  dialog?: boolean;
  onSaved?: () => Promise<void> | void;
  onCancel?: () => void;
}

let {
  open = $bindable(false),
  server = null,
  expectedRevision = 0,
  dialog = true,
  onSaved,
  onCancel,
}: Props = $props();

const config = getConfig();
const i18n = getI18n();

let id = $state('');
let name = $state('');
let enabled = $state(true);
let commandText = $state('');
let cwd = $state('');
let envText = $state('');
let protocol = $state<McpProxyProtocol>('auto');
let startupTimeout = $state(30);
let executionTimeout = $state(300);

let pending = $state(false);
let error = $state('');
let importMode = $state(false);
let availableMcps = $state<McpServer[]>([]);
let selectedImportMcp = $state('');
let loadingMcps = $state(false);
let mounted = true;

$effect(() => {
  if (server) {
    id = server.id;
    name = server.name;
    enabled = server.enabled;
    commandText = server.command.join('\n');
    cwd = server.cwd;
    envText = Object.entries(server.env || {})
      .map(([k, v]) => `${k}=${v}`)
      .join('\n');
    protocol = (server.protocol?.toLowerCase() as McpProxyProtocol) || 'auto';
    startupTimeout = server.startupTimeout ?? 30;
    executionTimeout = server.executionTimeout ?? 300;
  } else {
    id = '';
    name = '';
    enabled = true;
    commandText = '';
    cwd = '';
    envText = '';
    protocol = 'auto';
    startupTimeout = 30;
    executionTimeout = 300;
  }
  error = '';
  importMode = false;
});

onDestroy(() => {
  mounted = false;
});

function handleCancel() {
  if (!mounted || pending) return;
  open = false;
  error = '';
  importMode = false;
  onCancel?.();
}

async function loadExistingMcps() {
  if (!mounted || pending || loadingMcps) return;
  loadingMcps = true;
  try {
    const list = await config.listMcps();
    if (!mounted) return;
    availableMcps = list.data;
    if (availableMcps.length > 0 && !selectedImportMcp) {
      selectedImportMcp = availableMcps[0]?.name ?? '';
    }
  } catch (cause) {
    if (mounted) error = readError(cause);
  } finally {
    if (mounted) loadingMcps = false;
  }
}

async function performImport() {
  if (!mounted || pending || loadingMcps || !selectedImportMcp) return;
  const target = availableMcps.find((m) => m.name === selectedImportMcp);
  if (!target) return;

  pending = true;
  error = '';
  try {
    const imported = await config.importMcpProxyServer({
      scope: 'global',
      serverName: target.name,
      expectedSourcePath: target.sourcePath,
    });
    if (!mounted) return;

    id = imported.id;
    name = imported.name;
    enabled = imported.enabled;
    commandText = imported.command.join('\n');
    cwd = imported.cwd;
    envText = Object.entries(imported.env || {})
      .map(([k, v]) => `${k}=${v}`)
      .join('\n');
    protocol = (imported.protocol?.toLowerCase() as McpProxyProtocol) || 'auto';
    startupTimeout = imported.startupTimeout ?? 30;
    executionTimeout = imported.executionTimeout ?? 300;
    importMode = false;
  } catch (cause) {
    if (mounted) error = readError(cause);
  } finally {
    if (mounted) pending = false;
  }
}

function parseEnv(text: string): Record<string, string> {
  const entries: [string, string][] = [];
  for (const line of text.split(/\r?\n/)) {
    if (line.trimStart().startsWith('#')) continue;
    const idx = line.indexOf('=');
    if (idx > 0) {
      const k = line.slice(0, idx).trim();
      entries.push([k, line.slice(idx + 1)]);
    }
  }
  return Object.fromEntries(entries);
}

async function save() {
  if (!mounted || pending) return;
  const trimmedId = id.trim();
  if (!trimmedId) {
    error = i18n.t('mcp.proxy.form.validationId');
    return;
  }
  const cmd = commandText.split(/\r?\n/);
  if (!cmd[0]?.trim()) {
    error = i18n.t('mcp.proxy.form.validationCommand');
    return;
  }

  const payload: McpProxyServerDto = {
    id: trimmedId,
    name: name.trim() || trimmedId,
    enabled,
    command: cmd,
    cwd: cwd.trim(),
    env: parseEnv(envText),
    protocol,
    startupTimeout: Number(startupTimeout) || 30,
    executionTimeout: Number(executionTimeout) || 300,
  };

  pending = true;
  error = '';
  try {
    let rev = expectedRevision;
    if (server) {
      await config.updateMcpProxyServer(payload, expectedRevision);
    } else {
      if (!rev) {
        const state = await config.getMcpProxyState();
        if (!mounted) return;
        rev = state.revision;
      }
      await config.createMcpProxyServer(payload, rev);
    }
    if (!mounted) return;
    open = false;
    await onSaved?.();
  } catch (cause) {
    if (mounted) error = readError(cause);
  } finally {
    if (mounted) pending = false;
  }
}
</script>

{#snippet formFields()}
  {#if error}
    <div class="state-banner error" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]">{error}</span>
    </div>
  {/if}

  {#if importMode}
    <div class="space-y-4 py-2">
      <div class="flex items-center justify-between">
        <p class="form-hint m-0">{i18n.t('mcp.proxy.import.select')}</p>
        <Button
          variant="ghost"
          size="sm"
          disabled={pending}
          onclick={() => {
            importMode = false;
            error = '';
          }}
        >
          {i18n.t('common.cancel')}
        </Button>
      </div>
      {#if loadingMcps}
        <p class="text-xs text-[var(--text-muted)]">{i18n.t('common.loadingConfiguration')}</p>
      {:else if availableMcps.length === 0}
        <p class="text-xs text-[var(--text-muted)]">{i18n.t('mcp.proxy.import.empty')}</p>
      {:else}
        <Select
          id="proxy-import-select"
          bind:value={selectedImportMcp}
          disabled={pending}
          alwaysSearch
          placeholder={i18n.t('mcp.proxy.import.select')}
          options={availableMcps.map((mcp) => ({
            value: mcp.name,
            label: `${mcp.name} (${mcp.type})`,
          }))}
        />
      {/if}
    </div>
  {:else}
    <div class="space-y-4 py-2 text-xs">
      <!-- ID & Name -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <div>
          <label for="proxy-srv-id" class="font-medium text-[var(--text-primary)] block mb-1">
            {i18n.t('mcp.proxy.form.id')}
          </label>
          <Input id="proxy-srv-id" disabled={!!server || pending} bind:value={id} placeholder="fs-server" />
          <span class="field-hint mt-1">
            {i18n.t('mcp.proxy.form.idHint')}
          </span>
        </div>

        <div>
          <label for="proxy-srv-name" class="font-medium text-[var(--text-primary)] block mb-1">
            {i18n.t('mcp.proxy.form.name')}
          </label>
          <Input id="proxy-srv-name" disabled={pending} bind:value={name} placeholder="Filesystem" />
        </div>
      </div>

      <!-- Enabled Switch -->
      <div class="flex items-center justify-between gap-4 py-1">
        <span class="font-medium text-[var(--text-primary)]">{i18n.t('mcp.proxy.form.enabled')}</span>
        <Switch
          id="proxy-srv-enabled"
          name="proxy-srv-enabled"
          aria-label={i18n.t('mcp.proxy.form.enabled')}
          bind:checked={enabled}
          disabled={pending}
        />
      </div>

      <!-- Protocol Select -->
      <div>
        <label for="proxy-srv-protocol" class="font-medium text-[var(--text-primary)] block mb-1">
          {i18n.t('mcp.proxy.form.protocol')}
        </label>
        <Select
          id="proxy-srv-protocol"
          bind:value={protocol}
          disabled={pending}
          alwaysSearch
          options={[
            { value: 'auto', label: i18n.t('mcp.proxy.form.protocolAuto') },
            { value: 'legacy', label: i18n.t('mcp.proxy.form.protocolLegacy') },
            { value: 'modern', label: i18n.t('mcp.proxy.form.protocolModern') },
          ]}
        />
      </div>

      <!-- Command & Arguments -->
      <div>
        <label for="proxy-srv-cmd" class="font-medium text-[var(--text-primary)] block mb-1">
          {i18n.t('mcp.proxy.form.command')}
        </label>
        <textarea
          id="proxy-srv-cmd"
          class="input-field font-mono text-xs w-full min-h-20"
          disabled={pending}
          bind:value={commandText}
          placeholder="npx&#10;-y&#10;@modelcontextprotocol/server-filesystem"
        ></textarea>
        <span class="field-hint mt-1">
          {i18n.t('mcp.proxy.form.commandHint')}
        </span>
      </div>

      <!-- Working Directory -->
      <div>
        <label for="proxy-srv-cwd" class="font-medium text-[var(--text-primary)] block mb-1">
          {i18n.t('mcp.proxy.form.cwd')}
        </label>
        <Input id="proxy-srv-cwd" disabled={pending} bind:value={cwd} placeholder="/Users/example/project" />
        <span class="field-hint mt-1">
          {i18n.t('mcp.proxy.form.cwdHint')}
        </span>
      </div>

      <!-- Environment Variables -->
      <div>
        <label for="proxy-srv-env" class="font-medium text-[var(--text-primary)] block mb-1">
          {i18n.t('mcp.proxy.form.env')}
        </label>
        <textarea
          id="proxy-srv-env"
          class="input-field font-mono text-xs w-full min-h-16"
          disabled={pending}
          bind:value={envText}
          placeholder="KEY=VALUE&#10;ANOTHER_KEY=VALUE"
        ></textarea>
        <span class="field-hint mt-1">
          {i18n.t('mcp.proxy.form.envHint')}
        </span>
      </div>

      <!-- Timeouts -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <div>
          <label for="proxy-srv-startup" class="font-medium text-[var(--text-primary)] block mb-1">
            {i18n.t('mcp.proxy.form.startupTimeout')}
          </label>
          <Input
            id="proxy-srv-startup"
            type="number"
            min={1}
            max={300}
            disabled={pending}
            bind:value={startupTimeout}
          />
        </div>

        <div>
          <label for="proxy-srv-exec" class="font-medium text-[var(--text-primary)] block mb-1">
            {i18n.t('mcp.proxy.form.executionTimeout')}
          </label>
          <Input
            id="proxy-srv-exec"
            type="number"
            min={1}
            max={3600}
            disabled={pending}
            bind:value={executionTimeout}
          />
        </div>
      </div>
    </div>
  {/if}
{/snippet}

{#if dialog}
  <Dialog.Root
    bind:open
    onOpenChange={(val) => {
      if (!val) handleCancel();
    }}
  >
    <Dialog.Content
      aria-busy={pending}
      class="max-w-xl max-h-[calc(100dvh-32px)] flex flex-col overflow-hidden"
      onEscapeKeydown={(event) => {
        if (pending) event.preventDefault();
      }}
      onInteractOutside={(event) => {
        if (pending) event.preventDefault();
      }}
    >
      <Dialog.Header class="shrink-0">
        <div class="flex items-center justify-between gap-4">
          <div>
            <Dialog.Title>
              {i18n.t(server ? 'mcp.proxy.form.editTitle' : 'mcp.proxy.form.createTitle')}
            </Dialog.Title>
          </div>
          {#if !server && !importMode}
            <Button
              variant="outline"
              size="sm"
              disabled={pending}
              onclick={() => {
                importMode = true;
                void loadExistingMcps();
              }}
            >
              <Download size={14} />
              {i18n.t('mcp.proxy.importServer')}
            </Button>
          {/if}
        </div>
      </Dialog.Header>

      <div class="form-body">
        {@render formFields()}
      </div>

      <FormActions>
        <Button variant="outline" disabled={pending} onclick={handleCancel}>
          {i18n.t('common.cancel')}
        </Button>
        <Button variant="default" disabled={pending} onclick={importMode ? performImport : save}>
          {pending
            ? i18n.t('mcp.proxy.form.saving')
            : importMode
              ? i18n.t('mcp.proxy.import.submit')
              : i18n.t('mcp.proxy.form.save')}
        </Button>
      </FormActions>
    </Dialog.Content>
  </Dialog.Root>
{:else}
  <form
    class="panel form-panel min-w-0"
    aria-label={i18n.t(server ? 'mcp.proxy.form.editTitle' : 'mcp.proxy.form.createTitle')}
    aria-busy={pending}
    onsubmit={(event) => {
      event.preventDefault();
      if (importMode) void performImport();
      else void save();
    }}
  >
    <div class="form-body">
      {#if !server && !importMode}
        <div class="flex justify-end mb-2">
          <Button
            variant="outline"
            size="sm"
            disabled={pending}
            onclick={() => {
              importMode = true;
              void loadExistingMcps();
            }}
          >
            <Download size={14} />
            {i18n.t('mcp.proxy.importServer')}
          </Button>
        </div>
      {/if}
      {@render formFields()}
    </div>

    <FormActions>
      <Button variant="outline" disabled={pending} onclick={handleCancel}>
        {i18n.t('common.cancel')}
      </Button>
      <Button type="submit" variant="default" disabled={pending}>
        {pending
          ? i18n.t('mcp.proxy.form.saving')
          : importMode
            ? i18n.t('mcp.proxy.import.submit')
            : i18n.t('mcp.proxy.form.save')}
      </Button>
    </FormActions>
  </form>
{/if}
