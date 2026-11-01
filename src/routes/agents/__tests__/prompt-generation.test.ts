import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AgentDefinition } from '$src/types/agents.js';
import EditAgent from '../[id]/edit/+page.svelte';

let currentRouteId = 'reviewer';
let currentUrl = new URL('http://localhost/agents/reviewer/edit');

vi.mock('$app/navigation', () => ({
  goto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$app/state', () => ({
  page: {
    get params() {
      return { id: currentRouteId };
    },
    get url() {
      return currentUrl;
    },
  },
}));

const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;

beforeEach(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
  currentRouteId = 'reviewer';
  currentUrl = new URL('http://localhost/agents/reviewer/edit');
});

afterEach(() => {
  if (originalScrollIntoView) HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
  else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});

const editableAgent: AgentDefinition = {
  id: 'reviewer',
  description: 'Reviews code',
  prompt: 'Initial agent prompt',
  source: 'inline',
  storage: 'inline',
  sources: [
    {
      storage: 'inline',
      content: JSON.stringify({
        description: 'Reviews code',
        prompt: 'Initial agent prompt',
      }),
      raw: {
        description: 'Reviews code',
        prompt: 'Initial agent prompt',
      },
      prompt: 'Initial agent prompt',
      fields: {
        description: 'Reviews code',
        prompt: 'Initial agent prompt',
      },
    },
  ],
};

const builtinAgent: AgentDefinition = {
  id: 'build',
  description: 'Built-in build agent',
  prompt: 'Built-in prompt',
  source: 'inline',
  storage: 'inline',
  sources: [
    {
      storage: 'inline',
      content: JSON.stringify({ description: 'Built-in build agent' }),
      raw: { description: 'Built-in build agent' },
      fields: { description: 'Built-in build agent' },
    },
  ],
};

function createAgentTestConfig(agents: AgentDefinition[] = [editableAgent]) {
  const updateAgent = vi.fn().mockResolvedValue(editableAgent);
  const createAgent = vi.fn().mockResolvedValue(editableAgent);
  const generatePrompt = vi.fn().mockResolvedValue('Replaced agent prompt from AI');

  const config = {
    agents,
    saving: false,
    loading: false,
    catalogLoading: false,
    serviceTargetVersion: 0,
    catalogModels: vi.fn().mockReturnValue([]),
    models: vi.fn().mockReturnValue([]),
    updateAgent,
    createAgent,
    generatePrompt,
    preferences: { locale: 'en', theme: 'dark' },
  } as unknown as ConfigStore;

  return { config, updateAgent, createAgent, generatePrompt };
}

test('agent form generates preview and applies into prompt field without auto-saving', async () => {
  const { config, updateAgent, createAgent, generatePrompt } = createAgentTestConfig();

  render(
    EditAgent,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  const promptTextarea = (await screen.findByLabelText(en['agents.prompt'])) as HTMLTextAreaElement;
  expect(promptTextarea.value).toBe('Initial agent prompt');

  const openBtn = screen.getByRole('button', { name: en['promptGenerator.open'] });
  await fireEvent.click(openBtn);

  expect(screen.getByRole('heading', { name: en['promptGenerator.agentTitle'] })).toBeTruthy();

  const instructions = screen.getByLabelText(en['promptGenerator.instructions']);
  await fireEvent.input(instructions, { target: { value: 'Focus on performance and security' } });

  const generateBtn = screen.getByRole('button', { name: en['promptGenerator.generate'] });
  await fireEvent.click(generateBtn);

  expect(generatePrompt).toHaveBeenCalledExactlyOnceWith({
    kind: 'agent',
    preset: 'improve',
    instructions: 'Focus on performance and security',
    currentPrompt: 'Initial agent prompt',
    model: undefined,
    variant: undefined,
  });

  await waitFor(() => {
    expect(screen.getByLabelText(en['promptGenerator.preview'])).toBeTruthy();
  });

  const applyBtn = screen.getByRole('button', { name: en['promptGenerator.apply'] });
  await fireEvent.click(applyBtn);

  expect(promptTextarea.value).toBe('Replaced agent prompt from AI');
  expect(updateAgent).not.toHaveBeenCalled();
  expect(createAgent).not.toHaveBeenCalled();
});

test('built-in agent disables prompt generator trigger', async () => {
  currentRouteId = 'build';
  currentUrl = new URL('http://localhost/agents/build/edit');
  const { config, generatePrompt } = createAgentTestConfig([builtinAgent]);

  render(
    EditAgent,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  await waitFor(() => {
    expect(screen.getByText(en['agents.builtinReadOnly'])).toBeTruthy();
  });

  const promptGeneratorBtn = screen.queryByRole('button', { name: en['promptGenerator.open'] });
  expect(promptGeneratorBtn).toBeNull();
  expect(generatePrompt).not.toHaveBeenCalled();
});
