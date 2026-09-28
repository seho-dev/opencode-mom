import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ModelDef } from '$src/types/models.js';
import type { ProviderDef } from '$src/types/providers.js';
import ModelForm from './ModelForm.svelte';

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
  }) as ConfigStore;

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
