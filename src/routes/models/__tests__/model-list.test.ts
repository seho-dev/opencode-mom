import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, test, vi } from 'vitest';
import type { CommandAdapter } from '$src/config/adapter.js';
import { type ConfigStore, createConfigStore } from '$src/config/store.svelte.js';
import { getToasts } from '$src/shell/toast.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AppState } from '$src/types/app.js';
import type { ModelRef } from '$src/types/models.js';
import type { ProviderDef } from '$src/types/providers.js';
import ModelsPage from '../+page.svelte';

const customModel = {
  id: 'custom-gpt',
  ref: 'openai/custom-gpt',
  providerId: 'openai',
  name: 'Custom GPT',
  isCustom: true,
  limit: { context: 128000 },
};

const builtinModel = {
  id: 'official-claude',
  ref: 'anthropic/official-claude',
  providerId: 'anthropic',
  name: 'Official Claude',
  isCustom: false,
  limit: { context: 200000 },
};

function createMockConfig(overrides: Record<string, unknown> = {}) {
  const catalog = overrides.catalog
    ? (overrides.catalog as unknown as (typeof customModel)[])
    : [customModel, builtinModel];
  return {
    preferences: { locale: 'en' },
    catalog,
    catalogLoading: false,
    catalogError: null,
    loading: false,
    error: null,
    saving: false,
    models: () => catalog.filter((m) => m.isCustom),
    deleteModel: vi.fn().mockResolvedValue(undefined),
    refresh: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as unknown as ConfigStore;
}

describe('/models actions and copy contract', () => {
  test('model list query trims leading and trailing whitespace', async () => {
    const mockConfig = createMockConfig({
      models: () => [customModel, { ...customModel, id: 'beta', ref: 'openai/beta', name: 'Beta Model' }],
    });
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });
    const table = screen.getByRole('table') as HTMLTableElement;

    expect(table.tBodies[0].rows.length).toBe(2);

    await fireEvent.input(screen.getByRole('textbox', { name: 'Search models' }), {
      target: { value: '   custom-gpt   ' },
    });
    expect(table.tBodies[0].rows.length).toBe(1);
    expect(table.tBodies[0].rows[0].cells[1]?.textContent?.trim()).toBe('Custom GPT');
  });

  test('custom model renders copy button linking to /models/new?copyFrom=<ref>', async () => {
    const mockConfig = createMockConfig();
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    const copyLink = await screen.findByRole('link', { name: 'Copy openai/custom-gpt' });
    expect(copyLink).toBeDefined();
    expect(copyLink.getAttribute('href')).toBe(`/models/new?copyFrom=${encodeURIComponent(customModel.ref)}`);
    expect(copyLink.getAttribute('title')).toBe('Copy model');
  });

  test('builtin model rows are read-only and do not render copy, edit, or delete actions', async () => {
    const mockConfig = createMockConfig();
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    // Switch to Builtin tab
    const builtinTab = screen.getByRole('tab', { name: /Builtin/ });
    await fireEvent.click(builtinTab);

    expect(screen.getByText('Official Claude')).toBeDefined();
    expect(screen.getByText(/read-only/i)).toBeDefined();
    expect(screen.queryByRole('link', { name: /Copy/ })).toBeNull();
    expect(screen.queryByRole('link', { name: /Edit/ })).toBeNull();
    expect(screen.queryByRole('button', { name: /Delete/ })).toBeNull();
  });

  test('deleting a custom model calls deleteModel and removes row upon state update', async () => {
    let providersList: ProviderDef[] = [
      {
        name: 'openai',
        package: '@opencode/ai/providers/openai',
        models: {
          'custom-gpt': {
            id: 'custom-gpt',
            name: 'Custom GPT',
            limit: { context: 128000 },
          },
        },
      },
    ];

    const state: AppState = {
      preferences: { locale: 'en', theme: 'light' },
      providers: providersList,
      agents: [],
      groups: [],
      selectedGroupId: null,
    };

    const adapter = {
      loadAppState: vi.fn(async () => ({ ...state, providers: providersList })),
      deleteModel: vi.fn(async (ref: ModelRef) => {
        const [providerName, modelId] = ref.split('/') as [string, string];
        providersList = providersList.map((p) => {
          if (p.name !== providerName) return p;
          const nextModels = { ...p.models };
          delete nextModels[modelId];
          return { ...p, models: nextModels };
        });
      }),
      opencodeListModels: vi.fn(async () => [customModel, builtinModel]),
    };

    const config = createConfigStore(adapter as unknown as CommandAdapter, true, false);
    await config.refresh(true);

    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config } });

    expect(screen.getByText('Custom GPT')).toBeDefined();
    expect(screen.getByRole('tab', { name: 'Custom (1)' })).toBeDefined();

    const deleteBtn = screen.getByRole('button', { name: 'Delete openai/custom-gpt' });
    await fireEvent.click(deleteBtn);

    const confirmBtn = screen.getByRole('button', { name: 'Delete' });
    await fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(screen.queryByText('Custom GPT')).toBeNull();
      expect(screen.getByRole('tab', { name: 'Custom (0)' })).toBeDefined();
    });
    expect(adapter.deleteModel).toHaveBeenCalledWith('openai/custom-gpt');
    // Stale catalog still contains the ref, but custom tab is driven by persisted config.models()
    expect(config.catalog.some((m) => m.ref === 'openai/custom-gpt')).toBe(true);
  });

  test('failed deletion retains the custom model row and does not remove it from the list', async () => {
    const deleteModel = vi.fn().mockRejectedValue(new Error('Deletion blocked by references'));
    const mockConfig = createMockConfig({
      models: () => [customModel],
      deleteModel,
    });
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    expect(screen.getByText('Custom GPT')).toBeDefined();

    const deleteBtn = screen.getByRole('button', { name: 'Delete openai/custom-gpt' });
    await fireEvent.click(deleteBtn);

    const confirmBtn = screen.getByRole('button', { name: 'Delete' });
    await fireEvent.click(confirmBtn);

    await waitFor(() => expect(deleteModel).toHaveBeenCalledExactlyOnceWith('openai/custom-gpt'));
    await waitFor(() =>
      expect(
        getToasts().some(
          (entry) => entry.variant === 'error' && entry.description === 'Deletion blocked by references',
        ),
      ).toBe(true),
    );
    expect(screen.getByText('Custom GPT')).toBeDefined();
  });

  test('custom model without a name falls back to the model id instead of an unnamed placeholder', () => {
    const unnamedModel = { ...customModel, id: 'no-name-model', ref: 'openai/no-name-model', name: undefined };
    const mockConfig = createMockConfig({
      catalog: [unnamedModel],
      models: () => [unnamedModel],
    });
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    expect(screen.getByText('no-name-model')).toBeDefined();
    expect(screen.queryByText('unnamed')).toBeNull();
  });

  test('named custom model still renders its name', () => {
    const mockConfig = createMockConfig({ models: () => [customModel] });
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    expect(screen.getByText('Custom GPT')).toBeDefined();
    expect(screen.queryByText('unnamed')).toBeNull();
  });

  test('custom tab shows persisted models even while catalog is loading', () => {
    const mockConfig = createMockConfig({
      catalogLoading: true,
      catalog: [],
      models: () => [customModel],
    });
    render(ModelsPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    expect(screen.getByRole('tab', { name: 'Custom (1)' })).toBeDefined();
    expect(screen.getByText('Custom GPT')).toBeDefined();
  });
});
