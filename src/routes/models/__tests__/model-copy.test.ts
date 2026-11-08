import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterAll, beforeAll, describe, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ModelDef } from '$src/types/models.js';
import type { ProviderDef } from '$src/types/providers.js';
import NewModelPage from '../new/+page.svelte';

const originalScrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');
let mockPageUrl = new URL('http://localhost/models/new');

vi.mock('$app/navigation', () => ({
  goto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$app/state', () => ({
  page: {
    get url() {
      return mockPageUrl;
    },
  },
}));

beforeAll(() => {
  mockPageUrl = new URL('http://localhost/models/new');
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });
});

afterAll(() => {
  if (originalScrollIntoView) {
    Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', originalScrollIntoView);
  } else {
    Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
  }
});

const sourceModel: ModelDef & { ref: string; providerId: string; name: string; isCustom: boolean } = {
  id: 'gpt-4o',
  ref: 'openai/gpt-4o',
  providerId: 'openai',
  name: 'gpt-4o',
  isCustom: true,
  family: 'gpt-4',
  capabilities: { tools: true, input: ['text', 'image'], output: ['text'] },
  cost: { input: 2.5, output: 10 },
  limit: { context: 128000, output: 4096 },
  settings: { temperature: 0.7 },
  variants: [{ id: 'fast', body: { speed: 1 } }],
};

const collisionModel: ModelDef & { ref: string; providerId: string; name: string; isCustom: boolean } = {
  id: 'gpt-4o-copy',
  ref: 'openai/gpt-4o-copy',
  providerId: 'openai',
  name: 'gpt-4o-copy',
  isCustom: true,
};

const builtinModel = {
  id: 'claude-3-opus',
  ref: 'anthropic/claude-3-opus',
  providerId: 'anthropic',
  name: 'Claude 3 Opus',
  isCustom: false,
};

const providers: ProviderDef[] = [
  { name: 'openai', models: {} },
  { name: 'anthropic', models: {} },
];

function createMockConfig(overrides: Record<string, unknown> = {}) {
  const modelsList = overrides['modelsList']
    ? (overrides['modelsList'] as unknown as (typeof sourceModel)[])
    : [sourceModel, collisionModel, builtinModel];
  return {
    preferences: { locale: 'en' },
    providers,
    saving: false,
    loading: false,
    catalogLoading: false,
    models: () => modelsList,
    catalogModels: () => [],
    createModel: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as unknown as ConfigStore;
}

describe('/models/new copy integration', () => {
  test('copy preselects provider, generates collision-free ID with suffix, and pre-fills fields', async () => {
    mockPageUrl = new URL('http://localhost/models/new?copyFrom=openai%2Fgpt-4o');
    const mockConfig = createMockConfig();

    render(NewModelPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    // Since 'openai/gpt-4o-copy' already exists, next ID should be 'gpt-4o-copy-2'
    await waitFor(() => {
      const idInput = screen.getByRole('textbox', { name: 'Model ID' }) as HTMLInputElement;
      expect(idInput.value).toBe('gpt-4o-copy-2');
    });

    const familyInput = screen.getByRole('textbox', { name: 'Family' }) as HTMLInputElement;
    expect(familyInput.value).toBe('gpt-4');

    expect(mockConfig.createModel).not.toHaveBeenCalled();
  });

  test('saving copied model submits preselected provider and deep-copied fields', async () => {
    mockPageUrl = new URL('http://localhost/models/new?copyFrom=openai%2Fgpt-4o');
    const createModel = vi.fn().mockResolvedValue(undefined);
    const mockConfig = createMockConfig({ createModel });

    render(NewModelPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    await waitFor(() => {
      const idInput = screen.getByRole('textbox', { name: 'Model ID' }) as HTMLInputElement;
      expect(idInput.value).toBe('gpt-4o-copy-2');
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));

    await waitFor(() => expect(createModel).toHaveBeenCalledOnce());
    expect(createModel).toHaveBeenCalledWith('openai', {
      id: 'gpt-4o-copy-2',
      family: 'gpt-4',
      capabilities: {
        tools: true,
        input: ['text', 'image'],
        output: ['text'],
      },
      cost: {
        input: 2.5,
        output: 10,
      },
      limit: {
        context: 128000,
        output: 4096,
      },
      settings: {
        temperature: 0.7,
      },
      variants: [
        {
          id: 'fast',
          body: { speed: 1 },
        },
      ],
    });
  });

  test('invalid or missing copyFrom renders error state and directory return link', async () => {
    mockPageUrl = new URL('http://localhost/models/new?copyFrom=nonexistent%2Fmodel');
    const mockConfig = createMockConfig();

    render(NewModelPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    await waitFor(() => {
      expect(screen.getByRole('alert').textContent).toContain('source custom model does not exist');
    });
    const returnLink = screen.getByRole('link', { name: 'Back to models' });
    expect(returnLink.getAttribute('href')).toBe('/models');
  });

  test('model copy retains user edits on failed save without mutating source', async () => {
    mockPageUrl = new URL('http://localhost/models/new?copyFrom=openai%2Fgpt-4o');
    const createModel = vi.fn().mockRejectedValue(new Error('disk write failed'));
    const mockConfig = createMockConfig({ createModel });

    render(NewModelPage, {}, { wrapper: Harness, wrapperProps: { config: mockConfig } });

    const idInput = screen.getByRole('textbox', { name: 'Model ID' }) as HTMLInputElement;
    await waitFor(() => expect(idInput.value).toBe('gpt-4o-copy-2'));

    const familyInput = screen.getByRole('textbox', { name: 'Family' }) as HTMLInputElement;
    await fireEvent.input(familyInput, { target: { value: 'edited-family-draft' } });
    expect(familyInput.value).toBe('edited-family-draft');

    // Failed save keeps edited draft intact
    await fireEvent.click(screen.getByRole('button', { name: 'Save model' }));
    await waitFor(() => expect(createModel).toHaveBeenCalledOnce());
    expect(familyInput.value).toBe('edited-family-draft');

    // Source model remains completely unmutated
    expect(sourceModel.family).toBe('gpt-4');
    expect(sourceModel.id).toBe('gpt-4o');
  });
});
