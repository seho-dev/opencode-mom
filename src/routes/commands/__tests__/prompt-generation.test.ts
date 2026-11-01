import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page as routePage } from '$app/state';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { SlashCommandEntry } from '$src/types/commands.js';
import NewCommand from '../new/+page.svelte';

vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify: () => void;
  const track = createSubscriber((update) => {
    notify = update;
  });
  let params = {};
  let url = new URL('http://localhost/commands/new');
  return {
    page: {
      get url() {
        track();
        return url;
      },
      set url(value: URL) {
        url = value;
        notify?.();
      },
      get params() {
        track();
        return params;
      },
      set params(value: Record<string, unknown>) {
        params = value;
        notify?.();
      },
    },
  };
});

const page = routePage as unknown as {
  params: { id?: string };
  url: URL;
};

const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;

beforeEach(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
  vi.mocked(goto).mockClear();
  page.url = new URL('http://localhost/commands/new');
});

afterEach(() => {
  if (originalScrollIntoView) HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
  else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});

const generatedCommandContent = `---
description: Run automated test suite
agent: tester
---

Run npm test
$ARGUMENTS
`;

function createCommandTestStore(overrides: Record<string, unknown> = {}) {
  const createSlashCommand = vi.fn().mockResolvedValue({
    id: 'test',
    name: 'test',
    content: generatedCommandContent,
  } satisfies Partial<SlashCommandEntry>);
  const updateSlashCommand = vi.fn().mockResolvedValue(undefined);
  const generatePrompt = vi.fn().mockResolvedValue(generatedCommandContent);

  const store = makeScopedConfig({
    catalogModels: vi.fn().mockReturnValue([]),
    models: vi.fn().mockReturnValue([]),
    catalogLoading: false,
    serviceTargetVersion: 0,
    createSlashCommand,
    updateSlashCommand,
    generatePrompt,
    ...overrides,
  });

  return { store, createSlashCommand, updateSlashCommand, generatePrompt };
}

test('command form generates preview and applies into content field preserving structure without auto-saving', async () => {
  const { store, createSlashCommand, updateSlashCommand, generatePrompt } = createCommandTestStore();

  render(
    NewCommand,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config: store },
    },
  );

  const contentTextarea = screen.getByLabelText(en['commands.content']) as HTMLTextAreaElement;
  const initialContent = `---\ndescription: Initial\n---\n\n$ARGUMENTS\n`;
  await fireEvent.input(contentTextarea, { target: { value: initialContent } });

  const openBtn = screen.getByRole('button', { name: en['promptGenerator.open'] });
  await fireEvent.click(openBtn);

  expect(screen.getByRole('heading', { name: en['promptGenerator.commandTitle'] })).toBeTruthy();

  const instructions = screen.getByLabelText(en['promptGenerator.instructions']);
  await fireEvent.input(instructions, { target: { value: 'Add tester agent and npm test command' } });

  const generateBtn = screen.getByRole('button', { name: en['promptGenerator.generate'] });
  await fireEvent.click(generateBtn);

  expect(generatePrompt).toHaveBeenCalledExactlyOnceWith({
    kind: 'command',
    preset: 'improve',
    instructions: 'Add tester agent and npm test command',
    currentPrompt: initialContent,
    model: undefined,
    variant: undefined,
  });

  await waitFor(() => {
    expect(screen.getByLabelText(en['promptGenerator.preview'])).toBeTruthy();
  });

  const applyBtn = screen.getByRole('button', { name: en['promptGenerator.apply'] });
  await fireEvent.click(applyBtn);

  expect(contentTextarea.value).toBe(generatedCommandContent);
  expect(createSlashCommand).not.toHaveBeenCalled();
  expect(updateSlashCommand).not.toHaveBeenCalled();

  const nameInput = screen.getByLabelText(en['commands.name']);
  await fireEvent.input(nameInput, { target: { value: 'run-tests' } });

  const saveBtn = screen.getByRole('button', { name: en['configuration.save'] });
  await fireEvent.click(saveBtn);

  expect(createSlashCommand).toHaveBeenCalledExactlyOnceWith(
    { name: 'run-tests', content: generatedCommandContent },
    null,
  );
});

test('readonly command form disables prompt generator trigger', async () => {
  page.url = new URL('http://localhost/commands/new?readonly=1');
  const { store, generatePrompt } = createCommandTestStore();

  render(
    NewCommand,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config: store },
    },
  );

  const triggerBtn = screen.getByRole('button', { name: en['promptGenerator.open'] }) as HTMLButtonElement;
  expect(triggerBtn.disabled).toBe(true);

  await fireEvent.click(triggerBtn);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(generatePrompt).not.toHaveBeenCalled();
});
