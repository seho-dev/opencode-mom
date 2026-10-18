import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { SvelteMap } from 'svelte/reactivity';
import { afterAll, beforeAll, beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AgentDefinition, AgentWrite } from '$src/types/agents.js';
import EditAgent from '../[id]/edit/+page.svelte';
import NewAgent from '../new/+page.svelte';

const { goto } = vi.hoisted(() => ({ goto: vi.fn() }));
const routeId = new SvelteMap([['id', 'helper']]);
vi.mock('$app/navigation', () => ({ goto }));
vi.mock('$app/state', () => ({
  page: {
    params: {
      get id() {
        return routeId.get('id');
      },
    },
  },
}));

const model = {
  ref: 'demo/fast',
  id: 'fast',
  modelId: 'fast',
  providerId: 'demo',
  isCustom: true,
  name: 'Fast',
  variants: [{ id: 'quick' }],
} satisfies ReturnType<ConfigStore['models']>[number] & ReturnType<ConfigStore['catalogModels']>[number];
const rule = { action: 'read', resource: '*.ts', effect: 'allow' };
const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;

beforeAll(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
});
afterAll(() => {
  if (originalScrollIntoView) HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
  else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});

function mount(
  component: typeof NewAgent | typeof EditAgent,
  agents: AgentDefinition[] = [],
  initiallyLoading = false,
) {
  const createAgent = vi.fn(async (_value: AgentWrite) => {});
  const updateAgent = vi.fn(async (_value: AgentWrite) => {});
  const reset = new SvelteMap([['version', 0]]);
  const agentStore = new SvelteMap([['agents', agents]]);
  const loadingStore = new SvelteMap([['loading', initiallyLoading]]);
  const config: Partial<ConfigStore> = {
    preferences: { theme: 'light', locale: 'en' },
    get loading() {
      return loadingStore.get('loading') ?? false;
    },
    saving: false,
    catalogLoading: false,
    get agents() {
      return agentStore.get('agents') ?? [];
    },
    get formResetVersion() {
      return reset.get('version') ?? 0;
    },
    models: () => [model],
    catalogModels: () => [model],
    createAgent,
    updateAgent,
  };
  render(component, {}, { wrapper: Harness, wrapperProps: { config: config as ConfigStore } });
  return {
    createAgent,
    updateAgent,
    reset,
    setAgents: (value: AgentDefinition[]) => agentStore.set('agents', value),
    setLoading: (value: boolean) => loadingStore.set('loading', value),
  };
}

async function choose(label: string, option: string) {
  await fireEvent.click(screen.getByLabelText(label));
  await fireEvent.click(within(screen.getByRole('listbox')).getByRole('option', { name: option }));
}

beforeEach(() => {
  goto.mockReset();
  routeId.set('id', 'helper');
});

test('built-in edit is read-only even when the agent exists in config', () => {
  routeId.set('id', 'build');
  const { updateAgent } = mount(EditAgent, [
    {
      id: 'build',
      source: 'inline',
      sources: [
        {
          storage: 'inline',
          raw: { description: 'Built-in description' },
          fields: { description: 'Built-in description' },
        },
      ],
    },
  ]);
  expect(screen.getByRole('status').textContent).toBe('Built-in agents are read-only.');
  expect(screen.queryByRole('button', { name: 'Save changes' })).toBeNull();
  expect(screen.queryByRole('textbox', { name: /Description/ })).toBeNull();
  expect(updateAgent).not.toHaveBeenCalled();
});

test('edit waits for an initially missing agent and loads it when available', async () => {
  const { setAgents, setLoading, updateAgent } = mount(EditAgent, [], true);
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Save changes' })).toBeNull();
  setLoading(false);
  expect((await screen.findByRole('alert')).textContent).toBe('The agent does not exist or has not been loaded yet.');
  setAgents([
    {
      id: 'helper',
      source: 'inline',
      sources: [
        {
          storage: 'inline',
          raw: { description: 'Loaded description' },
          fields: { description: 'Loaded description' },
        },
      ],
    },
  ]);
  await waitFor(() =>
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: /Description/ }).value).toBe('Loaded description'),
  );
  expect(screen.queryByRole('alert')).toBeNull();
  expect(updateAgent).not.toHaveBeenCalled();
});

