import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
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
