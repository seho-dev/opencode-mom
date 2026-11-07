import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { beforeNavigate } from '$app/navigation';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { BehaviorDocument } from '$src/types/behavior.js';
import Behavior from '../Behavior.svelte';

vi.mock('$app/navigation', () => ({ beforeNavigate: vi.fn() }));

const document: BehaviorDocument = {
  path: '/global/AGENTS.md',
  resolvedPath: '/resolved/AGENTS.md',
  content: '# Original\n',
  exists: true,
};

function mount(overrides: Record<string, unknown> = {}, active = true) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    selectedProjectId: 'project-a',
    getGlobalBehavior: vi.fn().mockResolvedValue(document),
    saveGlobalBehavior: vi.fn().mockResolvedValue(document),
    generatePrompt: vi.fn().mockResolvedValue('Generated preview content'),
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
    ...overrides,
  } as unknown as ConfigStore;

  return { ...render(Behavior, { active }, { wrapper: Harness, wrapperProps: { config } }), config };
}

beforeEach(() => {
  vi.mocked(beforeNavigate).mockClear();
});

test('inactive mount does no native work; tabs retain dirty text without read or auto-save', async () => {
  const { config, rerender } = mount({}, false);

  expect(config.getGlobalBehavior).not.toHaveBeenCalled();
  await rerender({ active: true });

  const input = (await screen.findByLabelText(en['settings.behaviorContent'])) as HTMLTextAreaElement;

  await waitFor(() => expect(input.disabled).toBe(false));
  expect(screen.getByText(document.resolvedPath ?? '')).toBeTruthy();
  await fireEvent.input(input, { target: { value: 'private-local-draft' } });
  await rerender({ active: false });
  expect(input.value).toBe('private-local-draft');
  await fireEvent.submit(input.closest('form') as HTMLFormElement);
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();
  await rerender({ active: true });
  expect(input.value).toBe('private-local-draft');
  expect(config.getGlobalBehavior).toHaveBeenCalledOnce();
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();
});

test('refresh and discard require confirmation; failed refresh keeps dirty content and original snapshot', async () => {
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  const getGlobalBehavior = vi
    .fn()
    .mockResolvedValueOnce(document)
    .mockRejectedValueOnce(new Error('read failed'))
    .mockResolvedValue(document);
  const { config } = mount({ getGlobalBehavior });
  const input = screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;

  await waitFor(() => expect(input.disabled).toBe(false));
  await fireEvent.input(input, { target: { value: 'draft' } });
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorReload'] }));
  expect(confirm).toHaveBeenCalledWith(en['settings.behaviorDiscardConfirm'] ?? 'settings.behaviorDiscardConfirm');
  expect(config.getGlobalBehavior).toHaveBeenCalledOnce();
  expect(input.value).toBe('draft');
  confirm.mockReturnValue(true);
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorReload'] }));
  await screen.findByText(/read failed/);
  expect(input.value).toBe('draft');
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorDiscard'] }));
  await waitFor(() => expect(input.value).toBe(document.content));
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();
  confirm.mockRestore();
});

test('conflicts keep draft and frozen snapshot; empty existing file may be saved', async () => {
  const saveGlobalBehavior = vi
    .fn()
    .mockRejectedValueOnce(new Error('changed externally'))
    .mockResolvedValueOnce({ ...document, content: '' });

  mount({ saveGlobalBehavior });

  const input = screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;

  await waitFor(() => expect(input.disabled).toBe(false));
  await fireEvent.input(input, { target: { value: '' } });
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorSave'] }));
  await screen.findByText(/changed externally/);
  expect(input.value).toBe('');
  expect(saveGlobalBehavior).toHaveBeenCalledExactlyOnceWith({
    content: '',
    expectedPath: document.resolvedPath,
    expectedContent: document.content,
  });
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorSave'] }));
  await screen.findByText(en['settings.behaviorSaved']);
  expect(saveGlobalBehavior).toHaveBeenLastCalledWith({
    content: '',
    expectedPath: document.resolvedPath,
    expectedContent: document.content,
  });
});

