import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ProviderDef } from '$src/types/providers.js';
import EditProviderPage from '../[id]/edit/+page.svelte';
import NewProviderPage from '../new/+page.svelte';

const { goto, params } = vi.hoisted(() => ({ goto: vi.fn(), params: { id: 'existing' } }));
vi.mock('$app/navigation', () => ({ goto }));
vi.mock('$app/state', () => ({ page: { params } }));

function configWith(providers: ProviderDef[] = []) {
  const createProvider = vi.fn().mockResolvedValue(undefined);
  const updateProvider = vi.fn().mockResolvedValue(undefined);
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    providers,
    loading: false,
    saving: false,
    createProvider,
    updateProvider,
  } as unknown as ConfigStore;
  return { config, createProvider, updateProvider };
}

function mount(component: typeof NewProviderPage | typeof EditProviderPage, config: ConfigStore) {
  return render(component, {}, { wrapper: Harness, wrapperProps: { config } });
}

beforeEach(() => {
  goto.mockReset();
  params.id = 'existing';
});

test('new provider requires an NPM adapter and rejects invalid headers', async () => {
  const { config, createProvider } = configWith();
  mount(NewProviderPage, config);

  await fireEvent.input(screen.getByRole('textbox', { name: 'Name' }), { target: { value: 'example' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save provider' }));
  expect(await screen.findByText('Fix the highlighted fields before saving.')).toBeTruthy();
  const npmAdapter = screen.getByRole('combobox', { name: 'Adapter package' });
  expect(npmAdapter.getAttribute('aria-invalid')).toBe('true');
  expect(npmAdapter.getAttribute('aria-describedby')).toBe('provider-npm-hint provider-npm-error');
  expect(document.getElementById('provider-npm-error')?.textContent?.trim()).toBe(
    'Fix the highlighted fields before saving.',
  );
  expect(createProvider).not.toHaveBeenCalled();

  await fireEvent.input(screen.getByRole('combobox', { name: 'Adapter package' }), {
    target: { value: '@opencode/ai/providers/openai' },
  });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Headers' }), { target: { value: '{bad' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save provider' }));
  expect(await screen.findByText('Must be a valid JSON object, not an array or scalar.')).toBeTruthy();
  expect(createProvider).not.toHaveBeenCalled();

  await fireEvent.input(screen.getByRole('textbox', { name: 'Headers' }), {
    target: { value: '{"X-Count":1}' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save provider' }));
  expect(await screen.findByText('Header values must be strings.')).toBeTruthy();
  expect(createProvider).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
});

test('new provider parses JSON headers and navigates after saving', async () => {
  const { config, createProvider } = configWith();
  mount(NewProviderPage, config);

  await fireEvent.input(screen.getByRole('textbox', { name: 'Name' }), { target: { value: '  example  ' } });
  await fireEvent.input(screen.getByRole('combobox', { name: 'Adapter package' }), {
    target: { value: '@opencode/ai/providers/openai' },
  });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Base URL' }), {
    target: { value: 'https://api.example.com/v1' },
  });
  await fireEvent.input(screen.getByLabelText('API key'), { target: { value: 'secret' } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Headers' }), {
    target: { value: '{"Authorization":"Bearer token"}' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save provider' }));

  await waitFor(() =>
    expect(createProvider).toHaveBeenCalledWith({
      name: 'example',
      package: '@opencode/ai/providers/openai',
      settings: { apiKey: 'secret', baseURL: 'https://api.example.com/v1' },
      headers: { Authorization: 'Bearer token' },
      models: {},
    }),
  );
  expect(goto).toHaveBeenCalledWith('/providers');
});

test('edit keeps name immutable and existing models, and allows clearing package and key', async () => {
  const source: ProviderDef = {
    name: 'existing',
    package: '@opencode/ai/providers/openai',
    settings: { apiKey: 'old-secret', baseURL: 'https://old.example.com' },
    headers: { Old: 'value' },
    models: { 'my-model': { id: 'my-model', name: 'My model' } },
  };
  const { config, updateProvider } = configWith([source]);
  mount(EditProviderPage, config);

  const name = screen.getByRole('textbox', { name: 'Name' }) as HTMLInputElement;
  expect(name.disabled).toBe(true);
  expect(name.value).toBe('existing');
  expect((screen.getByLabelText('API key') as HTMLInputElement).value).toBe('old-secret');

  await fireEvent.input(screen.getByRole('combobox', { name: 'Adapter package' }), { target: { value: '' } });
  await fireEvent.input(screen.getByLabelText('API key'), { target: { value: '' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));

  await waitFor(() =>
    expect(updateProvider).toHaveBeenCalledWith({
      ...source,
      package: undefined,
      settings: { apiKey: undefined, baseURL: 'https://old.example.com' },
    }),
  );
  expect(goto).toHaveBeenCalledWith('/providers');
});

test('provider combobox offers native 23 @opencode/ai/providers options while still accepting custom packages', async () => {
  const { config, createProvider } = configWith();
  mount(NewProviderPage, config);

  const npmInput = screen.getByRole('combobox', { name: 'Adapter package' }) as HTMLInputElement;
  expect(npmInput.placeholder).toBe('@opencode/ai/providers/openai-compatible');
  expect(npmInput.getAttribute('aria-describedby')).toBe('provider-npm-hint');

  // Trigger dropdown options via ArrowDown on combobox
  await fireEvent.keyDown(npmInput, { key: 'ArrowDown' });
  const options = screen.getAllByRole('option');
  expect(options).toHaveLength(23);
  expect(options.some((opt) => opt.textContent?.includes('aisdk'))).toBe(false);
  expect(options.some((opt) => opt.textContent?.includes('@opencode/ai/providers/openai-compatible'))).toBe(true);

  // Accepts custom/any valid package, npm package, or file:// path without restriction
  await fireEvent.input(screen.getByRole('textbox', { name: 'Name' }), { target: { value: 'custom-driver' } });
  await fireEvent.input(npmInput, { target: { value: 'file:///local/custom-provider' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save provider' }));

  await waitFor(() => expect(createProvider).toHaveBeenCalledOnce());
  expect(createProvider).toHaveBeenCalledWith(
    expect.objectContaining({
      name: 'custom-driver',
      package: 'file:///local/custom-provider',
    }),
  );
});