test('edit switches agent snapshots when the route id changes without remounting', async () => {
  const { updateAgent } = mount(EditAgent, [
    {
      id: 'helper',
      source: 'inline',
      sources: [
        { storage: 'inline', raw: { description: 'First description' }, fields: { description: 'First description' } },
      ],
    },
    {
      id: 'second',
      source: 'inline',
      sources: [
        {
          storage: 'inline',
          raw: { description: 'Second description' },
          fields: { description: 'Second description' },
        },
      ],
    },
  ]);
  const description = screen.getByRole<HTMLInputElement>('textbox', { name: /Description/ });
  await waitFor(() => expect(description.value).toBe('First description'));
  await fireEvent.input(description, { target: { value: 'Unsaved description' } });
  routeId.set('id', 'second');
  await waitFor(() => expect(description.value).toBe('Second description'));
  await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
  await waitFor(() => expect(updateAgent).toHaveBeenCalledOnce());
  expect(updateAgent).toHaveBeenCalledWith(expect.objectContaining({ id: 'second' }));
});

test('new agent requires a non-reserved name and description before creating', async () => {
  const { createAgent } = mount(NewAgent);
  await fireEvent.click(screen.getByRole('button', { name: 'Create agent' }));
  expect(await screen.findByText('Name is required.')).toBeTruthy();
  expect(screen.getByText('Required by opencode.')).toBeTruthy();
  expect(createAgent).not.toHaveBeenCalled();

  await fireEvent.input(screen.getByRole('textbox', { name: /Name/ }), { target: { value: 'build' } });
  await fireEvent.input(screen.getByRole('textbox', { name: /Description/ }), { target: { value: 'Custom helper' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Create agent' }));
  expect(await screen.findByText('This name is reserved by a built-in agent.')).toBeTruthy();
  expect(createAgent).not.toHaveBeenCalled();
});

test('new agent sends source, storage, model variant and permission rules', async () => {
  const { createAgent } = mount(NewAgent);
  await fireEvent.input(screen.getByRole('textbox', { name: /Name/ }), { target: { value: '  helper  ' } });
  await fireEvent.input(screen.getByRole('textbox', { name: /Description/ }), { target: { value: 'Custom helper' } });
  await fireEvent.click(screen.getByRole('radio', { name: 'Inline' }));
  await choose('Model', 'Fast');
  await choose('Variant', 'quick');
  await fireEvent.input(screen.getByLabelText('Permissions JSON'), {
    target: { value: JSON.stringify([rule]) },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Create agent' }));

  await waitFor(() => expect(createAgent).toHaveBeenCalledOnce());
  expect(createAgent).toHaveBeenCalledWith({
    id: 'helper',
    source: 'inline',
    storage: 'inline',
    mutation: {
      fields: {
        model: 'demo/fast#quick',
        mode: undefined,
        description: 'Custom helper',
        disabled: false,
        hidden: false,
        color: undefined,
        steps: undefined,
        prompt: undefined,
        permissions: [rule],
      },
    },
  });
  await waitFor(() => expect(goto).toHaveBeenCalledWith('/agents'));
});

test('edit loads the selected source snapshot and clears removed model and prompt', async () => {
  const agent: AgentDefinition = {
    id: 'helper',
    source: 'both',
    sources: [
      {
        storage: 'inline',
        raw: { description: 'Inline description', model: 'demo/fast#quick', prompt: 'Inline prompt' },
        prompt: 'Inline prompt',
        fields: { description: 'Inline description', model: 'demo/fast#quick', prompt: 'Inline prompt' },
      },
      {
        storage: 'global_markdown',
        raw: '---\ndescription: Markdown description\nmodel: demo/fast\n---\n\nMarkdown prompt',
        prompt: 'Markdown prompt',
        fields: { description: 'Markdown description', model: 'demo/fast' },
      },
    ],
  };
  const { updateAgent } = mount(EditAgent, [agent]);
  await waitFor(() =>
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: /Description/ }).value).toBe('Inline description'),
  );
  await fireEvent.input(screen.getByRole('textbox', { name: /Description/ }), { target: { value: 'Unsaved edit' } });
  await choose('Edit source', 'Global Markdown');
  await waitFor(() =>
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: /Description/ }).value).toBe('Markdown description'),
  );
  expect(screen.getByLabelText<HTMLTextAreaElement>('Prompt').value).toBe('Markdown prompt');

  await choose('Model', 'Clear / inherit');
  await fireEvent.input(screen.getByLabelText('Prompt'), { target: { value: '' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
  await waitFor(() => expect(updateAgent).toHaveBeenCalledOnce());
  expect(updateAgent).toHaveBeenCalledWith({
    id: 'helper',
    source: 'both',
    storage: 'global_markdown',
    mutation: {
      fields: {
        model: undefined,
        mode: undefined,
        description: 'Markdown description',
        disabled: false,
        hidden: false,
        color: undefined,
        steps: undefined,
        prompt: undefined,
      },
      clearFields: ['model', 'prompt'],
    },
  });
  await waitFor(() => expect(goto).toHaveBeenCalledWith('/agents'));
});

test('edit restores the source snapshot after formResetVersion changes', async () => {
  const agent: AgentDefinition = {
    id: 'helper',
    source: 'inline',
    sources: [
      { storage: 'inline', raw: { description: 'Saved description' }, fields: { description: 'Saved description' } },
    ],
  };
  const { reset, updateAgent } = mount(EditAgent, [agent]);
  const description = screen.getByRole<HTMLInputElement>('textbox', { name: /Description/ });
  await waitFor(() => expect(description.value).toBe('Saved description'));
  await fireEvent.input(description, { target: { value: 'Unsaved description' } });
  expect(description.value).toBe('Unsaved description');
  reset.set('version', 1);
  await waitFor(() => expect(description.value).toBe('Saved description'));
  expect(updateAgent).not.toHaveBeenCalled();
});

test('edit source picker offers one Markdown choice for multiple alias previews', async () => {
  const agent: AgentDefinition = {
    id: 'helper',
    source: 'markdown',
    sources: [
      {
        storage: 'global_markdown',
        path: '/global/agent/helper.md',
        raw: '---\ndescription: Singular alias\n---\n\nFirst prompt',
        fields: { description: 'Singular alias' },
        prompt: 'First prompt',
      },
      {
        storage: 'global_markdown',
        path: '/global/agents/helper.md',
        raw: '---\ndescription: Plural alias\n---\n\nSecond prompt',
        fields: { description: 'Plural alias' },
        prompt: 'Second prompt',
      },
    ],
  };
  const { updateAgent } = mount(EditAgent, [agent]);
  await waitFor(() =>
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: /Description/ }).value).toBe('Singular alias'),
  );
  await fireEvent.click(screen.getByLabelText('Edit source'));
  const listbox = await screen.findByRole('listbox');
  expect(within(listbox).getAllByRole('option', { name: 'Global Markdown' })).toHaveLength(1);
  expect(within(listbox).queryByRole('option', { name: 'Inline' })).toBeNull();
  await fireEvent.click(within(listbox).getByRole('option', { name: 'Global Markdown' }));
  expect(screen.getByLabelText<HTMLTextAreaElement>('Prompt').value).toBe('First prompt');
  expect(agent.sources).toHaveLength(2);
  expect(updateAgent).not.toHaveBeenCalled();
});