test('missing source creates even an empty file with explicit null expectations', async () => {
  const { config } = mount({
    getGlobalBehavior: vi.fn().mockResolvedValue({ ...document, exists: false, resolvedPath: null, content: '' }),
  });

  await screen.findByText(en['settings.behaviorMissing']);
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorSave'] }));
  await waitFor(() =>
    expect(config.saveGlobalBehavior).toHaveBeenCalledExactlyOnceWith({
      content: '',
      expectedPath: null,
      expectedContent: null,
    }),
  );
});

test('route leave protects dirty local text but switching settings tabs never prompts', async () => {
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);

  mount();

  const input = screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;

  await waitFor(() => expect(input.disabled).toBe(false));
  await fireEvent.input(input, { target: { value: 'private' } });

  const callback = vi.mocked(beforeNavigate).mock.calls[0]?.[0];
  const cancel = vi.fn();

  callback?.({ to: { url: new URL('http://localhost/settings?tab=general') }, cancel, willUnload: false } as Parameters<
    NonNullable<typeof callback>
  >[0]);
  expect(confirm).not.toHaveBeenCalled();
  callback?.({ to: { url: new URL('http://localhost/plugins') }, cancel, willUnload: false } as Parameters<
    NonNullable<typeof callback>
  >[0]);
  expect(confirm).toHaveBeenCalledWith(en['settings.behaviorLeaveConfirm'] ?? 'settings.behaviorLeaveConfirm');
  expect(cancel).toHaveBeenCalledOnce();
  confirm.mockRestore();
});

