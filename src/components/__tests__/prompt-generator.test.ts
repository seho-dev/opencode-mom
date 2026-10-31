import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { createSubscriber } from 'svelte/reactivity';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { PromptGenerationRequest } from '$src/types/prompt-generation.js';
import PromptGenerator from '../PromptGenerator.svelte';

let currentUrl = new URL('http://localhost/agents/new');
let notifyUrl: (() => void) | undefined;

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  const track = createSubscriber((update) => {
    notifyUrl = update;
  });
  return {
    page: {
      get url() {
        track();
        return currentUrl;
      },
      set url(value: URL) {
        currentUrl = value;
        notifyUrl?.();
      },
    },
  };
});

const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;

beforeEach(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
  currentUrl = new URL('http://localhost/agents/new');
});

afterEach(() => {
  if (originalScrollIntoView) HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
  else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

interface MockConfigHelper {
  config: ConfigStore;
  setServiceTargetVersion(version: number): void;
}

function createMockConfig(overrides: Partial<ConfigStore> = {}): MockConfigHelper {
  let targetVersion = overrides.serviceTargetVersion ?? 0;
  let notifyTarget: (() => void) | undefined;
  const trackTarget = createSubscriber((update) => {
    notifyTarget = update;
  });

  const config = {
    catalogModels: vi.fn().mockReturnValue([
      {
        ref: 'demo/fast',
        providerId: 'demo',
        modelId: 'fast',
        name: 'Fast',
        isCustom: true,
        variants: [{ id: 'quick' }],
      },
    ]),
    models: vi.fn().mockReturnValue([]),
    catalogLoading: false,
    generatePrompt: vi.fn().mockResolvedValue('Generated preview content'),
    preferences: { locale: 'en', theme: 'dark' },
    get serviceTargetVersion() {
      trackTarget();
      return targetVersion;
    },
    ...overrides,
  } as unknown as ConfigStore;

  return {
    config,
    setServiceTargetVersion(v: number) {
      targetVersion = v;
      notifyTarget?.();
    },
  };
}

test('empty input disables generate; filling requirements allows generating preview and applying replaces draft', async () => {
  const onapply = vi.fn();
  const { config } = createMockConfig();
  const pendingGen = deferred<string>();
  vi.mocked(config.generatePrompt).mockImplementation(() => pendingGen.promise);

  render(
    PromptGenerator,
    { kind: 'agent', value: '', disabled: false, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  const trigger = screen.getByRole('button', { name: en['promptGenerator.open'] });
  expect((trigger as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(trigger);

  expect(screen.getByRole('heading', { name: en['promptGenerator.agentTitle'] })).toBeTruthy();
  const generateBtn = screen.getByRole('button', { name: en['promptGenerator.generate'] }) as HTMLButtonElement;
  expect(generateBtn.disabled).toBe(true);

  const instructionsInput = screen.getByLabelText(en['promptGenerator.instructions']);
  await fireEvent.input(instructionsInput, { target: { value: 'Create an autonomous test assistant' } });
  expect(generateBtn.disabled).toBe(false);

  await fireEvent.click(generateBtn);
  expect(config.generatePrompt).toHaveBeenCalledExactlyOnceWith({
    kind: 'agent',
    preset: 'generate',
    instructions: 'Create an autonomous test assistant',
    currentPrompt: '',
    model: undefined,
    variant: undefined,
  } satisfies PromptGenerationRequest);

  expect(generateBtn.getAttribute('aria-busy')).toBe('true');
  expect(screen.getByText(en['promptGenerator.generating'])).toBeTruthy();
  expect(screen.getByText(en['promptGenerator.waiting'])).toBeTruthy();

  await fireEvent.click(generateBtn);
  expect(config.generatePrompt).toHaveBeenCalledTimes(1);

  pendingGen.resolve('New autonomous system prompt draft');
  await waitFor(() => {
    expect(screen.getByLabelText(en['promptGenerator.preview'])).toBeTruthy();
  });

  const preview = screen.getByLabelText(en['promptGenerator.preview']) as HTMLTextAreaElement;
  expect(preview.readOnly).toBe(true);
  expect(preview.value).toBe('New autonomous system prompt draft');
  expect(screen.getByText(en['promptGenerator.ready'])).toBeTruthy();
  expect(onapply).not.toHaveBeenCalled();

  const applyBtn = screen.getByRole('button', { name: en['promptGenerator.apply'] });
  await fireEvent.click(applyBtn);

  expect(onapply).toHaveBeenCalledExactlyOnceWith('New autonomous system prompt draft');
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('existing draft defaults to improve preset and supports explicit model and variant selection', async () => {
  const onapply = vi.fn();
  const { config } = createMockConfig();
  vi.mocked(config.generatePrompt).mockResolvedValue('Improved command content');

  render(
    PromptGenerator,
    { kind: 'command', value: 'git status --short', disabled: false, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  expect(screen.getByRole('heading', { name: en['promptGenerator.commandTitle'] })).toBeTruthy();

  const generateBtn = screen.getByRole('button', { name: en['promptGenerator.generate'] }) as HTMLButtonElement;
  expect(generateBtn.disabled).toBe(false);

  const modelTrigger = screen.getByLabelText(en['common.model']);
  await fireEvent.click(modelTrigger);
  const modelOption = screen.getByRole('option', { name: /Fast/ });
  await fireEvent.click(modelOption);

  const variantTrigger = screen.getByLabelText(en['agents.variant']);
  await fireEvent.click(variantTrigger);
  const variantOption = screen.getByRole('option', { name: 'quick' });
  await fireEvent.click(variantOption);

  await fireEvent.click(generateBtn);

  expect(config.generatePrompt).toHaveBeenCalledExactlyOnceWith({
    kind: 'command',
    preset: 'improve',
    instructions: '',
    currentPrompt: 'git status --short',
    model: 'demo/fast',
    variant: 'quick',
  } satisfies PromptGenerationRequest);

  await waitFor(() => {
    expect(screen.getByLabelText(en['promptGenerator.preview'])).toBeTruthy();
  });
  expect(onapply).not.toHaveBeenCalled();
});

test('failure preserves input without retry or apply', async () => {
  const onapply = vi.fn();
  const { config } = createMockConfig({
    generatePrompt: vi.fn().mockRejectedValue(new Error('service offline')),
  });

  render(
    PromptGenerator,
    { kind: 'agent', value: '', disabled: false, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  const instructionsInput = screen.getByLabelText(en['promptGenerator.instructions']) as HTMLTextAreaElement;
  await fireEvent.input(instructionsInput, { target: { value: 'Draft with sensitive instructions' } });

  const generateBtn = screen.getByRole('button', { name: en['promptGenerator.generate'] });
  await fireEvent.click(generateBtn);

  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toContain(en['promptGenerator.failed']);
  });

  expect(instructionsInput.value).toBe('Draft with sensitive instructions');
  expect(config.generatePrompt).toHaveBeenCalledTimes(1);
  expect(onapply).not.toHaveBeenCalled();
  expect(screen.queryByLabelText(en['promptGenerator.preview'])).toBeNull();
});

test('closing dialog discards late results and reopen is not contaminated by stale reply', async () => {
  const onapply = vi.fn();
  const { config } = createMockConfig();
  const pendingGen = deferred<string>();
  vi.mocked(config.generatePrompt).mockImplementation(() => pendingGen.promise);

  render(
    PromptGenerator,
    { kind: 'agent', value: '', disabled: false, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  await fireEvent.input(screen.getByLabelText(en['promptGenerator.instructions']), {
    target: { value: 'Instructions for session 1' },
  });

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.generate'] }));
  expect(config.generatePrompt).toHaveBeenCalledTimes(1);

  await fireEvent.click(screen.getByRole('button', { name: en['common.close'] }));
  expect(screen.queryByRole('dialog')).toBeNull();

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  expect(screen.getByRole('heading', { name: en['promptGenerator.agentTitle'] })).toBeTruthy();
  expect((screen.getByLabelText(en['promptGenerator.instructions']) as HTMLTextAreaElement).value).toBe('');

  pendingGen.resolve('Late arriving result from session 1');

  await waitFor(() => {
    expect(screen.queryByLabelText(en['promptGenerator.preview'])).toBeNull();
  });
  expect(onapply).not.toHaveBeenCalled();
});

test('service target change invalidates dialog and blocks apply', async () => {
  const onapply = vi.fn();
  const { config, setServiceTargetVersion } = createMockConfig({
    generatePrompt: vi.fn().mockResolvedValue('Generated ready to apply'),
  });

  render(
    PromptGenerator,
    { kind: 'agent', value: 'initial prompt', disabled: false, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.generate'] }));

  await waitFor(() => {
    expect(screen.getByLabelText(en['promptGenerator.preview'])).toBeTruthy();
  });

  const applyBtn = screen.getByRole('button', { name: en['promptGenerator.apply'] }) as HTMLButtonElement;
  expect(applyBtn.disabled).toBe(false);

  setServiceTargetVersion(1);

  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toContain(en['promptGenerator.contextChanged']);
  });
  expect(applyBtn.disabled).toBe(true);

  await fireEvent.click(applyBtn);
  expect(onapply).not.toHaveBeenCalled();
});

test('unmounting while generation is in flight cleans up safely without unhandled errors or applying', async () => {
  const onapply = vi.fn();
  const { config } = createMockConfig();
  const pendingGen = deferred<string>();
  vi.mocked(config.generatePrompt).mockImplementation(() => pendingGen.promise);

  const { unmount } = render(
    PromptGenerator,
    { kind: 'agent', value: '', disabled: false, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  await fireEvent.input(screen.getByLabelText(en['promptGenerator.instructions']), {
    target: { value: 'Instructions before unmount' },
  });

  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.generate'] }));
  expect(config.generatePrompt).toHaveBeenCalledTimes(1);

  unmount();

  pendingGen.resolve('Result for unmounted component');
  await tick();

  expect(onapply).not.toHaveBeenCalled();
});

test('disabled prop prevents opening dialog and triggering prompt generation', async () => {
  const onapply = vi.fn();
  const { config } = createMockConfig();

  render(
    PromptGenerator,
    { kind: 'agent', value: 'hello', disabled: true, onapply },
    {
      wrapper: Harness,
      wrapperProps: { config },
    },
  );

  const trigger = screen.getByRole('button', { name: en['promptGenerator.open'] }) as HTMLButtonElement;
  expect(trigger.disabled).toBe(true);

  await fireEvent.click(trigger);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(config.generatePrompt).not.toHaveBeenCalled();
});
