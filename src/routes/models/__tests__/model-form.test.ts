import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ModelDef } from '$src/types/models.js';
import type { ProviderDef } from '$src/types/providers.js';
import ModelForm from '../ModelForm.svelte';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const scrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});
afterAll(() => {
  if (scrollIntoView) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scrollIntoView);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

const provider: ProviderDef = { name: 'example', models: {} };
const config = (overrides: Record<string, unknown> = {}) =>
  ({
    preferences: { locale: 'en' },
    providers: [provider],
    agents: [],
    saving: false,
    formResetVersion: 0,
    catalogLoading: false,
    catalogModels: () => [],
    models: () => [],
    ...overrides,
  }) as unknown as ConfigStore;

async function selectProvider() {
  await fireEvent.click(screen.getByRole('button', { name: 'Provider' }));
  await fireEvent.click(screen.getByRole('option', { name: 'example' }));
}

test('new model requires an ID before saving', async () => {
  const onSave = vi.fn();

  render(
    ModelForm,
    { mode: 'new', providers: [provider], onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );
  await selectProvider();

  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  expect(onSave).not.toHaveBeenCalled();
  expect(screen.getByRole('textbox', { name: 'Model ID' }).getAttribute('required')).not.toBeNull();
});

test('invalid model token limits block saving and show a field error', async () => {
  const onSave = vi.fn();

  render(
    ModelForm,
    { mode: 'new', providers: [provider], onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );
  await selectProvider();
  await fireEvent.input(screen.getByRole('textbox', { name: 'Model ID' }), { target: { value: 'model-a' } });
  await fireEvent.input(screen.getByRole('combobox', { name: 'Context' }), { target: { value: '-1' } });

  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  expect(onSave).not.toHaveBeenCalled();
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Context' }).getAttribute('aria-invalid')).toBe('true'),
  );
  expect(screen.getAllByRole('alert').some((alert) => alert.textContent?.includes('non-negative whole number'))).toBe(
    true,
  );
});

test('new model maps JSON and variants while retaining capability defaults', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const { container } = render(
    ModelForm,
    { mode: 'new', providers: [provider], onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );

  await selectProvider();
  await fireEvent.input(screen.getByRole('textbox', { name: 'Model ID' }), { target: { value: '  model-a  ' } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Family' }), { target: { value: '  family-a  ' } });
  await fireEvent.input(container.querySelector('#model-settings') as HTMLTextAreaElement, {
    target: { value: '{"temperature":0.5}' },
  });
  await fireEvent.input(container.querySelector('#model-headers') as HTMLTextAreaElement, {
    target: { value: '{"x-test":"yes"}' },
  });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Variant name' }), { target: { value: '  fast  ' } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Variant options JSON' }), {
    target: { value: '{"settings":{"temperature":0.2}}' },
  });

  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
  expect(onSave).toHaveBeenCalledWith('example', {
    id: 'model-a',
    family: 'family-a',
    capabilities: { tools: true, input: ['text', 'image', 'pdf'], output: ['text', 'image', 'pdf'] },
    settings: { temperature: 0.5 },
    headers: { 'x-test': 'yes' },
    variants: [{ id: 'fast', settings: { temperature: 0.2 } }],
  });
});

test('editing a model initializes saved fields and omits absent optional blocks', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const initial: ModelDef = {
    id: 'existing',
    family: 'old-family',
    variants: [{ id: 'balanced', body: { speed: 1 } }],
  };

  render(
    ModelForm,
    { mode: 'edit', defaultProviderId: 'example', initial, onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );

  await waitFor(() =>
    expect((screen.getByRole('textbox', { name: 'Family' }) as HTMLInputElement).value).toBe('old-family'),
  );
  expect((screen.getByRole('textbox', { name: 'Model ID' }) as HTMLInputElement).disabled).toBe(true);
  expect((screen.getByRole('textbox', { name: 'Variant name' }) as HTMLInputElement).value).toBe('balanced');
  await fireEvent.input(screen.getByRole('textbox', { name: 'Family' }), { target: { value: '  updated  ' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
  expect(onSave).toHaveBeenCalledWith('example', {
    id: 'existing',
    family: 'updated',
    variants: [{ id: 'balanced', body: { speed: 1 } }],
  });
});

test('editing an explicit empty capabilities block preserves its false tools value', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);

  render(
    ModelForm,
    { mode: 'edit', defaultProviderId: 'example', initial: { id: 'existing', capabilities: {} }, onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );
  await waitFor(() =>
    expect((screen.getByRole('textbox', { name: 'Model ID' }) as HTMLInputElement).value).toBe('existing'),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
  expect(onSave).toHaveBeenCalledWith('example', { id: 'existing', capabilities: { tools: false } });
});

test('initial.limit.input null renders empty string and modelToFormValues preserves 0 for cost and limit siblings', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const initialWithZeroAndNull: ModelDef = {
    id: 'zero-model',

    cost: {
      input: 0,
      output: 0,

      cache: {
        read: 0,
        write: 0,
      },
    },

    limit: {
      context: 0,
      output: 0,
      input: null,
    },
  };

  render(
    ModelForm,
    {
      mode: 'edit',
      defaultProviderId: 'example',
      initial: initialWithZeroAndNull,
      onSave,
    },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );

  const limitInput = screen.getByRole('combobox', { name: 'Input' }) as HTMLInputElement;

  expect(limitInput.value).toBe('');
  expect(limitInput.value).not.toBe('null');

  const costInput = screen.getByRole('spinbutton', { name: 'Input' }) as HTMLInputElement;

  expect(costInput.value).toBe('0');

  const limitContext = screen.getByRole('combobox', { name: 'Context' }) as HTMLInputElement;

  expect(limitContext.value).toBe('0');
});

test('editing a model with multiline variant json and backend-shaped nullable fields saves successfully', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const multilineVariantJson = `{\n  "settings": {\n    "reasoningEffort": "high"\n  }\n}`;
  const initial = {
    id: 'backend-variant-model',
    name: null,
    family: 'custom-family',
    disabled: null,
    capabilities: {
      tools: true,
      input: null,
      output: null,
    },
    cost: {
      input: 0,
      output: 0,
      cache: null,
    },
    limit: {
      context: 128000,
      output: 4096,
      input: null,
    },
    settings: null,
    headers: null,
    variants: [
      {
        id: 'high',
        settings: { reasoningEffort: 'high' },
      },
    ],
  } as unknown as ModelDef;

  render(
    ModelForm,
    { mode: 'edit', defaultProviderId: 'example', initial, onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );

  await waitFor(() =>
    expect((screen.getByRole('textbox', { name: 'Variant name' }) as HTMLInputElement).value).toBe('high'),
  );

  const variantJsonInput = screen.getByRole('textbox', { name: 'Variant options JSON' }) as HTMLTextAreaElement;
  await fireEvent.input(variantJsonInput, { target: { value: multilineVariantJson } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
  expect(onSave).toHaveBeenCalledWith('example', {
    id: 'backend-variant-model',
    family: 'custom-family',
    capabilities: { tools: true },
    cost: { input: 0, output: 0 },
    limit: { context: 128000, output: 4096 },
    variants: [
      {
        id: 'high',
        settings: { reasoningEffort: 'high' },
      },
    ],
  });
});

test('invalid Options JSON errors only its own field and does not blame valid variant', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const initial: ModelDef = {
    id: 'field-isolation-model',
    variants: [{ id: 'high', settings: { reasoningEffort: 'high' } }],
  };

  const { container } = render(
    ModelForm,
    { mode: 'edit', defaultProviderId: 'example', initial, onSave },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );

  const settingsArea = container.querySelector('#model-settings') as HTMLTextAreaElement;
  await fireEvent.input(settingsArea, { target: { value: '{invalid_json' } });

  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  expect(onSave).not.toHaveBeenCalled();
  await waitFor(() => {
    const settingsError = document.getElementById('model-settings-error');
    expect(settingsError).not.toBeNull();
    expect(settingsError?.textContent).toContain('valid JSON object');
  });

  // Variant row has NO error
  expect(document.getElementById('variant-0-error')).toBeNull();
});

test('saving realistic backend model keeps nested null in settings and omits absent optional blocks', async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const initial: ModelDef = {
    id: 'backend-model',
    settings: {
      reasoning: null,
      temperature: 0,
    },
    cost: {
      input: 0,
      output: 0,
    },
    limit: {
      context: 128000,
      output: 4096,
      input: null,
    },
  };

  const { container } = render(
    ModelForm,
    {
      mode: 'edit',
      defaultProviderId: 'example',
      initial,
      onSave,
    },
    { wrapper: Harness, wrapperProps: { config: config() } },
  );

  const settingsEl = container.querySelector('#model-settings') as HTMLTextAreaElement;
  expect(settingsEl.value).toContain('"reasoning": null');
  expect(settingsEl.value).toContain('"temperature": 0');

  // limit.input was null, rendered as empty in UI, omitted on save
  const limitInput = screen.getByRole('combobox', { name: 'Input' }) as HTMLInputElement;
  expect(limitInput.value).toBe('');

  await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

  await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
  expect(onSave).toHaveBeenCalledWith('example', {
    id: 'backend-model',
    settings: {
      reasoning: null,
      temperature: 0,
    },
    cost: {
      input: 0,
      output: 0,
    },
    limit: {
      context: 128000,
      output: 4096,
    },
  });
});