test('behavior PromptGenerator closing discards late results and reopen is not contaminated', async () => {
  let resolve!: (value: string) => void;
  const pendingPromise = new Promise<string>((res) => {
    resolve = res;
  });
  const { config } = mount({ generatePrompt: vi.fn().mockImplementation(() => pendingPromise) }, true);

  await waitFor(() =>
    expect((screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement).disabled).toBe(false),
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
  expect(screen.getByRole('heading', { name: en['promptGenerator.behaviorTitle'] })).toBeTruthy();
  expect((screen.getByLabelText(en['promptGenerator.instructions']) as HTMLTextAreaElement).value).toBe('');
  resolve('Late arriving result from session 1');
  await pendingPromise;
  await tick();
  expect(screen.queryByLabelText(en['promptGenerator.preview'])).toBeNull();
  expect((screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement).value).toBe(document.content);
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();
});

test('behavior PromptGenerator integration happy: edited draft, lazy catalog, preview-only, explicit Save', async () => {
  const catalogModels = vi.fn().mockReturnValue([
    {
      ref: 'demo/fast',
      providerId: 'demo',
      modelId: 'fast',
      name: 'Fast',
      isCustom: true,
      variants: [{ id: 'quick' }],
    },
  ]);
  const models = vi.fn().mockReturnValue([]);
  const { config } = mount(
    {
      catalogModels,
      models,
      generatePrompt: vi.fn().mockResolvedValue('Generated behavior preview'),
      saveGlobalBehavior: vi.fn().mockResolvedValue({ ...document, content: 'Generated behavior preview' }),
    },
    true,
  );
  const input = screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;

  await waitFor(() => expect(input.disabled).toBe(false));
  expect(catalogModels).not.toHaveBeenCalled();
  expect(models).not.toHaveBeenCalled();
  await fireEvent.input(input, { target: { value: 'my local draft' } });
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  expect(catalogModels).toHaveBeenCalled();
  await fireEvent.input(screen.getByLabelText(en['promptGenerator.instructions']), {
    target: { value: 'Improve rules' },
  });
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.generate'] }));
  expect(config.generatePrompt).toHaveBeenCalledExactlyOnceWith({
    kind: 'behavior',
    preset: 'improve',
    instructions: 'Improve rules',
    currentPrompt: 'my local draft',
    model: undefined,
    variant: undefined,
  });
  await waitFor(() => expect(screen.getByLabelText(en['promptGenerator.preview'])).toBeTruthy());
  expect(input.value).toBe('my local draft');
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.apply'] }));
  expect(input.value).toBe('Generated behavior preview');
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();

  const saveBtn = screen.getByRole('button', { name: en['settings.behaviorSave'] });

  expect((saveBtn as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(saveBtn);
  await waitFor(() =>
    expect(config.saveGlobalBehavior).toHaveBeenCalledExactlyOnceWith({
      content: 'Generated behavior preview',
      expectedPath: document.resolvedPath,
      expectedContent: document.content,
    }),
  );
  await screen.findByText(en['settings.behaviorSaved']);
});

test('behavior PromptGenerator disabled when inactive prevents opening and leaves draft unchanged', async () => {
  const getGlobalBehavior = vi.fn().mockResolvedValue(document);
  const saveGlobalBehavior = vi.fn().mockResolvedValue(document);
  const { config } = mount(
    { getGlobalBehavior, saveGlobalBehavior, generatePrompt: vi.fn().mockResolvedValue('Generated content') },
    false,
  );
  const trigger = screen.getByRole('button', { name: en['promptGenerator.open'] }) as HTMLButtonElement;

  expect(trigger.disabled).toBe(true);
  await fireEvent.click(trigger);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(config.generatePrompt).not.toHaveBeenCalled();
  expect(getGlobalBehavior).not.toHaveBeenCalled();
  expect(saveGlobalBehavior).not.toHaveBeenCalled();
});

test('behavior generator stays disabled during pending document reads and saves', async () => {
  let getResolve!: (value: typeof document) => void;
  let saveResolve!: (value: typeof document) => void;
  const getPromise = new Promise<typeof document>((res) => {
    getResolve = res;
  });
  const savePromise = new Promise<typeof document>((res) => {
    saveResolve = res;
  });
  const { config } = mount(
    {
      getGlobalBehavior: vi.fn().mockImplementation(() => getPromise),
      saveGlobalBehavior: vi.fn().mockImplementation(() => savePromise),
      generatePrompt: vi.fn().mockResolvedValue('Generated'),
    },
    true,
  );

  await waitFor(() => expect(config.getGlobalBehavior).toHaveBeenCalledOnce());

  const trigger = screen.getByRole('button', { name: en['promptGenerator.open'] }) as HTMLButtonElement;

  expect(trigger.disabled).toBe(true);
  await fireEvent.click(trigger);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(config.generatePrompt).not.toHaveBeenCalled();
  getResolve(document);
  await waitFor(() =>
    expect((screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement).disabled).toBe(false),
  );

  const input = screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;

  await fireEvent.input(input, { target: { value: 'draft' } });
  await fireEvent.click(screen.getByRole('button', { name: en['settings.behaviorSave'] }));
  expect(config.saveGlobalBehavior).toHaveBeenCalledOnce();

  const savingTrigger = screen.getByRole('button', { name: en['promptGenerator.open'] }) as HTMLButtonElement;

  expect(savingTrigger.disabled).toBe(true);
  await fireEvent.click(savingTrigger);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(config.generatePrompt).not.toHaveBeenCalled();
  expect(input.value).toBe('draft');
  saveResolve({ ...document, content: 'draft' });
  await screen.findByText(en['settings.behaviorSaved']);
  expect((screen.getByRole('button', { name: en['promptGenerator.open'] }) as HTMLButtonElement).disabled).toBe(false);
});

test('deactivating behavior discards in-flight generation without changing the retained draft', async () => {
  let resolve!: (value: string) => void;
  const pendingPromise = new Promise<string>((res) => {
    resolve = res;
  });
  const { config, rerender } = mount({ generatePrompt: vi.fn().mockImplementation(() => pendingPromise) }, true);
  const input = screen.getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;

  await waitFor(() => expect(input.disabled).toBe(false));
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  await fireEvent.input(screen.getByLabelText(en['promptGenerator.instructions']), {
    target: { value: 'Instructions before unmount' },
  });
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.generate'] }));
  expect(config.generatePrompt).toHaveBeenCalledTimes(1);
  await rerender({ active: false });
  resolve('Result for unmounted component');
  await pendingPromise;
  await tick();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(input.value).toBe(document.content);
  expect(config.saveGlobalBehavior).not.toHaveBeenCalled();
  await rerender({ active: true });
  await fireEvent.click(screen.getByRole('button', { name: en['promptGenerator.open'] }));
  expect(screen.getByRole('heading', { name: en['promptGenerator.behaviorTitle'] })).toBeTruthy();
  expect(screen.queryByLabelText(en['promptGenerator.preview'])).toBeNull();
});
