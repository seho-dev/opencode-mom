import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { CommandAdapter } from '$src/config/adapter.js';
import { type ConfigStore, createConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AppState } from '$src/types/app.js';
import type { Group } from '$src/types/groups.js';
import NewGroupRoute from '../new/+page.svelte';

const originalScrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');

let mockPageUrl = new URL('http://localhost/groups/new');

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

beforeEach(() => {
  mockPageUrl = new URL('http://localhost/groups/new');
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });
});

afterEach(() => {
  if (originalScrollIntoView) {
    Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', originalScrollIntoView);
  } else {
    Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
  }
});

const sourceGroup: Group = {
  id: 'group_prod',
  name: 'Production Cluster',
  description: 'Production primary group',
  type: 'omo',
  openCodeAgentOverrides: [{ agentName: 'build', modelRef: 'example/existing' }],
  slimAgentOverrides: [{ agentName: 'build', modelRef: 'example/existing' }],
  omoAgentOverrides: [{ agentName: 'build', modelRef: 'example/existing', variant: 'fast' }],
  omoCategoryMappings: [{ categoryName: 'visual_engineering', modelRef: 'example/existing' }],
  isEnabled: true,
  updatedAt: '2025-01-01T00:00:00.000Z',
};

function createMockConfig(overrides: Record<string, unknown> = {}) {
  const groups = [sourceGroup];
  return {
    preferences: { locale: 'en', theme: 'system' },
    locale: 'en',
    groups,
    providers: [{ id: 'example', name: 'Example Provider', models: [{ id: 'existing', name: 'Existing' }] }],
    agents: [{ id: 'build', name: 'Build Agent', source: 'inline' }],
    categories: [{ id: 'visual_engineering', name: 'Visual Engineering' }],
    saving: false,
    formResetVersion: 0,
    catalogLoading: false,
    catalogModels: () => [],
    models: () => [],
    saveGroup: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as unknown as ConfigStore;
}

describe('/groups/new route end-to-end integration', () => {
  test('without copyFrom param: renders blank form without saving on mount', () => {
    const saveGroup = vi.fn();
    const config = createMockConfig({ saveGroup });

    render(NewGroupRoute, {}, { wrapper: Harness, wrapperProps: { config } });

    const nameInput = screen.getByRole('textbox', { name: 'Group name' }) as HTMLInputElement;
    expect(nameInput.value).toBe('');
    expect(saveGroup).not.toHaveBeenCalled();
  });

  test('with copyFrom param: prefills copy data, preserves overrides and disabled initial state, and saves with new ID on explicit save', async () => {
    mockPageUrl = new URL('http://localhost/groups/new?copyFrom=group_prod');
    const saveGroup = vi.fn().mockResolvedValue(undefined);
    const config = createMockConfig({ saveGroup });

    render(NewGroupRoute, {}, { wrapper: Harness, wrapperProps: { config } });

    const nameInput = screen.getByRole('textbox', { name: 'Group name' }) as HTMLInputElement;
    expect(nameInput.value).toBe('Production Cluster (copy)');

    const descInput = screen.getByRole('textbox', { name: 'Description' }) as HTMLInputElement;
    expect(descInput.value).toBe('Production primary group');

    // Immutability before explicit save: no premature writes
    expect(saveGroup).not.toHaveBeenCalled();

    // Source object in config remains completely untouched
    expect(sourceGroup.id).toBe('group_prod');
    expect(sourceGroup.name).toBe('Production Cluster');

    // Explicit submit saves with fresh UUID and timestamp
    await fireEvent.click(screen.getByRole('button', { name: 'Save group' }));

    await waitFor(() => expect(saveGroup).toHaveBeenCalledOnce());
    const saved = saveGroup.mock.calls[0][0];

    expect(saved.id).not.toBe('group_prod');
    expect(saved.id).toMatch(/^[0-9a-f-]{36}$/);
    expect(saved.name).toBe('Production Cluster (copy)');
    expect(saved.description).toBe('Production primary group');
    expect(saved.type).toBe('omo');
    expect(saved.isEnabled).toBe(true);
    expect(saved.openCodeAgentOverrides).toEqual([{ agentName: 'build', modelRef: 'example/existing' }]);
    expect(saved.slimAgentOverrides).toEqual([{ agentName: 'build', modelRef: 'example/existing' }]);
    expect(saved.omoAgentOverrides).toEqual([{ agentName: 'build', modelRef: 'example/existing', variant: 'fast' }]);
    expect(saved.omoCategoryMappings).toEqual([{ categoryName: 'visual_engineering', modelRef: 'example/existing' }]);
    expect(saved.createdAt).toBeUndefined();
    expect(typeof saved.updatedAt).toBe('string');
  });

  test('with invalid or missing source id in copyFrom: renders error banner, prevents blank save and shows return link', async () => {
    mockPageUrl = new URL('http://localhost/groups/new?copyFrom=nonexistent_id');
    const saveGroup = vi.fn();
    const config = createMockConfig({ saveGroup });

    render(NewGroupRoute, {}, { wrapper: Harness, wrapperProps: { config } });

    // Renders error state banner indicating group not found
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('The source group is unavailable. Return to Groups and choose another group.');

    // Save button does not exist in missing-source state, preventing any writes
    expect(screen.queryByRole('button', { name: 'Save group' })).toBeNull();
    expect(saveGroup).not.toHaveBeenCalled();

    // Provides return navigation to groups directory
    const dirLink = screen.getByRole('link', { name: 'Group Directory' });
    expect(dirLink.getAttribute('href')).toBe('/groups');
  });

  test('editing draft and background config refresh preserves local draft and source immutability', async () => {
    mockPageUrl = new URL('http://localhost/groups/new?copyFrom=group_prod');
    const rawState: AppState = {
      groups: [structuredClone(sourceGroup)],
      selectedGroupId: null,
      preferences: { locale: 'en', theme: 'system', defaultPlugin: 'native' },
      providers: [{ id: 'example', name: 'Example Provider', models: [{ id: 'existing', name: 'Existing' }] }],
      agents: [{ id: 'build', name: 'Build Agent', source: 'inline' }],
      categories: [{ id: 'visual_engineering', name: 'Visual Engineering' }],
      commands: [],
      skills: [],
      mcp: [],
      models: [],
      mcpProxies: [],
      mcpProxySettings: { enabled: false },
    } as unknown as AppState;

    const saveGroup = vi.fn().mockResolvedValue(undefined);
    const adapter: Partial<CommandAdapter> = {
      loadAppState: vi.fn(async () => structuredClone(rawState)),
      saveGroup,
      opencodeListModels: vi.fn(async () => []),
    };

    const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);
    await config.refresh(true);

    render(NewGroupRoute, {}, { wrapper: Harness, wrapperProps: { config } });

    const nameInput = screen.getByRole('textbox', { name: 'Group name' }) as HTMLInputElement;
    const descInput = screen.getByRole('textbox', { name: 'Description' }) as HTMLInputElement;
    expect(nameInput.value).toBe('Production Cluster (copy)');
    expect(descInput.value).toBe('Production primary group');

    // User edits the local draft name and description
    await fireEvent.input(nameInput, { target: { value: 'Custom Cluster Name' } });
    await fireEvent.input(descInput, { target: { value: 'Custom Cluster Description' } });
    expect(nameInput.value).toBe('Custom Cluster Name');
    expect(descInput.value).toBe('Custom Cluster Description');

    // Simulate background config refresh with remotely changed source group
    rawState.groups = [
      {
        ...sourceGroup,
        name: 'Production Cluster Renamed Remotely',
        description: 'Remotely updated description',
      },
    ];
    await config.refresh(true);

    // Form retains local user edits and source group is not mutated by the draft
    expect(nameInput.value).toBe('Custom Cluster Name');
    expect(descInput.value).toBe('Custom Cluster Description');
    expect(config.groups[0]?.name).toBe('Production Cluster Renamed Remotely');
    expect(saveGroup).not.toHaveBeenCalled();
  });
});
