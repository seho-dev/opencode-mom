import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { IslandMessage, IslandState } from '$src/types/island.js';
import Island from '../+page.svelte';
import islandSource from '../+page.svelte?raw';

const native = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  listen: vi.fn(),
  handlers: new Map<string, () => void>(),
  unlisten: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: native.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));

function state(): IslandState {
  return {
    supported: true,
    enabled: true,
    connection: 'connected',
    connectionGeneration: 0,
    sessions: [
      { id: 'recent', title: 'Recent work', directory: '/code/recent-project', updatedAt: 4, status: 'idle' },
      { id: 'running', title: 'Running work', directory: '/code/active-project', updatedAt: 3, status: 'running' },
      { id: 'pending', title: 'Pending work', directory: '/code/pending-project', updatedAt: 1, status: 'idle' },
      {
        id: 'child',
        title: 'Hidden child',
        directory: '/code/recent-project',
        updatedAt: 5,
        status: 'idle',
        parentId: 'recent',
      },
    ],
    permissions: [
      {
        id: 'perm_1',
        sessionId: 'pending',
        action: 'bash',
        resources: ['git status'],
        message: 'Read repository status',
      },
    ],
    questions: [],
    preferences: { locale: 'en', theme: 'dark' },
    notchWidth: 180,
    topInset: 32,
  };
}

function mount(overrides: Record<string, unknown> = {}, initial = state()) {
  let current = initial;
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    applyPreferences: vi.fn(),
    getIslandState: vi.fn().mockImplementation(async () => current),
    refreshIsland: vi.fn().mockImplementation(async () => current),
    setIslandView: vi.fn().mockResolvedValue(undefined),
    islandMessages: vi
      .fn()
      .mockImplementation(async (id: string) => [
        { id: `msg_${id}`, role: 'assistant', text: `<b>${id}</b>\nPublic text`, time: 1, truncated: false },
      ]),
    islandSend: vi.fn().mockResolvedValue(undefined),
    islandReplyPermission: vi.fn().mockResolvedValue(undefined),
    islandReplyQuestion: vi.fn().mockResolvedValue(undefined),
    islandCancelQuestion: vi.fn().mockResolvedValue(undefined),
    islandInterrupt: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  const view = render(Island, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });
  return {
    ...view,
    config,
    setState: (next: IslandState) => {
      current = next;
    },
  };
}
async function expand() {
  await peek();
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  await screen.findByRole('button', { name: 'Allow once' });
  await waitFor(() =>
    expect(screen.getByRole('region', { name: 'Recent messages and pending requests' }).getAttribute('aria-busy')).toBe(
      'false',
    ),
  );
}
async function peek() {
  await screen.findByRole('button', { name: 'Show quick area' });
  await fireEvent.pointerEnter(screen.getByRole('main', { name: 'Sessions' }));
  await screen.findByRole('button', { name: 'Allow once' });
}
beforeEach(() => {
  native.isTauri.mockReturnValue(true);
  native.listen.mockReset();
  native.handlers.clear();
  native.unlisten.mockClear();
  native.listen.mockImplementation(async (name: string, handler: () => void) => {
    native.handlers.set(name, handler);
    return () => {
      native.handlers.delete(name);
      native.unlisten(name);
    };
  });
});

test('read → expand → select → messages → send → permission → collapse cleans up', async () => {
  const view = mount();
  await screen.findByText('1 pending');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(native.listen).toHaveBeenCalledTimes(5);
  for (const name of [
    'island:changed',
    'island:collapsed',
    'island:pointer-enter',
    'island:pointer-leave',
    'app:preferences-changed',
  ])
    expect(native.listen).toHaveBeenCalledWith(name, expect.any(Function), { target: 'island' });
  expect(view.container.querySelector('.notch-gap')?.children).toHaveLength(0);
  expect(view.container.querySelector('main')?.getAttribute('style')).toContain('--top-inset: 32px');
  await expand();
  expect(view.config.setIslandView).toHaveBeenCalledWith('expanded');
  const navigation = screen.getByRole('navigation', { name: 'Sessions' });
  expect(
    within(navigation)
      .getAllByRole('button')
      .map((button) => button.textContent),
  ).toEqual([
    expect.stringContaining('Pending work'),
    expect.stringContaining('Running work'),
    expect.stringContaining('Recent work'),
  ]);
  expect(screen.queryByText('Hidden child')).toBeNull();
  expect(screen.getByText('<b>pending</b> Public text').querySelector('b')).toBeNull();
  expect(view.config.islandMessages).toHaveBeenCalledWith('pending', 0);
  expect(document.activeElement).not.toBe(screen.getByRole('textbox', { name: 'Message' }));
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Hello\nworld' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() => expect(view.config.islandSend).toHaveBeenCalledOnce());
  expect(view.config.islandSend).toHaveBeenCalledWith(
    'pending',
    'Hello\nworld',
    expect.stringMatching(/^msg_[0-9a-f-]{36}$/),
    0,
  );
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe(''));
  await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('pending', 'perm_1', 'once', 0);
  view.setState({ ...state(), permissions: [] });
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull());
  await fireEvent.click(within(navigation).getByRole('button', { name: /Running work/ }));
  await screen.findByText('<b>running</b> Public text');
  await fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
  expect(view.config.islandInterrupt).toHaveBeenCalledExactlyOnceWith('running', 0);
  await fireEvent.keyDown(window, { key: 'Escape' });
  await screen.findByRole('button', { name: 'Show quick area' });
  expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact');
  const messagesRead = view.config.islandMessages.mock.calls.length;
  const changed = native.handlers.get('island:changed');
  changed?.();
  const reads = view.config.getIslandState.mock.calls.length;
  view.unmount();
  await new Promise((resolve) => setTimeout(resolve, 250));
  expect(native.unlisten).toHaveBeenCalledTimes(5);
  changed?.();
  expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
  expect(view.config.islandMessages).toHaveBeenCalledTimes(messagesRead);
});

test('ambiguous sends keep draft and UUID, guard double submit, and require explicit refresh before retry', async () => {
  let fail!: (cause: Error) => void;
  const islandSend = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((_resolve, reject) => {
          fail = reject;
        }),
    )
    .mockResolvedValueOnce(undefined);
  const view = mount({ islandSend });
  await expand();
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Keep this draft' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  expect(islandSend).toHaveBeenCalledOnce();
  fail(new Error('Reply lost'));
  await screen.findByText('Message may have been accepted. Refresh before retrying.');
  const retry = screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement;
  expect(retry.disabled).toBe(true);
  expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('Keep this draft');
  await fireEvent.click(screen.getByRole('button', { name: /Running work/ }));
  await screen.findByText('<b>running</b> Public text');
  await fireEvent.click(screen.getByRole('button', { name: /Pending work/ }));
  await screen.findByText('<b>pending</b> Public text');
  expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('Keep this draft');
  expect((screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(view.config.refreshIsland).toHaveBeenCalledOnce();
  await fireEvent.click(screen.getByRole('button', { name: 'Retry same message' }));
  expect(islandSend.mock.calls[1]).toEqual(islandSend.mock.calls[0]);
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe(''));
});

test('only explicit Refresh with the same public user ID confirms acceptance and permits a new message without duplicate POST', async () => {
  const islandSend = vi.fn().mockRejectedValueOnce(new Error('Reply lost')).mockResolvedValueOnce(undefined);
  const view = mount({ islandSend });
  await expand();
  const textbox = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
  await fireEvent.input(textbox, { target: { value: 'Original text' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await screen.findByText('Message may have been accepted. Refresh before retrying.');
  const messageId = islandSend.mock.calls[0][2];
  view.config.islandMessages.mockResolvedValue([
    { id: messageId, role: 'assistant', text: 'Wrong role', time: 1, truncated: false },
    { id: 'msg_different', role: 'user', text: 'Original text', time: 1, truncated: false },
  ]);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByRole('button', { name: 'Discard this attempt' });
  expect(textbox.value).toBe('Original text');
  expect(textbox.disabled).toBe(true);
  expect(islandSend).toHaveBeenCalledOnce();
  view.config.islandMessages.mockResolvedValue([
    { id: messageId, role: 'user', text: 'Accepted public text', time: 1, truncated: false },
  ]);
  native.handlers.get('island:changed')?.();
  await screen.findByText('Accepted public text');
  expect(textbox.value).toBe('Original text');
  expect(textbox.disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(textbox.value).toBe(''));
  expect(textbox.disabled).toBe(false);
  expect(screen.queryByRole('button', { name: 'Retry same message' })).toBeNull();
  expect(screen.queryByRole('alert')).toBeNull();
  expect(islandSend).toHaveBeenCalledOnce();
  await fireEvent.input(textbox, { target: { value: 'Next message' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() => expect(islandSend).toHaveBeenCalledTimes(2));
  expect(islandSend.mock.calls[1]).toEqual([
    'pending',
    'Next message',
    expect.stringMatching(/^msg_[0-9a-f-]{36}$/),
    0,
  ]);
  expect(islandSend.mock.calls[1][2]).not.toBe(messageId);
});

test('explicit discard after 409 clears only the local attempt and allows a subsequent manual Send', async () => {
  const islandSend = vi
    .fn()
    .mockRejectedValueOnce({ code: 'conflict', message: 'HTTP 409' })
    .mockResolvedValueOnce(undefined);
  const view = mount({ islandSend });
  await expand();
  const textbox = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
  await fireEvent.input(textbox, { target: { value: 'Conflicted attempt' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await screen.findByText(
    'Message ID conflict (409). Refresh and check the session before retrying. Your draft and message ID are kept.',
  );
  expect(screen.queryByRole('button', { name: 'Discard this attempt' })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Discard this attempt' }));
  expect(textbox.value).toBe('');
  expect(textbox.disabled).toBe(false);
  expect((screen.getByRole('button', { name: 'Send' }) as HTMLButtonElement).disabled).toBe(true);
  expect(islandSend).toHaveBeenCalledOnce();
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandReplyQuestion).not.toHaveBeenCalled();
  expect(view.config.islandCancelQuestion).not.toHaveBeenCalled();
  await fireEvent.input(textbox, { target: { value: 'New explicit attempt' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() => expect(islandSend).toHaveBeenCalledTimes(2));
  expect(islandSend.mock.calls[1][2]).not.toBe(islandSend.mock.calls[0][2]);
  await waitFor(() => expect(textbox.value).toBe(''));
  native.handlers.get('island:collapsed')?.();
  await screen.findByRole('button', { name: 'Show quick area' });
  view.unmount();
  expect(islandSend).toHaveBeenCalledTimes(2);
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
  expect(view.config.islandCancelQuestion).not.toHaveBeenCalled();
});

test.each([
  { code: 'validation_failed', message: 'Invalid message' },
  { code: 'not_found', message: 'Session not found' },
  { code: 'configuration_failed', message: 'API HTTP 400' },
  { code: 'configuration_failed', message: 'API HTTP 401' },
  { code: 'configuration_failed', message: 'API HTTP 404' },
])(
  'definite rejection leaves draft editable, retains same-ID retry until editing, and never automatically posts ($message)',
  async (failure) => {
    const islandSend = vi.fn().mockRejectedValue(failure);
    mount({ islandSend });
    await expand();
    const textbox = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
    await fireEvent.input(textbox, { target: { value: 'Rejected text' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await screen.findByText(failure.message);
    expect(textbox.disabled).toBe(false);
    expect(textbox.value).toBe('Rejected text');
    expect(screen.queryByText('Message may have been accepted. Refresh before retrying.')).toBeNull();
    expect(islandSend).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await waitFor(() => expect(islandSend).toHaveBeenCalledTimes(2));
    expect(islandSend.mock.calls[1]).toEqual(islandSend.mock.calls[0]);
    await waitFor(() => expect(textbox.disabled).toBe(false));
    await fireEvent.input(textbox, { target: { value: 'Edited text' } });
    expect(islandSend).toHaveBeenCalledTimes(2);
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await waitFor(() => expect(islandSend).toHaveBeenCalledTimes(3));
    expect(islandSend.mock.calls[2][2]).not.toBe(islandSend.mock.calls[0][2]);
  },
);

test.each([
  ['en', 'Session status · Show quick area', '2 running', '0 running', 'Connection unavailable'],
  ['zh', '会话状态 · 显示快捷区', '2 个运行中', '0 个运行中', '连接不可用'],
])(
  'compact shows connected running count including children, never stale/unknown zero (%s)',
  async (locale, expandName, running, zero, unavailable) => {
    const initial = state();
    initial.sessions = initial.sessions.map((session) =>
      session.id === 'child' ? { ...session, status: 'running' } : session,
    );
    const view = mount({ preferences: { theme: 'dark', locale } }, initial);
    await screen.findByText(running);
    const compact = screen.getByRole('button', { name: expandName });
    expect(compact.textContent).toContain(running);
    view.setState({ ...initial, sessions: initial.sessions.map((session) => ({ ...session, status: 'idle' })) });
    native.handlers.get('island:changed')?.();
    await screen.findByText(zero);
    view.setState({ ...initial, connection: 'error' });
    native.handlers.get('island:changed')?.();
    await screen.findByText(unavailable);
    expect(compact.textContent).not.toContain(running);
    expect(compact.textContent).not.toContain(zero);
  },
);

test('enabled=false blocks all session writes despite a malformed connected snapshot', async () => {
  const view = mount();
  await expand();
  const textbox = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
  await fireEvent.input(textbox, { target: { value: 'Do not send' } });
  view.setState({ ...state(), enabled: false });
  native.handlers.get('island:changed')?.();
  await screen.findByText('Disabled');
  expect((screen.getByRole('button', { name: 'Send' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.submit(textbox.closest('form') as HTMLFormElement);
  await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  await fireEvent.click(screen.getByRole('button', { name: /Running work/ }));
  expect((screen.getByRole('button', { name: 'Stop' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
  expect(view.config.islandSend).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
});

test('continuous invalidations flush on the first bounded deadline and serialize dirty reads without polling', async () => {
  const view = mount();
  await screen.findByText('1 pending');
  const previous = view.config.getIslandState.mock.calls.length;
  let finish!: (value: IslandState) => void;
  view.config.getIslandState.mockImplementationOnce(
    () =>
      new Promise<IslandState>((resolve) => {
        finish = resolve;
      }),
  );
  vi.useFakeTimers();
  try {
    for (let index = 0; index < 8; index += 1) {
      native.handlers.get('island:changed')?.();
      await vi.advanceTimersByTimeAsync(80);
      if (index === 2) expect(view.config.getIslandState).toHaveBeenCalledTimes(previous + 1);
    }
    expect(view.config.getIslandState).toHaveBeenCalledTimes(previous + 1);
    await vi.advanceTimersByTimeAsync(200);
    finish(state());
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    expect(view.config.getIslandState).toHaveBeenCalledTimes(previous + 2);
    await vi.advanceTimersByTimeAsync(1000);
    expect(view.config.getIslandState).toHaveBeenCalledTimes(previous + 2);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test.each([
  { code: 'conflict', message: 'Message ID already exists' },
  {
    code: 'configuration_failed',
    message: 'opencode island: API HTTP 409; write may have been accepted; do not retry.',
  },
])(
  '409 is never success: keeps draft/ID and requires another explicit refresh after conflict ($code)',
  async (failure) => {
    const islandSend = vi.fn().mockRejectedValue(failure);
    const view = mount({ islandSend });
    await expand();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Original draft' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await screen.findByText(
      'Message ID conflict (409). Refresh and check the session before retrying. Your draft and message ID are kept.',
    );
    expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('Original draft');
    expect((screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Retry same message' }));
    expect(islandSend).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() =>
      expect((screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement).disabled).toBe(false),
    );
    expect(view.config.refreshIsland).toHaveBeenCalledOnce();
    expect(islandSend).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole('button', { name: 'Retry same message' }));
    await waitFor(() => expect(islandSend).toHaveBeenCalledTimes(2));
    expect(islandSend.mock.calls[1]).toEqual(islandSend.mock.calls[0]);
    await waitFor(() =>
      expect((screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement).disabled).toBe(true),
    );
    expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('Original draft');
  },
);

test('multiple question fields send keys and option values; custom/multiselect and read-only skip remain explicit', async () => {
  const question = {
    id: 'form_1',
    sessionId: 'pending',
    title: 'Choose settings',
    answerable: true,
    fields: [
      {
        key: 'mode_key',
        type: 'string' as const,
        title: 'Mode',
        options: [{ value: 'fast_value', label: 'Fast label' }],
        custom: true,
        required: true,
      },
      {
        key: 'targets_key',
        type: 'multiselect' as const,
        title: 'Targets',
        options: [
          { value: 'web_value', label: 'Web label' },
          { value: 'cli_value', label: 'CLI label' },
        ],
        custom: true,
        required: true,
      },
      { key: 'note_key', type: 'string' as const, title: 'Note', options: [], custom: true, required: true },
    ],
  };
  const initial = { ...state(), questions: [question] };
  const view = mount({}, initial);
  await screen.findByText('2 pending');
  await expand();
  await screen.findByText('Choose settings');
  await fireEvent.click(screen.getByRole('button', { name: 'Submit answers' }));
  await screen.findByText('Answer all required questions.');
  expect(view.config.islandReplyQuestion).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('radio', { name: 'Fast label' }));
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Web label' }));
  await fireEvent.click(screen.getByRole('checkbox', { name: 'CLI label' }));
  await fireEvent.input(screen.getByRole('textbox', { name: 'Custom answer for Targets' }), {
    target: { value: 'desktop_value' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Add' }));
  const note = screen.getByRole('group', { name: 'Note · Required' });
  await fireEvent.click(within(note).getByRole('radio', { name: 'Custom answer' }));
  await fireEvent.input(screen.getByRole('textbox', { name: 'Custom answer for Note' }), {
    target: { value: 'My own text' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Submit answers' }));
  expect(view.config.islandReplyQuestion).toHaveBeenCalledExactlyOnceWith(
    'pending',
    'form_1',
    {
      mode_key: 'fast_value',
      targets_key: ['web_value', 'cli_value', 'desktop_value'],
      note_key: 'My own text',
    },
    0,
  );
  view.setState({ ...initial, questions: [{ ...question, id: 'form_readonly', answerable: false }] });
  native.handlers.get('island:changed')?.();
  await screen.findByText('This question cannot be answered here. Use the OpenCode terminal, or skip it.');
  expect(screen.queryByRole('button', { name: 'Submit answers' })).toBeNull();
  expect(screen.getByRole('radio', { name: 'Fast label' }).closest('fieldset')?.hasAttribute('disabled')).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Skip' }));
  expect(view.config.islandCancelQuestion).toHaveBeenCalledExactlyOnceWith('pending', 'form_readonly', 0);
  view.setState({ ...initial, questions: [] });
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(screen.queryByText('Choose settings')).toBeNull());
});

test('non-question/unsupported form projection is read-only even when the backend exposes no fields', async () => {
  const initial: IslandState = {
    ...state(),
    questions: [
      { id: 'form_non_question', sessionId: 'pending', title: 'Terminal-only form', answerable: false, fields: [] },
    ],
  };
  const view = mount({}, initial);
  await screen.findByText('2 pending');
  await expand();
  await screen.findByText('This question cannot be answered here. Use the OpenCode terminal, or skip it.');
  expect(screen.queryByRole('button', { name: 'Submit answers' })).toBeNull();
  expect(view.config.islandReplyQuestion).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Collapse island' }));
  await screen.findByRole('button', { name: 'Show quick area' });
  view.unmount();
  expect(view.config.islandCancelQuestion).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
});

test.each(['connecting', 'error', 'disabled'] as const)(
  'service %s never displays cached Idle status or allows writes',
  async (connection) => {
    const view = mount();
    await expand();
    view.setState({ ...state(), connection });
    native.handlers.get('island:changed')?.();
    await waitFor(() => expect(screen.queryByText('Idle')).toBeNull());
    expect(screen.getAllByText('Status unknown')).toHaveLength(3);
    expect((screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  },
);

test('invalidations coalesce cache reads independently of pending messages; selection/collapse reject stale messages and preserve drafts', async () => {
  const view = mount();
  await expand();
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Unsent text' } });
  let finish!: (messages: IslandMessage[]) => void;
  view.config.islandMessages.mockImplementationOnce(
    () =>
      new Promise<IslandMessage[]>((resolve) => {
        finish = resolve;
      }),
  );
  const previous = view.config.getIslandState.mock.calls.length;
  for (let index = 0; index < 10; index += 1) native.handlers.get('island:changed')?.();
  await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(previous + 1));
  await fireEvent.click(screen.getByRole('button', { name: /Running work/ }));
  for (let index = 0; index < 10; index += 1) native.handlers.get('island:changed')?.();
  await new Promise((resolve) => setTimeout(resolve, 250));
  expect(view.config.getIslandState).toHaveBeenCalledTimes(previous + 3);
  finish([{ id: 'stale', role: 'assistant', text: 'Stale pending message', time: 1, truncated: false }]);
  await screen.findByText('<b>running</b> Public text');
  expect(screen.queryByText('Stale pending message')).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: /Pending work/ }));
  await screen.findByText('<b>pending</b> Public text');
  expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('Unsent text');
  native.handlers.get('island:collapsed')?.();
  await screen.findByRole('button', { name: 'Show quick area' });
  const reads = view.config.islandMessages.mock.calls.length;
  native.handlers.get('app:preferences-changed')?.();
  await waitFor(() => expect(view.config.applyPreferences).toHaveBeenCalled());
  await new Promise((resolve) => setTimeout(resolve, 250));
  expect(view.config.islandMessages).toHaveBeenCalledTimes(reads);
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function backend(generation: number, title: string): IslandState {
  return {
    ...state(),
    connectionGeneration: generation,
    sessions: [{ id: 'ses_same', title, directory: '/code/same', updatedAt: 1, status: 'running' }],
    permissions: [
      { id: 'perm_same', sessionId: 'ses_same', action: 'bash', resources: ['git status'], save: ['git *'] },
    ],
    questions: [
      {
        id: 'question_same',
        sessionId: 'ses_same',
        title: 'Same question',
        answerable: true,
        fields: [{ key: 'note', type: 'string', title: 'Note', custom: true, required: true, options: [] }],
      },
    ],
  };
}

async function changeBackend(view: ReturnType<typeof mount>, next: IslandState) {
  const reads = view.config.getIslandState.mock.calls.length;
  view.setState(next);
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(reads + 1));
  await tick();
}

test('connected-to-connected generation change clears same-ID drafts, selected session and QuestionForm without view or focus changes', async () => {
  const view = mount({}, backend(4, 'Backend A'));
  await expand();
  const composer = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
  await fireEvent.input(composer, { target: { value: 'Only for A' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Submit answers' }));
  await screen.findByText('Answer all required questions.');
  await fireEvent.click(screen.getByRole('radio', { name: 'Custom answer' }));
  await fireEvent.input(screen.getByRole('textbox', { name: 'Custom answer for Note' }), {
    target: { value: 'A answer' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'More approval options' }));
  const oldAlways = screen.getByRole('button', { name: 'Always for this project' });
  const oldSubmit = screen.getByRole('button', { name: 'Submit answers' });
  const calls = view.config.setIslandView.mock.calls.length;
  const active = document.activeElement;
  view.config.islandMessages.mockResolvedValue([
    { id: 'msg_b', role: 'assistant', text: 'Backend B public', time: 1, truncated: false },
  ]);
  await changeBackend(view, backend(5, 'Backend B'));
  await screen.findByText('Backend B public');
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('expanded');
  expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
  expect(document.activeElement).toBe(active);
  expect(screen.getByRole('textbox', { name: 'Message' })).toBe(composer);
  expect(composer.value).toBe('');
  expect((screen.getByRole('textbox', { name: 'Custom answer for Note' }) as HTMLInputElement).value).toBe('');
  expect(screen.queryByText('Answer all required questions.')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Always for this project' })).toBeNull();
  expect(screen.queryByText('<b>ses_same</b> Public text')).toBeNull();
  await fireEvent.submit(composer.closest('form') as HTMLFormElement);
  await fireEvent.click(oldAlways);
  await fireEvent.click(oldSubmit);
  expect(view.config.islandSend).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandReplyQuestion).not.toHaveBeenCalled();
  // An older cache result cannot roll back the accepted epoch or its new draft.
  await fireEvent.input(composer, { target: { value: 'Only for B' } });
  await changeBackend(view, backend(4, 'Stale A'));
  expect(screen.queryByText('Stale A')).toBeNull();
  expect(composer.value).toBe('Only for B');
  expect(screen.getByRole('button', { name: /Backend B/ }).getAttribute('aria-current')).toBe('true');
  const disconnected = {
    ...backend(6, 'Backend C'),
    connection: 'connecting' as const,
    sessions: [],
    permissions: [],
    questions: [],
  };
  await changeBackend(view, disconnected);
  expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
  expect(screen.queryByText('Backend B public')).toBeNull();
  await changeBackend(view, backend(6, 'Backend C'));
  expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('');
});

test.each(['resolve', 'reject'] as const)(
  'new generation is read while old same-ID messages are pending; late %s cannot repaint or hide current loading',
  async (completion) => {
    const oldMessages = deferred<IslandMessage[]>();
    const newMessages = deferred<IslandMessage[]>();
    const view = mount({}, backend(1, 'Backend A'));
    await expand();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Private A draft' } });
    view.config.islandMessages.mockReturnValueOnce(oldMessages.promise).mockReturnValueOnce(newMessages.promise);
    await changeBackend(view, backend(1, 'Backend A'));
    await waitFor(() => expect(view.config.islandMessages).toHaveBeenCalledTimes(2));
    expect(view.config.islandMessages).toHaveBeenLastCalledWith('ses_same', 1);
    await changeBackend(view, backend(2, 'Backend B'));
    expect(view.config.islandMessages).toHaveBeenCalledTimes(3);
    expect(view.config.islandMessages).toHaveBeenLastCalledWith('ses_same', 2);
    expect(screen.queryByText('<b>ses_same</b> Public text')).toBeNull();
    expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('');
    if (completion === 'resolve')
      oldMessages.resolve([{ id: 'msg_same', role: 'assistant', text: 'Late A private', time: 1, truncated: false }]);
    else oldMessages.reject(new Error('Late A message failure'));
    await tick();
    expect(screen.queryByText('Late A private')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('region', { name: 'Recent messages and pending requests' }).getAttribute('aria-busy')).toBe(
      'true',
    );
    newMessages.resolve([{ id: 'msg_same', role: 'assistant', text: 'Current B public', time: 1, truncated: false }]);
    await screen.findByText('Current B public');
    expect(screen.getByRole('region', { name: 'Recent messages and pending requests' }).getAttribute('aria-busy')).toBe(
      'false',
    );
    expect(view.config.islandSend).not.toHaveBeenCalled();
  },
);

test.each(['resolve', 'reject'] as const)(
  'late old send %s cannot clear the new same-ID draft, restore uncertainty or unlock a pending new send',
  async (completion) => {
    const oldSend = deferred<void>();
    const newSend = deferred<void>();
    const islandSend = vi.fn().mockReturnValueOnce(oldSend.promise).mockReturnValueOnce(newSend.promise);
    const view = mount({ islandSend }, backend(1, 'Backend A'));
    await expand();
    const composer = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
    await fireEvent.input(composer, { target: { value: 'Send to A' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(islandSend).toHaveBeenCalledExactlyOnceWith('ses_same', 'Send to A', expect.any(String), 1);
    const oldId = islandSend.mock.calls[0][2];
    await changeBackend(view, backend(2, 'Backend B'));
    expect(composer.value).toBe('');
    expect(composer.disabled).toBe(false);
    await fireEvent.input(composer, { target: { value: 'Send to B' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(islandSend.mock.calls[1]).toEqual(['ses_same', 'Send to B', expect.any(String), 2]);
    expect(islandSend.mock.calls[1][2]).not.toBe(oldId);
    const reads = view.config.getIslandState.mock.calls.length;
    if (completion === 'resolve') oldSend.resolve();
    else oldSend.reject(new Error('Late A send error'));
    await tick();
    expect(composer.value).toBe('Send to B');
    expect(composer.disabled).toBe(true);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Retry same message' })).toBeNull();
    expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
    await fireEvent.submit(composer.closest('form') as HTMLFormElement);
    expect(islandSend).toHaveBeenCalledTimes(2);
    newSend.resolve();
    await waitFor(() => expect(composer.value).toBe(''));
  },
);

test.each([
  ['permission', 'resolve'],
  ['permission', 'reject'],
  ['question', 'resolve'],
  ['question', 'reject'],
  ['interrupt', 'resolve'],
  ['interrupt', 'reject'],
] as const)(
  'late old %s reply %s cannot restore errors, refresh or unlock a current same-ID action',
  async (action, completion) => {
    const oldReply = deferred<void>();
    const newReply = deferred<void>();
    const method =
      action === 'permission'
        ? 'islandReplyPermission'
        : action === 'question'
          ? 'islandCancelQuestion'
          : 'islandInterrupt';
    const reply = vi.fn().mockReturnValueOnce(oldReply.promise).mockReturnValueOnce(newReply.promise);
    const view = mount({ [method]: reply }, backend(1, 'Backend A'));
    await expand();
    const name = action === 'permission' ? 'Allow once' : action === 'question' ? 'Skip' : 'Stop';
    await fireEvent.click(screen.getByRole('button', { name }));
    expect(reply.mock.calls[0].at(-1)).toBe(1);
    await changeBackend(view, backend(2, 'Backend B'));
    const control = screen.getByRole('button', { name }) as HTMLButtonElement;
    expect(control.disabled).toBe(false);
    await fireEvent.click(control);
    expect(reply.mock.calls[1].at(-1)).toBe(2);
    const reads = view.config.getIslandState.mock.calls.length;
    if (completion === 'resolve') oldReply.resolve();
    else oldReply.reject(new Error('Late A action error'));
    await tick();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(control.disabled).toBe(true);
    expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
    expect(view.config.refreshIsland).not.toHaveBeenCalled();
    await fireEvent.click(control);
    expect(reply).toHaveBeenCalledTimes(2);
    newReply.resolve();
    await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(reads + 1));
  },
);

test.each(['send', 'permission', 'answer', 'skip', 'interrupt'] as const)(
  'stale displayed %s submits its own epoch before the new same-ID backend snapshot arrives; refresh never replays it',
  async (action) => {
    const method = {
      send: 'islandSend',
      permission: 'islandReplyPermission',
      answer: 'islandReplyQuestion',
      skip: 'islandCancelQuestion',
      interrupt: 'islandInterrupt',
    }[action];
    const mutation = vi
      .fn()
      .mockRejectedValueOnce({ code: 'conflict', message: 'Connection generation changed' })
      .mockResolvedValue(undefined);
    const view = mount({ [method]: mutation }, backend(11, 'Backend A'));
    await expand();
    const write = async (label: string) => {
      if (action === 'send') {
        await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: label } });
      } else if (action === 'answer') {
        await fireEvent.click(screen.getByRole('radio', { name: 'Custom answer' }));
        await fireEvent.input(screen.getByRole('textbox', { name: 'Custom answer for Note' }), {
          target: { value: label },
        });
      }
      const name = {
        send: 'Send',
        permission: 'Allow once',
        answer: 'Submit answers',
        skip: 'Skip',
        interrupt: 'Stop',
      }[action];
      await fireEvent.click(screen.getByRole('button', { name }));
    };
    // Native has changed, but the displayed snapshot still belongs to A.
    view.setState(backend(12, 'Backend B'));
    await write('Only A payload');
    expect(mutation).toHaveBeenCalledOnce();
    expect(mutation.mock.calls[0].at(-1)).toBe(11);
    native.handlers.get('island:changed')?.();
    await screen.findByRole('button', { name: /Backend B/ });
    await waitFor(() =>
      expect((screen.getByRole('button', { name: 'Refresh' }) as HTMLButtonElement).disabled).toBe(false),
    );
    expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe('');
    expect((screen.getByRole('textbox', { name: 'Custom answer for Note' }) as HTMLInputElement).value).toBe('');
    expect(mutation).toHaveBeenCalledOnce();
    await write('Only B payload');
    expect(mutation).toHaveBeenCalledTimes(2);
    expect(mutation.mock.calls[1].at(-1)).toBe(12);
    if (action === 'send') {
      expect(mutation.mock.calls[1][1]).toBe('Only B payload');
      expect(mutation.mock.calls[1][2]).not.toBe(mutation.mock.calls[0][2]);
    } else if (action === 'answer') {
      expect(mutation.mock.calls[1][2]).toEqual({ note: 'Only B payload' });
    }
  },
);

test('a pending old snapshot starts messages with its own epoch, never the unseen backend generation', async () => {
  const oldSnapshot = deferred<IslandState>();
  const oldMessages = deferred<IslandMessage[]>();
  const view = mount({}, backend(21, 'Backend A'));
  await expand();
  view.config.getIslandState.mockReturnValueOnce(oldSnapshot.promise);
  view.config.islandMessages.mockReturnValueOnce(oldMessages.promise);
  const reads = view.config.getIslandState.mock.calls.length;
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(reads + 1));
  view.setState(backend(22, 'Backend B'));
  oldSnapshot.resolve(backend(21, 'Backend A'));
  await waitFor(() => expect(view.config.islandMessages).toHaveBeenCalledTimes(2));
  expect(view.config.islandMessages).toHaveBeenLastCalledWith('ses_same', 21);
  await changeBackend(view, backend(22, 'Backend B'));
  expect(view.config.islandMessages).toHaveBeenLastCalledWith('ses_same', 22);
  oldMessages.reject({ code: 'conflict', message: 'Connection generation changed' });
  await tick();
  expect(screen.queryByRole('alert')).toBeNull();
  expect(view.config.islandMessages).toHaveBeenCalledTimes(3);
});

test('generation change reselects from the current snapshot rather than retaining a reused session ID and rejects a pending older cache read', async () => {
  const view = mount({}, { ...state(), connectionGeneration: 3 });
  await expand();
  await fireEvent.click(screen.getByRole('button', { name: /Recent work/ }));
  await screen.findByText('<b>recent</b> Public text');
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Old selected draft' } });
  await changeBackend(view, { ...state(), connectionGeneration: 4 });
  await screen.findByText('<b>pending</b> Public text');
  expect(screen.getByRole('button', { name: /Pending work/ }).getAttribute('aria-current')).toBe('true');
  await fireEvent.click(screen.getByRole('button', { name: /Recent work/ }));
  await screen.findByText('<b>recent</b> Public text');
  const composer = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
  expect(composer.value).toBe('');
  await fireEvent.input(composer, { target: { value: 'Current draft' } });
  const stale = deferred<IslandState>();
  view.config.getIslandState.mockReturnValueOnce(stale.promise);
  const reads = view.config.getIslandState.mock.calls.length;
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(reads + 1));
  stale.resolve({ ...backend(3, 'Stale old target'), enabled: false });
  await tick();
  expect(screen.queryByText('Stale old target')).toBeNull();
  expect(composer.value).toBe('Current draft');
  expect(screen.getByRole('button', { name: /Recent work/ }).getAttribute('aria-current')).toBe('true');
  expect(view.config.islandSend).not.toHaveBeenCalled();
});

test('generation switch preserves native hover and compact/peek nodes without auto-expansion, focus or message reads', async () => {
  const view = mount({}, backend(1, 'Backend A'));
  await peek();
  const header = screen.getByRole('button', { name: 'Show quick area' });
  const activity = view.container.querySelector('.activity');
  const active = document.activeElement;
  await changeBackend(view, backend(2, 'Backend B'));
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('peek');
  expect(screen.getByRole('button', { name: 'Show quick area' })).toBe(header);
  expect(view.container.querySelector('.activity')).toBe(activity);
  expect(document.activeElement).toBe(active);
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test.each([
  ['peek', 'resolve'],
  ['peek', 'reject'],
  ['expanded', 'resolve'],
  ['expanded', 'reject'],
] as const)(
  'generation reset rejects old pending %s grow %s without changing committed view or hover',
  async (growing, completion) => {
    const view = mount({}, backend(31, 'Backend A'));
    const pending = deferred<void>();
    if (growing === 'expanded') await peek();
    else await screen.findByText('2 pending');
    const committed = growing === 'expanded' ? 'peek' : 'compact';
    view.config.setIslandView.mockReturnValueOnce(pending.promise);
    await fireEvent.click(
      screen.getByRole('button', { name: growing === 'expanded' ? 'Expand sessions' : 'Show quick area' }),
    );
    const calls = view.config.setIslandView.mock.calls.length;
    const active = document.activeElement;
    await changeBackend(view, backend(32, 'Backend B'));
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe(committed);
    if (completion === 'resolve') pending.resolve();
    else pending.reject(new Error('Old grow failure'));
    await tick();
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe(committed);
    expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(active);
    expect(screen.queryByText('Old grow failure')).toBeNull();
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
    if (growing === 'peek') {
      await fireEvent.click(screen.getByRole('button', { name: 'Show quick area' }));
      await screen.findByRole('button', { name: 'Expand sessions' });
    }
    await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
    await screen.findByRole('textbox', { name: 'Message' });
    expect(view.config.islandMessages).toHaveBeenCalledExactlyOnceWith('ses_same', 32);
  },
);

test('send generation rejection immediately rereads cache without Refresh, event notification or payload replay', async () => {
  const islandSend = vi.fn().mockRejectedValueOnce({
    code: 'validation_failed',
    message: 'Mini Island is disabled or its connection changed',
  });
  const view = mount({ islandSend }, backend(41, 'Backend A'));
  await expand();
  const composer = screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
  await fireEvent.input(composer, { target: { value: 'Never replay this payload' } });
  view.setState(backend(42, 'Backend B'));
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await screen.findByRole('button', { name: /Backend B/ });
  expect(composer.value).toBe('');
  expect(islandSend).toHaveBeenCalledExactlyOnceWith('ses_same', 'Never replay this payload', expect.any(String), 41);
  expect(view.config.refreshIsland).not.toHaveBeenCalled();
  expect(screen.queryByRole('alert')).toBeNull();
});

test('browser mode has no native reads/listeners and late listener registrations are disposed', async () => {
  native.isTauri.mockReturnValue(false);
  const browser = mount();
  await tick();
  expect(native.listen).not.toHaveBeenCalled();
  expect(browser.config.getIslandState).not.toHaveBeenCalled();
  browser.unmount();
  native.isTauri.mockReturnValue(true);
  const finish: (() => void)[] = [];
  native.listen.mockImplementation((name: string, handler: () => void) => {
    native.handlers.set(name, handler);
    return new Promise<() => void>((resolve) => {
      finish.push(() => resolve(() => native.unlisten(name)));
    });
  });
  const view = mount();
  await tick();
  view.unmount();
  native.handlers.get('island:pointer-enter')?.();
  native.handlers.get('island:pointer-leave')?.();
  for (const register of finish) register();
  await waitFor(() => expect(native.unlisten).toHaveBeenCalledTimes(5));
  expect(native.unlisten).toHaveBeenCalledWith('island:pointer-enter');
  expect(native.unlisten).toHaveBeenCalledWith('island:pointer-leave');
  expect(view.config.getIslandState).not.toHaveBeenCalled();
  expect(view.config.setIslandView).not.toHaveBeenCalled();
});

test('transient reads keep session drafts; Escape during expansion sends compact before grow readback', async () => {
  const view = mount();
  await expand();
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), {
    target: { value: 'Keep across read errors' },
  });
  view.config.refreshIsland.mockRejectedValueOnce(new Error('Temporary read failure'));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByText('Temporary read failure');
  expect(screen.queryByText('Idle')).toBeNull();
  expect(screen.getAllByText('Status unknown')).toHaveLength(3);
  expect((screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement).value).toBe(
    'Keep across read errors',
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Collapse island' }));
  await screen.findByRole('button', { name: 'Show quick area' });
  await fireEvent.click(screen.getByRole('button', { name: 'Show quick area' }));
  await screen.findByRole('button', { name: 'Expand sessions' });
  let finish!: () => void;
  view.config.setIslandView.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  const messagesRead = view.config.islandMessages.mock.calls.length;
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact');
  await fireEvent.keyDown(window, { key: 'Escape' });
  const calls = view.config.setIslandView.mock.calls.length;
  finish();
  await tick();
  expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
  expect(view.config.islandMessages).toHaveBeenCalledTimes(messagesRead);
  await screen.findByRole('button', { name: 'Show quick area' });
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandCancelQuestion).not.toHaveBeenCalled();
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
});

test('compact child approval captures exact IDs, locks double clicks and queue, and never expands or loads messages', async () => {
  const initial = state();
  initial.notchWidth = 300;
  initial.permissions.push({
    id: 'perm_1',
    sessionId: 'child',
    action: 'edit',
    resources: ['/code/recent-project/src/<unsafe>.ts'],
  });
  let finish!: () => void;
  const islandReplyPermission = vi.fn().mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const view = mount({ islandReplyPermission }, initial);
  await peek();
  await screen.findByText('2 pending');
  expect(view.container.querySelector('main')?.getAttribute('style')).toContain('--compact-width: 400px');
  expect(view.container.querySelector('main')?.getAttribute('data-state')).toBe('attention');
  await fireEvent.click(screen.getByRole('button', { name: 'Next permission request' }));
  const preview = screen.getByText('/code/recent-project/src/<unsafe>.ts');
  expect(preview.closest('[title]')?.getAttribute('title')).toContain('/code/recent-project/src/<unsafe>.ts');
  expect(preview.querySelector('unsafe')).toBeNull();
  const allow = screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement;
  await fireEvent.click(allow);
  await fireEvent.click(allow);
  await fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
  expect(islandReplyPermission).toHaveBeenCalledExactlyOnceWith('child', 'perm_1', 'once', 0);
  expect(allow.disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Next permission request' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.blur(window);
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(view.container.querySelector('#compact-permission-details')).toBeTruthy();
  expect(view.config.setIslandView).not.toHaveBeenCalledWith('expanded');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  finish();
  await waitFor(() => expect(view.config.refreshIsland).toHaveBeenCalledOnce());
  await screen.findByText('Awaiting confirmation · Refresh to check');
  expect(allow.disabled).toBe(true);
  expect(islandReplyPermission).toHaveBeenCalledOnce();
  view.setState({
    ...initial,
    permissions: initial.permissions.filter((permission) => permission.sessionId !== 'child'),
  });
  native.handlers.get('island:changed')?.();
  await peek();
  await screen.findByText('git status');
  expect((screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement).disabled).toBe(false);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
});

test('compact queue identity survives incoming reorder; external replies advance and removed controls cannot write', async () => {
  const view = mount();
  await peek();
  const staleAllow = await screen.findByRole('button', { name: 'Allow once' });
  const newcomer = { id: 'perm_1', sessionId: 'child', action: 'edit', resources: ['/child/file.ts'] };
  view.setState({ ...state(), permissions: [newcomer, ...state().permissions] });
  native.handlers.get('island:changed')?.();
  await screen.findByText('Permission 2 of 2');
  expect(screen.getByText('git status')).toBeTruthy();
  expect(screen.queryByText('/child/file.ts')).toBeNull();
  view.setState({ ...state(), permissions: [newcomer] });
  native.handlers.get('island:changed')?.();
  await screen.findByText('/child/file.ts');
  await fireEvent.click(staleAllow);
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('child', 'perm_1', 'reject', 0);
  view.setState({ ...state(), permissions: [] });
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull());
  await screen.findByRole('region', { name: 'Quick area' });
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await waitFor(() =>
    expect(view.container.querySelector('main')?.getAttribute('style')).toContain('--compact-width: 292px'),
  );
  expect(view.config.setIslandView).not.toHaveBeenCalledWith('expanded');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test.each([{ enabled: false }, { connection: 'error' as const }, { connection: 'connecting' as const }])(
  'compact hides stale attention and blocks detached permission controls when unavailable ($enabled/$connection)',
  async (unavailable) => {
    const view = mount();
    await peek();
    const staleAllow = await screen.findByRole('button', { name: 'Allow once' });
    view.setState({ ...state(), ...unavailable });
    native.handlers.get('island:changed')?.();
    await waitFor(() => expect(screen.queryByRole('region', { name: 'Permission requested' })).toBeNull());
    await fireEvent.click(staleAllow);
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    expect(screen.getByRole('region', { name: 'Quick area' })).toBeTruthy();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  },
);

test('uncertain compact permission decisions require explicit Refresh, never blind or automatic retry', async () => {
  const view = mount({ islandReplyPermission: vi.fn().mockRejectedValue(new Error('Decision reply lost')) });
  await peek();
  await fireEvent.click(await screen.findByRole('button', { name: 'Allow once' }));
  await screen.findByRole('alert');
  expect(screen.getByRole('alert').textContent).toBe('Decision reply lost');
  expect((screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
  native.handlers.get('island:changed')?.();
  await new Promise((resolve) => setTimeout(resolve, 250));
  expect(view.config.islandReplyPermission).toHaveBeenCalledOnce();
  expect((screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Allow once' }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(screen.queryByRole('alert')).toBeNull();
  expect(view.config.islandReplyPermission).toHaveBeenCalledOnce();
  expect(view.config.refreshIsland).toHaveBeenCalledOnce();
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test('compact cross-session reply preserves selection and uncertain draft; automatic readback cannot confirm a send', async () => {
  const view = mount({ islandSend: vi.fn().mockRejectedValue(new Error('Send reply lost')) });
  await expand();
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Keep exact draft' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await screen.findByText('Message may have been accepted. Refresh before retrying.');
  const messageId = view.config.islandSend.mock.calls[0][2];
  await fireEvent.click(screen.getByRole('button', { name: 'Collapse island' }));
  await screen.findByRole('button', { name: 'Show quick area' });
  const permission = { id: 'perm_other', sessionId: 'child', action: 'read', resources: ['/child/resource'] };
  view.setState({ ...state(), permissions: [permission] });
  native.handlers.get('island:changed')?.();
  await peek();
  await screen.findByText('/child/resource');
  const previousReads = view.config.islandMessages.mock.calls.length;
  view.config.islandMessages.mockResolvedValue([
    { id: messageId, role: 'user', text: 'Keep exact draft', time: 1, truncated: false },
  ]);
  await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  await waitFor(() => expect(view.config.refreshIsland).toHaveBeenCalledOnce());
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('child', 'perm_other', 'once', 0);
  expect(view.config.islandMessages).toHaveBeenCalledTimes(previousReads);
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  const textbox = await screen.findByRole('textbox', { name: 'Message' });
  expect((textbox as HTMLTextAreaElement).value).toBe('Keep exact draft');
  expect((textbox as HTMLTextAreaElement).disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Retry same message' }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole('button', { name: /Pending work/ }).getAttribute('aria-current')).toBe('true');
});

test('compact activity animates only connected enabled running snapshots and questions stay a full-form hint', async () => {
  const initial = { ...state(), permissions: [] };
  const view = mount({}, initial);
  await screen.findByRole('button', { name: 'Show quick area' });
  await screen.findByText('1 running');
  expect(screen.queryByText('MOM')).toBeNull();
  const activity = view.container.querySelector('.activity');
  expect(activity?.getAttribute('data-running')).toBe('true');
  expect(view.container.querySelector('main')?.getAttribute('data-state')).toBe('running');
  view.setState({ ...initial, sessions: initial.sessions.map((session) => ({ ...session, status: 'idle' })) });
  native.handlers.get('island:changed')?.();
  await screen.findByText('0 running');
  expect(activity?.getAttribute('data-running')).toBe('false');
  expect(view.container.querySelector('main')?.getAttribute('data-state')).toBe('idle');
  view.setState({ ...initial, connection: 'error' });
  native.handlers.get('island:changed')?.();
  await screen.findByText('Connection unavailable');
  expect(activity?.getAttribute('data-running')).toBe('false');
  view.setState({ ...initial, enabled: false });
  native.handlers.get('island:changed')?.();
  await screen.findByText('Disabled');
  expect(activity?.getAttribute('data-running')).toBe('false');
  view.setState({
    ...initial,
    questions: [{ id: 'q_1', sessionId: 'child', title: 'Choose mode', answerable: true, fields: [] }],
  });
  native.handlers.get('island:changed')?.();
  await screen.findByText('1 pending');
  expect(screen.getByText('Open the island to answer pending questions.')).toBeTruthy();
  expect(screen.queryByText('Choose mode')).toBeNull();
  expect(view.container.querySelector('main')?.classList.contains('compact-attention')).toBe(false);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test('teardown ignores a late compact permission result without refresh, expansion, or further writes', async () => {
  let finish!: () => void;
  const view = mount({
    islandReplyPermission: vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    ),
  });
  await peek();
  await fireEvent.click(await screen.findByRole('button', { name: 'Allow once' }));
  const reads = view.config.getIslandState.mock.calls.length;
  view.unmount();
  finish();
  await tick();
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('pending', 'perm_1', 'once', 0);
  expect(view.config.refreshIsland).not.toHaveBeenCalled();
  expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
  expect(view.config.setIslandView).not.toHaveBeenCalledWith('expanded');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test('unavailable Refresh cannot unlock an ambiguous permission retry, even after an automatic reconnect', async () => {
  const view = mount({ islandReplyPermission: vi.fn().mockRejectedValue(new Error('Decision reply lost')) });
  await peek();
  await fireEvent.click(await screen.findByRole('button', { name: 'Allow once' }));
  await screen.findByRole('alert');
  view.setState({ ...state(), connection: 'error', permissions: [] });
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(screen.getByRole('status').textContent).toBe('Connection unavailable'));
  view.setState(state());
  native.handlers.get('island:changed')?.();
  await peek();
  const allow = (await screen.findByRole('button', { name: 'Allow once' })) as HTMLButtonElement;
  expect(allow.disabled).toBe(true);
  await fireEvent.click(allow);
  expect(view.config.islandReplyPermission).toHaveBeenCalledOnce();
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(allow.disabled).toBe(false));
  expect(view.config.islandReplyPermission).toHaveBeenCalledOnce();
});

test('compact has no branding or extra right controls; hover peeks without chat and leave returns to compact', async () => {
  const initial = state();
  const view = mount({}, initial);
  const header = await screen.findByRole('button', { name: 'Show quick area' });
  await screen.findByText('1 pending');
  expect(screen.queryByText('MOM')).toBeNull();
  expect(screen.queryByText('OpenCode')).toBeNull();
  expect(screen.queryByRole('navigation', { name: 'Sessions' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull();
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  expect(view.container.querySelector('main')?.getAttribute('style')).toContain('--compact-width: 292px');
  const status = screen.getByRole('button', { name: 'Session status · Show quick area' });
  expect(status.getAttribute('aria-describedby')).toBe('compact-status-hint compact-counts-hint compact-pending-hint');
  expect(header.querySelector('.wing-label')?.textContent).toBe('Running');
  expect(header.querySelector('.wing-count')?.textContent).toBe('1');
  expect(view.container.querySelectorAll('.compact-lane button')).toHaveLength(1);
  expect(
    view.container.querySelectorAll('.compact-lane svg, .compact-lane .pending-badge, .question-badge'),
  ).toHaveLength(0);
  expect(view.container.querySelectorAll('.activity')).toHaveLength(1);
  expect(view.container.querySelector('#compact-pending-hint')?.textContent).toBe('1 pending');
  expect(view.config.setIslandView).not.toHaveBeenCalled();
  await peek();
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.container.querySelector('main')?.getAttribute('style')).toContain('--compact-width: 400px');
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await waitFor(() => expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact'));
  expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull();
  expect(screen.getByText('1 pending')).toBeTruthy();
});

test('100+ pending requests reuse the single activity visual and retain accessible count without expansion', async () => {
  const initial = state();
  initial.permissions = Array.from({ length: 105 }, (_, index) => ({
    ...initial.permissions[0],
    id: `perm_${index}`,
  }));
  const view = mount({}, initial);
  await screen.findByText('105 pending');
  expect(view.container.querySelector('.activity')?.classList.contains('pending')).toBe(true);
  expect(view.container.querySelectorAll('.activity')).toHaveLength(1);
  expect(view.container.querySelector('.compact-status .pending-badge')).toBeNull();
  expect(view.container.querySelector('#compact-pending-hint')?.textContent).toBe('105 pending');
  expect(view.config.setIslandView).not.toHaveBeenCalled();
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test.each(['Show quick area', 'Session status · Show quick area'])(
  'compact click on %s requests only peek with a single four-bar visual and no chat',
  async (control) => {
    const view = mount({}, { ...state(), permissions: [] });
    await screen.findByText('1 running');
    expect(view.container.querySelector('.compact-left .wing-label')?.textContent).toBe('Running');
    expect(view.container.querySelector('.compact-left .wing-count')?.textContent).toBe('1');
    expect(screen.queryByText('MOM')).toBeNull();
    expect(view.container.querySelectorAll('.compact-lane button')).toHaveLength(1);
    expect(view.container.querySelectorAll('.compact-lane .activity')).toHaveLength(1);
    expect(view.container.querySelectorAll('.compact-lane .activity i')).toHaveLength(4);
    expect(view.container.querySelectorAll('.compact-header svg')).toHaveLength(0);
    await fireEvent.click(screen.getByRole('button', { name: control }));
    await screen.findByRole('region', { name: 'Quick area' });
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    expect(screen.getAllByRole('button', { name: 'Expand sessions' })).toHaveLength(1);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  },
);

test.each([true, false])(
  'the exposed compact center receives pointer move with permissions=%s and peeks without reads or focus',
  async (pending) => {
    const initial = { ...state(), permissions: pending ? state().permissions : [] };
    const view = mount({}, initial);
    await screen.findByText('1 running');
    const main = screen.getByRole('main', { name: 'Sessions' });
    const active = document.activeElement;
    const reads = view.config.getIslandState.mock.calls.length;
    const gap = main.querySelector('.notch-gap') as HTMLElement;
    expect(gap.getAttribute('aria-hidden')).toBe('true');
    expect(gap.querySelector('button, input, textarea, [tabindex]')).toBeNull();
    vi.useFakeTimers();
    try {
      await fireEvent.pointerMove(main, { clientX: 146, clientY: 16 });
      await vi.advanceTimersByTimeAsync(60);
      await fireEvent.pointerMove(main, { clientX: 147, clientY: 16 });
      await vi.advanceTimersByTimeAsync(39);
      expect(view.config.setIslandView).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      expect(main.dataset.view).toBe('peek');
      expect(document.activeElement).toBe(active);
      expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
      await fireEvent.pointerLeave(main);
      await vi.advanceTimersByTimeAsync(179);
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

test('center pointer activation opens only peek and internal wing crossings cannot cancel the main hover deadline', async () => {
  const view = mount();
  await screen.findByText('1 pending');
  const main = screen.getByRole('main', { name: 'Sessions' });
  const shell = main.querySelector('.compact-shell') as HTMLElement;
  const left = main.querySelector('.compact-left') as HTMLElement;
  await fireEvent.pointerUp(main.querySelector('.notch-gap') as HTMLElement, { pointerType: 'touch' });
  await screen.findByRole('button', { name: 'Expand sessions' });
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  native.handlers.get('island:collapsed')?.();
  await tick();
  view.config.setIslandView.mockClear();
  vi.useFakeTimers();
  try {
    await fireEvent.pointerEnter(main);
    await vi.advanceTimersByTimeAsync(50);
    await fireEvent.pointerLeave(left);
    await fireEvent.pointerLeave(shell);
    await fireEvent.pointerMove(main, { clientX: 146, clientY: 16 });
    await vi.advanceTimersByTimeAsync(50);
    await tick();
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test.each(['Enter', ' '])(
  'native %s activation opens peek first and only its disclosure opens sessions',
  async (key) => {
    const view = mount();
    await screen.findByText('1 pending');
    const status = screen.getByRole('button', { name: 'Session status · Show quick area' });
    status.focus();
    await fireEvent.keyDown(status, { key });
    await fireEvent.keyUp(status, { key });
    // jsdom does not synthesize the native keyboard click; Chrome proof covers the default action.
    await fireEvent.click(status, { detail: 0 });
    await screen.findByRole('button', { name: 'Expand sessions' });
    expect(document.activeElement).toBe(status);
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    const disclosure = screen.getByRole('button', { name: 'Expand sessions' });
    disclosure.focus();
    await fireEvent.keyDown(disclosure, { key });
    await fireEvent.keyUp(disclosure, { key });
    expect(view.config.setIslandView).not.toHaveBeenCalledWith('expanded');
    await fireEvent.click(disclosure, { detail: 0 });
    await screen.findByRole('navigation', { name: 'Sessions' });
    expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['expanded']]);
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  },
);

test('keyboard focus and compact click reveal semantic sibling controls without selecting or authorizing', async () => {
  const view = mount();
  await screen.findByText('1 pending');
  const details = await screen.findByRole('button', { name: 'Show quick area' });
  expect(details.getAttribute('aria-expanded')).toBe('false');
  expect(details.getAttribute('aria-controls')).toBe('compact-permission-details');
  expect(screen.getByRole('button', { name: 'Show quick area' }).getAttribute('aria-controls')).toBe(
    'compact-permission-details',
  );
  details.focus();
  await screen.findByRole('button', { name: 'Allow once' });
  expect(document.activeElement).toBe(details);
  expect(details.getAttribute('aria-expanded')).toBe('true');
  const allow = screen.getByRole('button', { name: 'Allow once' });
  const reject = screen.getByRole('button', { name: 'Reject' });
  expect(allow.parentElement?.closest('button')).toBeNull();
  expect(reject.parentElement?.closest('button')).toBeNull();
  allow.focus();
  await tick();
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await new Promise((resolve) => setTimeout(resolve, 260));
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  allow.blur();
  await waitFor(() => expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact'));
  await fireEvent.click(details);
  await screen.findByRole('button', { name: 'Allow once' });
  expect(details.getAttribute('aria-expanded')).toBe('true');
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test.each([undefined, []])('Always is absent without server save rules (%s)', async (save) => {
  const initial = state();
  initial.permissions = initial.permissions.map((permission) => ({ ...permission, save }));
  const view = mount({}, initial);
  await peek();
  expect(screen.queryByRole('button', { name: 'More approval options' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Always for this project', hidden: true })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  await screen.findByRole('navigation', { name: 'Sessions' });
  expect(screen.queryByRole('button', { name: 'More approval options' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Always for this project', hidden: true })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('pending', 'perm_1', 'once', 0);
});

test('hover opens Always dropdown without approval; explicit click captures exact child IDs and guards duplicates', async () => {
  const initial = state();
  initial.permissions.push({
    id: 'perm_1',
    sessionId: 'child',
    action: 'edit',
    resources: ['/child/file'],
    save: ['/child/*'],
  });
  let finish!: () => void;
  const islandReplyPermission = vi.fn().mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const view = mount({ islandReplyPermission }, initial);
  await peek();
  await fireEvent.click(screen.getByRole('button', { name: 'Next permission request' }));
  const options = screen.getByRole('button', { name: 'More approval options' });
  expect(options.getAttribute('aria-expanded')).toBe('false');
  const dropdown = options.closest('.approval-options') as HTMLElement;
  await fireEvent.pointerEnter(dropdown);
  let always = await screen.findByRole('button', { name: 'Always for this project' });
  expect(options.getAttribute('aria-expanded')).toBe('true');
  expect(always.getAttribute('aria-describedby')).toBe('approval-scope-child/perm_1 approval-patterns-child/perm_1');
  const scope = dropdown.querySelector('.approval-scope') as HTMLElement;
  expect(scope.textContent).toContain('Save edit rules in this OpenCode Project');
  expect((within(scope).getByRole('textbox') as HTMLTextAreaElement).value).toBe('/child/*');
  expect(document.getElementById('approval-patterns-child/perm_1')?.textContent).toBe('/child/*');
  expect(scope.textContent).toContain('May also approve this Project’s matching pending requests.');
  expect(within(scope).getByRole('textbox').getAttribute('readonly')).not.toBeNull();
  expect(scope.textContent).not.toContain('/child/file');
  expect(scope.classList.contains('sr-only')).toBe(false);
  expect(islandReplyPermission).not.toHaveBeenCalled();
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(options.getAttribute('aria-expanded')).toBe('false');
  await waitFor(() => expect(document.activeElement).toBe(options));
  expect(view.config.setIslandView).not.toHaveBeenCalledWith('expanded');
  await fireEvent.click(options);
  always = await screen.findByRole('button', { name: 'Always for this project' });
  await fireEvent.click(always);
  await fireEvent.click(always);
  await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  expect(islandReplyPermission).toHaveBeenCalledExactlyOnceWith('child', 'perm_1', 'always', 0);
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await new Promise((resolve) => setTimeout(resolve, 260));
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  finish();
  await waitFor(() => expect(view.config.refreshIsland).toHaveBeenCalledOnce());
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.setIslandView).not.toHaveBeenCalledWith('expanded');
});

test('updated save rules remove Always controls and detached menu items cannot authorize', async () => {
  const initial = state();
  initial.permissions = initial.permissions.map((permission) => ({ ...permission, save: ['git *'] }));
  const view = mount({}, initial);
  await peek();
  await fireEvent.click(screen.getByRole('button', { name: 'More approval options' }));
  const staleAlways = await screen.findByRole('button', { name: 'Always for this project' });
  view.setState({ ...initial, permissions: initial.permissions.map((permission) => ({ ...permission, save: [] })) });
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(screen.queryByRole('button', { name: 'More approval options' })).toBeNull());
  await fireEvent.click(staleAlways);
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('peek readback failure preserves compact and pending request; compact click can retry without writes', async () => {
  const view = mount({
    setIslandView: vi.fn().mockRejectedValueOnce(new Error('Resize failed')).mockResolvedValue(undefined),
  });
  const details = await screen.findByRole('button', { name: 'Show quick area' });
  await fireEvent.pointerEnter(screen.getByRole('main', { name: 'Sessions' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Session status · Show quick area' }).getAttribute('title')).toBe(
      'Resize failed',
    ),
  );
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  expect(details.getAttribute('aria-expanded')).toBe('false');
  expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull();
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  await fireEvent.click(details);
  await screen.findByRole('button', { name: 'Allow once' });
  expect(view.config.setIslandView).toHaveBeenCalledTimes(2);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test('late peek readback after pointer leave returns native view to compact without reading messages', async () => {
  let finish!: () => void;
  const setIslandView = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    )
    .mockResolvedValue(undefined);
  const view = mount({ setIslandView });
  await screen.findByRole('button', { name: 'Show quick area' });
  const compact = screen.getByRole('main', { name: 'Sessions' });
  await fireEvent.pointerEnter(compact);
  await waitFor(() => expect(setIslandView).toHaveBeenCalledExactlyOnceWith('peek'));
  await fireEvent.pointerLeave(compact);
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  finish();
  await waitFor(() => expect(setIslandView).toHaveBeenLastCalledWith('compact'));
  expect(setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('resize pointer leave retains the 180 ms grace period so entering the new peek body does not collapse it', async () => {
  let finish!: () => void;
  const setIslandView = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    )
    .mockResolvedValue(undefined);
  const view = mount({ setIslandView });
  await screen.findByText('1 pending');
  const shell = screen.getByRole('main', { name: 'Sessions' });
  await fireEvent.pointerEnter(shell);
  await waitFor(() => expect(setIslandView).toHaveBeenCalledExactlyOnceWith('peek'));
  vi.useFakeTimers();
  try {
    await fireEvent.pointerLeave(shell);
    finish();
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('peek');
    await vi.advanceTimersByTimeAsync(179);
    expect(setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    await fireEvent.pointerEnter(shell);
    await vi.advanceTimersByTimeAsync(200);
    expect(setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test('compact click during pending hover resize stays peek; only its explicit disclosure opens expanded', async () => {
  let finish!: () => void;
  const setIslandView = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    )
    .mockResolvedValue(undefined);
  const view = mount({ setIslandView });
  await screen.findByRole('button', { name: 'Show quick area' });
  await fireEvent.pointerEnter(screen.getByRole('main', { name: 'Sessions' }));
  await waitFor(() => expect(setIslandView).toHaveBeenCalledExactlyOnceWith('peek'));
  const header = screen.getByRole('button', { name: 'Show quick area' });
  expect(header.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(header);
  await fireEvent.focusIn(header);
  finish();
  await screen.findByRole('button', { name: 'Allow once' });
  expect(setIslandView.mock.calls).toEqual([['peek']]);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  const disclosure = screen.getByRole('button', { name: 'Expand sessions' });
  await fireEvent.pointerEnter(disclosure);
  await new Promise((resolve) => setTimeout(resolve, 140));
  expect(setIslandView.mock.calls).toEqual([['peek']]);
  await fireEvent.click(disclosure);
  await screen.findByRole('navigation', { name: 'Sessions' });
  expect(setIslandView.mock.calls).toEqual([['peek'], ['expanded']]);
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('late header hover/focus during explicit expanded resize cannot replace the expanded request', async () => {
  const view = mount();
  await peek();
  let finish!: () => void;
  view.config.setIslandView.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const shell = screen.getByRole('main', { name: 'Sessions' });
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  await fireEvent.pointerEnter(shell);
  await fireEvent.focusIn(screen.getByRole('button', { name: 'Show quick area' }));
  native.handlers.get('island:pointer-enter')?.();
  native.handlers.get('island:pointer-leave')?.();
  await new Promise((resolve) => setTimeout(resolve, 130));
  expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['expanded']]);
  finish();
  await screen.findByRole('navigation', { name: 'Sessions' });
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('expanded');
  expect(view.config.setIslandView).toHaveBeenLastCalledWith('expanded');
});

test.each([
  [0, 0, 200, 28],
  [180, 32, 292, 32],
  [300, 24, 412, 28],
  [40, 40, 200, 40],
])(
  'compact matches authoritative native geometry (%s notch / %s inset)',
  async (notchWidth, topInset, width, height) => {
    const view = mount({}, { ...state(), notchWidth, topInset });
    await screen.findByText('1 pending');
    const main = view.container.querySelector('main') as HTMLElement;
    expect(main.style.getPropertyValue('--compact-width')).toBe(`${width}px`);
    expect(main.style.getPropertyValue('--compact-height')).toBe(`${height}px`);
    expect(main.dataset.view).toBe('compact');
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    if (notchWidth && topInset) {
      expect(main.querySelector(':scope > .notch-gap')).toBeNull();
      const gap = main.querySelector('.notch-gap') as HTMLElement;
      expect(gap.previousElementSibling?.classList.contains('compact-left')).toBe(true);
      expect(gap.nextElementSibling?.classList.contains('compact-lane')).toBe(true);
      expect(gap.getAttribute('aria-hidden')).toBe('true');
      expect(gap.children).toHaveLength(0);
    }
  },
);

test('hover without permissions reveals only cached quick status, body Refresh and question hint; disclosure click reads chat', async () => {
  const initial = {
    ...state(),
    permissions: [],
    questions: [{ id: 'q_1', sessionId: 'child', title: 'Private full form', answerable: true, fields: [] }],
  };
  const view = mount({}, initial);
  await screen.findByText('1 pending');
  const active = document.activeElement;
  const reads = view.config.getIslandState.mock.calls.length;
  await fireEvent.pointerEnter(screen.getByRole('main', { name: 'Sessions' }));
  const quick = await screen.findByRole('region', { name: 'Quick area' });
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(within(quick).getByRole('status').textContent).toBe('1 running');
  expect(within(quick).getByText('Open the island to answer pending questions.')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Refresh' })).toBeTruthy();
  expect(screen.queryByText('Private full form')).toBeNull();
  expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
  expect(screen.queryByRole('navigation', { name: 'Sessions' })).toBeNull();
  expect(document.activeElement).toBe(active);
  expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.islandReplyQuestion).not.toHaveBeenCalled();
  const disclosure = screen.getByRole('button', { name: 'Expand sessions' });
  await fireEvent.pointerEnter(disclosure);
  await fireEvent.click(view.container.querySelector('.compact-header') as HTMLElement);
  await new Promise((resolve) => setTimeout(resolve, 140));
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(within(quick).getByRole('button', { name: 'Refresh' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Open sessions' })).toBeNull();
  await fireEvent.click(disclosure);
  await screen.findByRole('navigation', { name: 'Sessions' });
  await waitFor(() => expect(view.config.islandMessages).toHaveBeenCalledWith('child', 0));
  expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['expanded']]);
});

test('pending arrival stays compact; short hover and teardown cancel the 100 ms entry timer', async () => {
  const view = mount({}, { ...state(), permissions: [] });
  await screen.findByText('1 running');
  view.setState(state());
  native.handlers.get('island:changed')?.();
  await screen.findByText('1 pending');
  expect(view.config.setIslandView).not.toHaveBeenCalled();
  expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull();
  vi.useFakeTimers();
  try {
    const compact = screen.getByRole('main', { name: 'Sessions' });
    await fireEvent.pointerEnter(compact);
    await vi.advanceTimersByTimeAsync(99);
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    await fireEvent.pointerLeave(compact);
    await vi.advanceTimersByTimeAsync(200);
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    await fireEvent.pointerEnter(compact);
    view.unmount();
    await vi.advanceTimersByTimeAsync(200);
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    vi.useRealTimers();
  }
});

test('focus without permissions still opens general peek when pointer leaves during the entry delay', async () => {
  const view = mount({}, { ...state(), permissions: [] });
  const details = await screen.findByRole('button', { name: 'Show quick area' });
  details.focus();
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await screen.findByRole('region', { name: 'Quick area' });
  expect(document.activeElement).toBe(details);
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  details.blur();
  await waitFor(() => expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact'));
});

test.each(['collapse', 'unmount'])('late peek cannot revive UI after authoritative %s', async (end) => {
  let finish!: () => void;
  const setIslandView = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    )
    .mockResolvedValue(undefined);
  const view = mount({ setIslandView });
  await screen.findByText('1 pending');
  await fireEvent.pointerEnter(screen.getByRole('main', { name: 'Sessions' }));
  await waitFor(() => expect(setIslandView).toHaveBeenCalledExactlyOnceWith('peek'));
  if (end === 'unmount') view.unmount();
  else native.handlers.get('island:collapsed')?.();
  finish();
  await tick();
  if (end === 'unmount') expect(setIslandView).toHaveBeenCalledOnce();
  else {
    expect(setIslandView.mock.calls).toEqual([['peek']]);
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
    expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull();
  }
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('local blur/leave protects busy peek, but authoritative collapse wins and never retries or cancels its reply', async () => {
  let finish!: () => void;
  const view = mount({
    islandReplyPermission: vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    ),
  });
  await peek();
  await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await fireEvent.blur(window);
  await new Promise((resolve) => setTimeout(resolve, 210));
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('peek');
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  native.handlers.get('island:collapsed')?.();
  await tick();
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  finish();
  await waitFor(() => expect(view.config.refreshIsland).toHaveBeenCalledOnce());
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('pending', 'perm_1', 'once', 0);
  expect(view.config.islandCancelQuestion).not.toHaveBeenCalled();
  expect(view.config.islandInterrupt).not.toHaveBeenCalled();
  expect(view.config.islandMessages).not.toHaveBeenCalled();
});

test('open Always menu prevents hover-leave shrink; click toggles, Escape returns focus before compact', async () => {
  const initial = state();
  initial.permissions[0].save = ['git *'];
  const view = mount({}, initial);
  await peek();
  const options = screen.getByRole('button', { name: 'More approval options' });
  await fireEvent.click(options);
  await screen.findByRole('button', { name: 'Always for this project' });
  await fireEvent.pointerLeave(screen.getByRole('main', { name: 'Sessions' }));
  await new Promise((resolve) => setTimeout(resolve, 210));
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  await fireEvent.click(options);
  expect(screen.queryByRole('button', { name: 'Always for this project' })).toBeNull();
  await fireEvent.click(options);
  await fireEvent.keyDown(window, { key: 'Escape' });
  await waitFor(() => expect(document.activeElement).toBe(options));
  expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
  await fireEvent.keyDown(window, { key: 'Escape' });
  await waitFor(() => expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact'));
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('expanded permission has the same explicit Always menu, visible patterns and live removal guard', async () => {
  const initial = state();
  initial.permissions[0].save = ['git *', '/very/long/project/path/**'];
  const view = mount({}, initial);
  await expand();
  const options = screen.getByRole('button', { name: 'More approval options' });
  await fireEvent.pointerEnter(options.closest('.approval-options') as HTMLElement);
  const staleAlways = await screen.findByRole('button', { name: 'Always for this project' });
  expect(
    (screen.getByRole('textbox', { name: 'Save bash rules in this OpenCode Project' }) as HTMLTextAreaElement).value,
  ).toBe('git * · /very/long/project/path/**');
  expect(screen.getByText('Save bash rules in this OpenCode Project')).toBeTruthy();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  await fireEvent.keyDown(window, { key: 'Escape' });
  await waitFor(() => expect(document.activeElement).toBe(options));
  expect(screen.getByRole('navigation', { name: 'Sessions' })).toBeTruthy();
  await fireEvent.click(options);
  view.setState({ ...initial, permissions: [] });
  native.handlers.get('island:changed')?.();
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Allow once' })).toBeNull());
  await fireEvent.click(staleAlways);
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  view.setState(initial);
  native.handlers.get('island:changed')?.();
  await fireEvent.click(await screen.findByRole('button', { name: 'More approval options' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Always for this project' }));
  expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('pending', 'perm_1', 'always', 0);
});

test.each([true, false])(
  'native hover alone with permissions=%s opens at 100 ms and leaves at 180 ms without reads, writes or focus',
  async (pending) => {
    const view = mount({}, { ...state(), permissions: pending ? state().permissions : [] });
    await screen.findByText('1 running');
    const enter = native.handlers.get('island:pointer-enter');
    const leave = native.handlers.get('island:pointer-leave');
    expect(enter).toBeTypeOf('function');
    expect(leave).toBeTypeOf('function');
    const reads = view.config.getIslandState.mock.calls.length;
    const active = document.activeElement;
    const main = screen.getByRole('main', { name: 'Sessions' });
    await fireEvent.blur(window);
    vi.useFakeTimers();
    try {
      enter?.();
      await vi.advanceTimersByTimeAsync(60);
      enter?.();
      await vi.advanceTimersByTimeAsync(39);
      expect(view.config.setIslandView).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      expect(main.dataset.view).toBe('peek');
      enter?.();
      await fireEvent.blur(window);
      await vi.advanceTimersByTimeAsync(500);
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      leave?.();
      await vi.advanceTimersByTimeAsync(179);
      expect(main.dataset.view).toBe('peek');
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(main.dataset.view).toBe('compact');
      expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
      expect(document.activeElement).toBe(active);
      expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
      expect(view.config.refreshIsland).not.toHaveBeenCalled();
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      for (const mutation of [
        view.config.islandSend,
        view.config.islandReplyPermission,
        view.config.islandReplyQuestion,
        view.config.islandCancelQuestion,
        view.config.islandInterrupt,
      ])
        expect(mutation).not.toHaveBeenCalled();
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

test.each(['native', 'dom'] as const)(
  '%s leave cannot cancel entry or compact while the other hover source remains inside',
  async (firstLeave) => {
    const view = mount();
    await screen.findByText('1 pending');
    const main = screen.getByRole('main', { name: 'Sessions' });
    vi.useFakeTimers();
    try {
      native.handlers.get('island:pointer-enter')?.();
      await vi.advanceTimersByTimeAsync(60);
      await fireEvent.pointerEnter(main);
      if (firstLeave === 'native') native.handlers.get('island:pointer-leave')?.();
      else await fireEvent.pointerLeave(main);
      await vi.advanceTimersByTimeAsync(39);
      expect(view.config.setIslandView).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      await vi.advanceTimersByTimeAsync(400);
      expect(main.dataset.view).toBe('peek');
      if (firstLeave === 'native') await fireEvent.pointerLeave(main);
      else native.handlers.get('island:pointer-leave')?.();
      await vi.advanceTimersByTimeAsync(179);
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
      expect(view.config.getIslandState).toHaveBeenCalledOnce();
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

test('DOM leave during native peek resize cannot shrink the connected native hover surface', async () => {
  let finish!: () => void;
  const view = mount({
    setIslandView: vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValue(undefined),
  });
  await screen.findByText('1 pending');
  const main = screen.getByRole('main', { name: 'Sessions' });
  vi.useFakeTimers();
  try {
    native.handlers.get('island:pointer-enter')?.();
    await fireEvent.pointerEnter(main);
    await vi.advanceTimersByTimeAsync(100);
    await fireEvent.pointerLeave(main);
    await vi.advanceTimersByTimeAsync(250);
    finish();
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    await vi.advanceTimersByTimeAsync(400);
    expect(main.dataset.view).toBe('peek');
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    native.handlers.get('island:pointer-leave')?.();
    await vi.advanceTimersByTimeAsync(180);
    expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test.each(['menu', 'focus'] as const)('native leave retains %s protection until it is released', async (protection) => {
  const initial = state();
  initial.permissions[0].save = ['git *'];
  const view = mount({}, initial);
  await screen.findByText('1 pending');
  const main = screen.getByRole('main', { name: 'Sessions' });
  vi.useFakeTimers();
  try {
    native.handlers.get('island:pointer-enter')?.();
    await vi.advanceTimersByTimeAsync(100);
    await tick();
    const control = screen.getByRole('button', {
      name: protection === 'menu' ? 'More approval options' : 'Show quick area',
    });
    if (protection === 'menu') await fireEvent.click(control);
    else {
      control.focus();
      await tick();
    }
    native.handlers.get('island:pointer-leave')?.();
    await vi.advanceTimersByTimeAsync(400);
    expect(main.dataset.view).toBe('peek');
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    if (protection === 'menu') await fireEvent.click(control);
    else {
      control.blur();
      await tick();
    }
    await vi.advanceTimersByTimeAsync(179);
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    await vi.advanceTimersByTimeAsync(1);
    await tick();
    expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
    expect(view.config.getIslandState).toHaveBeenCalledOnce();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test('native leave preserves a busy reply and checking readback, then shrinks only after confirmation', async () => {
  let finishReply!: () => void;
  let finishRead!: (value: IslandState) => void;
  const view = mount({
    islandReplyPermission: vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finishReply = resolve;
        }),
    ),
  });
  await screen.findByText('1 pending');
  view.config.getIslandState.mockImplementationOnce(
    () =>
      new Promise<IslandState>((resolve) => {
        finishRead = resolve;
      }),
  );
  const main = screen.getByRole('main', { name: 'Sessions' });
  vi.useFakeTimers();
  try {
    native.handlers.get('island:pointer-enter')?.();
    await vi.advanceTimersByTimeAsync(100);
    await tick();
    await fireEvent.click(screen.getByRole('button', { name: 'Allow once' }));
    native.handlers.get('island:pointer-leave')?.();
    await vi.advanceTimersByTimeAsync(400);
    expect(main.dataset.view).toBe('peek');
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    finishReply();
    await vi.advanceTimersByTimeAsync(0);
    expect(view.config.refreshIsland).toHaveBeenCalledOnce();
    expect(view.config.getIslandState).toHaveBeenCalledTimes(2);
    expect(screen.getByRole('region', { name: 'Permission requested' }).getAttribute('aria-busy')).toBe('true');
    await vi.advanceTimersByTimeAsync(400);
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    finishRead({ ...state(), permissions: [] });
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    await vi.advanceTimersByTimeAsync(179);
    expect(main.dataset.view).toBe('peek');
    await vi.advanceTimersByTimeAsync(1);
    await tick();
    expect(main.dataset.view).toBe('compact');
    expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact']]);
    expect(view.config.islandReplyPermission).toHaveBeenCalledExactlyOnceWith('pending', 'perm_1', 'once', 0);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandInterrupt).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test.each(['timer', 'resize'] as const)(
  'authoritative collapse clears both hover sources and beats pending native %s',
  async (pending) => {
    let finish!: () => void;
    const setIslandView = vi.fn().mockResolvedValue(undefined);
    if (pending === 'resize')
      setIslandView.mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            finish = resolve;
          }),
      );
    const view = mount({ setIslandView });
    await screen.findByText('1 pending');
    const main = screen.getByRole('main', { name: 'Sessions' });
    vi.useFakeTimers();
    try {
      native.handlers.get('island:pointer-enter')?.();
      await fireEvent.pointerEnter(main);
      await vi.advanceTimersByTimeAsync(pending === 'timer' ? 99 : 100);
      native.handlers.get('island:collapsed')?.();
      await tick();
      expect(main.dataset.view).toBe('compact');
      if (pending === 'resize') finish();
      await vi.advanceTimersByTimeAsync(500);
      await tick();
      expect(main.dataset.view).toBe('compact');
      expect(view.config.setIslandView.mock.calls).toEqual(pending === 'timer' ? [] : [['peek']]);
      await fireEvent.pointerMove(main);
      await vi.advanceTimersByTimeAsync(100);
      expect(view.config.setIslandView).toHaveBeenLastCalledWith('peek');
      await fireEvent.pointerLeave(main);
      await vi.advanceTimersByTimeAsync(180);
      expect(view.config.setIslandView).toHaveBeenLastCalledWith('compact');
      native.handlers.get('island:pointer-enter')?.();
      await vi.advanceTimersByTimeAsync(100);
      expect(main.dataset.view).toBe('peek');
      native.handlers.get('island:pointer-leave')?.();
      await vi.advanceTimersByTimeAsync(180);
      expect(main.dataset.view).toBe('compact');
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

test('expanded ignores native hover without automatic view, reads, messages or focus changes', async () => {
  const view = mount();
  await expand();
  const reads = view.config.getIslandState.mock.calls.length;
  const messagesRead = view.config.islandMessages.mock.calls.length;
  const active = document.activeElement;
  vi.useFakeTimers();
  try {
    for (let index = 0; index < 3; index += 1) {
      native.handlers.get('island:pointer-enter')?.();
      native.handlers.get('island:pointer-leave')?.();
    }
    await vi.advanceTimersByTimeAsync(500);
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('expanded');
    expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['expanded']]);
    expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
    expect(view.config.islandMessages).toHaveBeenCalledTimes(messagesRead);
    expect(view.config.refreshIsland).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(active);
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test.each(['timer', 'resize'] as const)(
  'disabled snapshot cancels pending %s and ignores both hover sources',
  async (pending) => {
    let finish!: () => void;
    const setIslandView = vi.fn().mockResolvedValue(undefined);
    if (pending === 'resize')
      setIslandView.mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            finish = resolve;
          }),
      );
    const view = mount({ setIslandView });
    await screen.findByText('1 pending');
    const main = screen.getByRole('main', { name: 'Sessions' });
    vi.useFakeTimers();
    try {
      view.setState({ ...state(), enabled: false });
      native.handlers.get('island:changed')?.();
      if (pending === 'timer') await vi.advanceTimersByTimeAsync(150);
      native.handlers.get('island:pointer-enter')?.();
      await fireEvent.pointerEnter(main);
      await vi.advanceTimersByTimeAsync(pending === 'timer' ? 50 : 200);
      if (pending === 'resize') finish();
      await vi.advanceTimersByTimeAsync(0);
      await tick();
      native.handlers.get('island:pointer-enter')?.();
      native.handlers.get('island:pointer-leave')?.();
      await fireEvent.pointerMove(main);
      await vi.advanceTimersByTimeAsync(300);
      expect(main.dataset.view).toBe('compact');
      expect(view.config.setIslandView.mock.calls).toEqual(pending === 'timer' ? [] : [['peek']]);
      view.setState(state());
      native.handlers.get('island:changed')?.();
      await vi.advanceTimersByTimeAsync(200);
      await vi.advanceTimersByTimeAsync(300);
      expect(main.dataset.view).toBe('compact');
      native.handlers.get('island:pointer-enter')?.();
      await vi.advanceTimersByTimeAsync(100);
      expect(main.dataset.view).toBe('peek');
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

test.each(['timer', 'resize'] as const)('teardown ignores retained native callbacks and late %s', async (pending) => {
  let finish!: () => void;
  const view = mount({
    setIslandView: vi.fn().mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    ),
  });
  await screen.findByText('1 pending');
  const enter = native.handlers.get('island:pointer-enter');
  const leave = native.handlers.get('island:pointer-leave');
  const reads = view.config.getIslandState.mock.calls.length;
  vi.useFakeTimers();
  try {
    enter?.();
    await vi.advanceTimersByTimeAsync(pending === 'timer' ? 99 : 100);
    view.unmount();
    enter?.();
    leave?.();
    if (pending === 'resize') finish();
    await vi.advanceTimersByTimeAsync(500);
    expect(view.config.setIslandView).toHaveBeenCalledTimes(pending === 'timer' ? 0 : 1);
    expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
    expect(view.config.refreshIsland).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    expect(native.unlisten).toHaveBeenCalledTimes(5);
  } finally {
    vi.useRealTimers();
  }
});

test('pointer registrations finish before the initial snapshot read and pre-read native entry cannot revive a disabled island', async () => {
  const finish: (() => void)[] = [];
  native.listen.mockImplementation((name: string, handler: () => void) => {
    native.handlers.set(name, handler);
    return new Promise<() => void>((resolve) => {
      finish.push(() => resolve(() => native.unlisten(name)));
    });
  });
  const view = mount({}, { ...state(), enabled: false });
  await tick();
  expect(native.listen).toHaveBeenCalledTimes(5);
  expect(view.config.getIslandState).not.toHaveBeenCalled();
  vi.useFakeTimers();
  try {
    native.handlers.get('island:pointer-enter')?.();
    for (const register of finish.slice(0, -1)) register();
    await vi.advanceTimersByTimeAsync(0);
    expect(view.config.getIslandState).not.toHaveBeenCalled();
    finish.at(-1)?.();
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    expect(view.config.getIslandState).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(300);
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

test.each([
  ['en', '1 running · Sessions 4', 'Approve 1 · Answer 0', '1 running', '1 pending'],
  ['zh', '1 运行中 · 4 会话', '1 权限 · 0 问题', '1 个运行中', '1 项待处理'],
])(
  'no-notch compact fits factual bilingual status without sensitive details (%s)',
  async (locale, status, pending, fullStatus, fullPending) => {
    const active = document.activeElement;
    const view = mount({ preferences: { theme: 'dark', locale } }, { ...state(), notchWidth: 0, topInset: 0 });
    await waitFor(() => expect(view.container.querySelector('.compact-summary')?.textContent).toContain(status));
    const main = view.container.querySelector('main') as HTMLElement;
    expect(main.dataset.view).toBe('compact');
    expect(main.style.getPropertyValue('--compact-width')).toBe('200px');
    expect(main.style.getPropertyValue('--compact-height')).toBe('28px');
    expect(main.querySelector('.compact-secondary')?.textContent).toBe(pending);
    expect(main.querySelector('#compact-status-hint')?.textContent).toBe(fullStatus);
    expect(main.querySelector('#compact-pending-hint')?.textContent).toBe(fullPending);
    expect(main.querySelector('.compact-summary')?.getAttribute('aria-hidden')).toBe('true');
    expect(main.querySelectorAll('.activity')).toHaveLength(1);
    expect(main.querySelectorAll('.activity i')).toHaveLength(4);
    expect(main.querySelectorAll('svg, .quick-area, .compact-permission, .session-disclosure')).toHaveLength(0);
    expect(main.textContent).not.toMatch(/MOM|work|\/code|git status|repository/);
    expect(document.activeElement).toBe(active);
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  },
);

test.each([
  ['en', '99+ running · Sessions 99+', 'Approve 99+ · Answer 0', '105 running', '105 pending'],
  ['zh', '99+ 运行中 · 99+ 会话', '99+ 权限 · 0 问题', '105 个运行中', '105 项待处理'],
])(
  'no-notch large counts cap only visible text and retain exact accessible counts (%s)',
  async (locale, status, pending, fullStatus, fullPending) => {
    const initial = state();
    initial.notchWidth = 0;
    initial.topInset = 0;
    initial.sessions = Array.from({ length: 105 }, (_, index) => ({ ...initial.sessions[1], id: `run_${index}` }));
    initial.permissions = Array.from({ length: 105 }, (_, index) => ({
      ...initial.permissions[0],
      id: `perm_${index}`,
    }));
    const view = mount({ preferences: { theme: 'dark', locale } }, initial);
    await waitFor(() => expect(view.container.querySelector('.compact-summary')?.textContent).toContain(status));
    expect(view.container.querySelector('.compact-secondary')?.textContent).toBe(pending);
    expect(view.container.querySelector('#compact-status-hint')?.textContent).toBe(fullStatus);
    expect(view.container.querySelector('#compact-pending-hint')?.textContent).toBe(fullPending);
    expect(view.container.querySelectorAll('.activity')).toHaveLength(1);
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  },
);

test.each([
  ['en', 'Idle', 'Connecting', 'Unavailable', 'Disabled'],
  ['zh', '空闲', '连接中', '不可用', '已停用'],
])(
  'no-notch idle and unavailable states never imply running or expose raw errors (%s)',
  async (locale, idle, connecting, unavailable, disabled) => {
    const initial = { ...state(), notchWidth: 0, topInset: 0 };
    const view = mount({ preferences: { theme: 'dark', locale } }, initial);
    await waitFor(() => expect(view.container.querySelector('.activity')?.getAttribute('data-running')).toBe('true'));
    const summary = () => view.container.querySelector('.compact-summary > span')?.textContent;
    for (const [next, label] of [
      [{ ...initial, sessions: initial.sessions.map((session) => ({ ...session, status: 'idle' as const })) }, idle],
      [{ ...initial, connection: 'connecting' as const }, connecting],
      [{ ...initial, connection: 'error' as const, error: 'Private /code/project diagnostic' }, unavailable],
      [{ ...initial, enabled: false }, disabled],
    ] as const) {
      view.setState(next);
      native.handlers.get('island:changed')?.();
      await waitFor(() =>
        expect(summary()).toBe(label === idle ? `${idle} · ${locale === 'en' ? 'Sessions 4' : '4 会话'}` : label),
      );
      expect(view.container.querySelector('.activity')?.getAttribute('data-running')).toBe('false');
      expect(view.container.querySelector('main')?.textContent).not.toContain('Private');
      expect(view.container.querySelector('.compact-status-control')?.getAttribute('title')).not.toContain('/code');
    }
    view.setState(initial);
    native.handlers.get('island:changed')?.();
    await waitFor(() => expect(view.container.querySelector('.activity')?.getAttribute('data-running')).toBe('true'));
    view.config.getIslandState.mockRejectedValueOnce(new Error('Private /code/read failure'));
    native.handlers.get('island:changed')?.();
    await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(7));
    await tick();
    expect(summary()).toBe(unavailable);
    expect(view.container.querySelector('.activity')?.getAttribute('data-running')).toBe('false');
    expect(view.container.querySelector('.compact-secondary')?.textContent).toBe(
      locale === 'en' ? 'Sessions — · Requests —' : '会话 — · 请求 —',
    );
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  },
);

test.each(['dom', 'native'] as const)(
  'no-notch %s hover preserves the three-form lifecycle without automatic focus or chat',
  async (source) => {
    const view = mount({}, { ...state(), notchWidth: 0, topInset: 0 });
    await screen.findByText('1 pending');
    const main = screen.getByRole('main', { name: 'Sessions' });
    const active = document.activeElement;
    vi.useFakeTimers();
    try {
      if (source === 'dom') await fireEvent.pointerEnter(main);
      else native.handlers.get('island:pointer-enter')?.();
      await vi.advanceTimersByTimeAsync(99);
      expect(view.config.setIslandView).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(main.dataset.view).toBe('peek');
      expect(main.style.getPropertyValue('--compact-width')).toBe('400px');
      await vi.advanceTimersByTimeAsync(500);
      expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
      expect(document.activeElement).toBe(active);
      expect(view.config.getIslandState).toHaveBeenCalledOnce();
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      if (source === 'dom') await fireEvent.pointerLeave(main);
      else native.handlers.get('island:pointer-leave')?.();
      await vi.advanceTimersByTimeAsync(180);
      await tick();
      expect(main.dataset.view).toBe('compact');
      await fireEvent.click(screen.getByRole('button', { name: 'Show quick area' }));
      await tick();
      const disclosure = screen.getByRole('button', { name: 'Expand sessions' });
      await fireEvent.pointerEnter(disclosure);
      await vi.advanceTimersByTimeAsync(150);
      expect(view.config.islandMessages).not.toHaveBeenCalled();
      await fireEvent.click(disclosure);
      await tick();
      expect(main.dataset.view).toBe('expanded');
      expect(view.config.islandMessages).toHaveBeenCalledExactlyOnceWith('pending', 0);
      native.handlers.get('island:collapsed')?.();
      await tick();
      expect(main.dataset.view).toBe('compact');
      expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['compact'], ['peek'], ['expanded']]);
      expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
      expect(view.config.islandSend).not.toHaveBeenCalled();
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

test.each(['click', 'touch', 'Enter', ' '] as const)(
  'no-notch %s activation opens only Peek, never sessions directly',
  async (activation) => {
    const view = mount({}, { ...state(), notchWidth: 0, topInset: 0 });
    await screen.findByText('1 pending');
    const status = screen.getByRole('button', { name: 'Session status · Show quick area' });
    if (activation === 'touch') await fireEvent.pointerUp(screen.getByRole('main'), { pointerType: 'touch' });
    else {
      if (activation !== 'click') {
        status.focus();
        await fireEvent.keyDown(status, { key: activation });
        await fireEvent.keyUp(status, { key: activation });
      }
      await fireEvent.click(status, { detail: activation === 'click' ? 1 : 0 });
    }
    await screen.findByRole('button', { name: 'Expand sessions' });
    expect(view.config.setIslandView).toHaveBeenCalledExactlyOnceWith('peek');
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  },
);

test.each(['compact', 'peek', 'expanded'] as const)(
  'current screen notch geometry changes density, never the %s view or resize IPC',
  async (mode) => {
    const initial = { ...state(), notchWidth: 0, topInset: 0 };
    const view = mount({}, initial);
    await screen.findByText('1 pending');
    if (mode === 'peek') await peek();
    else if (mode === 'expanded') await expand();
    const calls = view.config.setIslandView.mock.calls.length;
    const main = screen.getByRole('main', { name: 'Sessions' });
    for (const [notchWidth, topInset] of [
      [180, 32],
      [0, 0],
      [0, 38],
      [180, 0],
    ]) {
      const reads = view.config.getIslandState.mock.calls.length;
      const messagesRead = view.config.islandMessages.mock.calls.length;
      view.setState({ ...initial, notchWidth, topInset });
      native.handlers.get('island:changed')?.();
      await waitFor(() => expect(view.config.getIslandState).toHaveBeenCalledTimes(reads + 1));
      await tick();
      expect(main.dataset.view).toBe(mode);
      expect(main.classList.contains('notched-compact')).toBe(mode === 'compact' && notchWidth > 0 && topInset > 0);
      expect(!!main.querySelector('.compact-summary')).toBe(mode === 'compact' && !(notchWidth > 0 && topInset > 0));
      expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
      if (mode !== 'expanded') expect(view.config.islandMessages).toHaveBeenCalledTimes(messagesRead);
      expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    }
    native.handlers.get('island:collapsed')?.();
    await tick();
    expect(main.dataset.view).toBe('compact');
    expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
  },
);

test.each(['pending', 'failed', 'invalid'] as const)(
  'unknown %s geometry stays hidden and safe until known, without auto-Peek',
  async (kind) => {
    let finish!: (value: IslandState) => void;
    const getIslandState =
      kind === 'pending'
        ? vi.fn().mockImplementationOnce(
            () =>
              new Promise<IslandState>((resolve) => {
                finish = resolve;
              }),
          )
        : kind === 'failed'
          ? vi.fn().mockRejectedValueOnce(new Error('Snapshot unavailable'))
          : vi.fn().mockResolvedValueOnce({ ...state(), notchWidth: Number.NaN, topInset: -1 });
    const view = mount({ getIslandState });
    await waitFor(() => expect(getIslandState).toHaveBeenCalledOnce());
    const main = screen.getByRole('main', { name: 'Sessions' });
    await tick();
    expect(main.classList.contains('geometry-pending')).toBe(true);
    expect(main.style.getPropertyValue('--top-inset')).toBe('0px');
    expect(main.style.getPropertyValue('--notch-width')).toBe('0px');
    native.handlers.get('island:pointer-enter')?.();
    await fireEvent.pointerMove(main);
    await new Promise((resolve) => setTimeout(resolve, 130));
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    const known = { ...state(), notchWidth: 0, topInset: 0 };
    if (kind === 'pending') finish(known);
    else {
      getIslandState.mockResolvedValueOnce(known);
      native.handlers.get('island:changed')?.();
    }
    await waitFor(() => expect(main.classList.contains('geometry-pending')).toBe(false));
    expect(main.dataset.view).toBe('compact');
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  },
);

test.each([
  ['en', 'Running', 'Idle', 'Connecting', 'Offline', 'Off', 'Pending'],
  ['zh', '运行中', '空闲', '连接中', '不可用', '已关闭', '待处理'],
])(
  'both notched wings remain meaningful in every state (%s)',
  async (locale, runningLabel, idle, connecting, offline, off, pendingLabel) => {
    const initial = { ...state(), notchWidth: 209, topInset: 38 };
    const view = mount({ preferences: { theme: 'dark', locale } }, initial);
    await waitFor(() => expect(view.container.querySelector('.wing-label')?.textContent).toBe(runningLabel));
    const main = view.container.querySelector('main') as HTMLElement;
    expect(main.style.getPropertyValue('--compact-width')).toBe('321px');
    expect(main.style.getPropertyValue('--compact-height')).toBe('38px');
    expect(main.querySelector('.wing-count')?.textContent).toBe('1');
    expect(main.querySelector('.wing-pending-count')?.textContent).toBe('1');
    expect(main.querySelector('.compact-status > .wing-label')?.textContent).toBe(pendingLabel);
    const gap = main.querySelector('.notch-gap') as HTMLElement;
    expect(gap.children).toHaveLength(0);
    expect(gap.textContent).toBe('');
    expect(main.querySelectorAll('svg')).toHaveLength(0);
    for (const [next, label, count, requests] of [
      [
        {
          ...initial,
          permissions: [],
          sessions: initial.sessions.map((session) => ({ ...session, status: 'idle' as const })),
        },
        idle,
        '0',
        '0',
      ],
      [{ ...initial, connection: 'connecting' as const }, connecting, '—', '—'],
      [{ ...initial, connection: 'error' as const }, offline, '—', '—'],
      [{ ...initial, enabled: false }, off, '—', '—'],
    ] as const) {
      view.setState(next);
      native.handlers.get('island:changed')?.();
      await waitFor(() => expect(main.querySelector('.compact-wing-state .wing-label')?.textContent).toBe(label));
      expect(main.querySelector('.wing-count')?.textContent).toBe(count);
      expect(main.querySelector('.wing-pending-count')?.textContent).toBe(requests);
      expect(main.querySelector('.compact-status > .wing-label')?.textContent).toBe(pendingLabel);
      expect(main.querySelector('.activity')?.getAttribute('data-running')).toBe('false');
      expect(main.dataset.view).toBe('compact');
    }
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  },
);

test.each(['en', 'zh'] as const)(
  'compact counts distinguish permissions and questions, include cached children, and preserve exact large totals (%s)',
  async (locale) => {
    const initial = { ...state(), notchWidth: 209, topInset: 38 };
    initial.sessions = Array.from({ length: 105 }, (_, index) => ({ ...initial.sessions[1], id: `run_${index}` }));
    initial.permissions = Array.from({ length: 104 }, (_, index) => ({
      ...initial.permissions[0],
      id: `perm_${index}`,
    }));
    initial.questions = [{ id: 'q_1', sessionId: 'child', title: 'Private question', answerable: true, fields: [] }];
    const view = mount({ preferences: { theme: 'dark', locale } }, initial);
    await waitFor(() => expect(view.container.querySelector('.wing-count')?.textContent).toBe('99+'));
    const main = view.container.querySelector('main') as HTMLElement;
    expect(main.querySelector('.wing-pending-count')?.textContent).toBe('99+');
    expect(main.querySelector('#compact-counts-hint')?.textContent).toBe(
      locale === 'en'
        ? 'Current snapshot: 105 sessions; 104 permission requests; 1 questions.'
        : '当前快照：105 个会话；104 项权限请求；1 个问题。',
    );
    const left = main.querySelector('.compact-left') as HTMLButtonElement;
    expect(left.getAttribute('aria-describedby')).toContain('compact-counts-hint');
    view.setState({ ...state(), notchWidth: 0, topInset: 0, questions: initial.questions });
    native.handlers.get('island:changed')?.();
    await waitFor(() =>
      expect(main.querySelector('.compact-primary')?.textContent).toBe(
        locale === 'en' ? '1 running · Sessions 4' : '1 运行中 · 4 会话',
      ),
    );
    expect(main.querySelector('.compact-secondary')?.textContent).toBe(
      locale === 'en' ? 'Approve 1 · Answer 1' : '1 权限 · 1 问题',
    );
    expect(main.querySelector('#compact-counts-hint')?.textContent).not.toMatch(/total|history|总计|历史/i);
    expect(main.textContent).not.toContain('Private question');
    expect(view.config.setIslandView).not.toHaveBeenCalled();
    expect(view.config.islandMessages).not.toHaveBeenCalled();
  },
);

test('presentation preserves the focused peek header, swaps permission controls without overlap, and removes expanded content on collapse', async () => {
  const initial = state();
  initial.permissions.push({ id: 'perm_child', sessionId: 'child', action: 'edit', resources: ['/child/motion.ts'] });
  const view = mount({}, initial);
  await screen.findByText('2 pending');
  const header = screen.getByRole('button', { name: 'Show quick area' });
  const status = screen.getByRole('button', { name: 'Session status · Show quick area' });
  const activity = view.container.querySelector('.activity');
  header.focus();
  await screen.findByRole('button', { name: 'Allow once' });
  expect(screen.getByRole('button', { name: 'Show quick area' })).toBe(header);
  expect(screen.getByRole('button', { name: 'Session status · Show quick area' })).toBe(status);
  expect(view.container.querySelector('.activity')).toBe(activity);
  expect(document.activeElement).toBe(header);
  const previous = screen.getByRole('button', { name: 'Allow once' });
  await fireEvent.click(screen.getByRole('button', { name: 'Next permission request' }));
  expect(previous.isConnected).toBe(false);
  expect(screen.getAllByRole('button', { name: 'Allow once', hidden: true })).toHaveLength(1);
  expect(screen.queryByText('git status')).toBeNull();
  expect(screen.getByText('/child/motion.ts')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  await screen.findByRole('textbox', { name: 'Message' });
  expect(screen.getAllByRole('button', { name: 'Allow once', hidden: true })).toHaveLength(1);
  const selectedSession = screen.getByRole('button', { name: /Pending work/ });
  await fireEvent.click(selectedSession);
  await fireEvent.input(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Local motion draft' } });
  const composer = screen.getByRole('textbox', { name: 'Message' });
  const previousConversation = screen.getByRole('region', { name: 'Recent messages and pending requests' });
  await fireEvent.click(screen.getByRole('button', { name: /Running work/ }));
  expect(previousConversation.isConnected).toBe(false);
  expect(screen.getByRole('textbox', { name: 'Message' })).toBe(composer);
  expect(screen.getAllByRole('region', { name: 'Recent messages and pending requests', hidden: true })).toHaveLength(1);
  await fireEvent.click(screen.getByRole('button', { name: /Pending work/ }));
  expect((composer as HTMLTextAreaElement).value).toBe('Local motion draft');
  composer.focus();
  native.handlers.get('island:collapsed')?.();
  await tick();
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  expect(view.container.querySelector('#island-session-window')).toBeNull();
  expect(composer.isConnected).toBe(false);
  expect(document.activeElement).not.toBe(composer);
  expect(screen.queryByRole('button', { name: 'Allow once', hidden: true })).toBeNull();
  expect(view.config.islandSend).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('rapid expansion/collapse waits for IPC success, rejects stale message readback, and never revives removed content', async () => {
  let finishExpand!: () => void;
  let finishCollapse!: () => void;
  let finishMessages!: (value: IslandMessage[]) => void;
  const view = mount();
  await peek();
  view.config.setIslandView.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finishExpand = resolve;
      }),
  );
  view.config.islandMessages.mockImplementationOnce(
    () =>
      new Promise<IslandMessage[]>((resolve) => {
        finishMessages = resolve;
      }),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('peek');
  expect(screen.queryByRole('textbox', { name: 'Message', hidden: true })).toBeNull();
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  finishExpand();
  await screen.findByRole('textbox', { name: 'Message' });
  await waitFor(() => expect(view.config.islandMessages).toHaveBeenCalledOnce());
  view.config.setIslandView.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finishCollapse = resolve;
      }),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Collapse island' }));
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('expanded');
  native.handlers.get('island:collapsed')?.();
  await tick();
  expect(view.container.querySelector('#island-session-window')).toBeNull();
  finishMessages([{ id: 'late_motion', role: 'assistant', text: 'Late motion message', time: 1, truncated: false }]);
  finishCollapse();
  await tick();
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  expect(screen.queryByText('Late motion message')).toBeNull();
  expect(screen.queryByRole('textbox', { name: 'Message', hidden: true })).toBeNull();
  expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['expanded'], ['compact']]);
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
});

test('failed grow retains peek; early compact collapse preserves shrink error and drafts without restoring controls', async () => {
  const view = mount();
  await peek();
  const header = screen.getByRole('button', { name: 'Show quick area' });
  view.config.setIslandView.mockRejectedValueOnce(new Error('Expansion failed'));
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  await screen.findByRole('alert');
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('peek');
  expect(screen.getByRole('button', { name: 'Show quick area' })).toBe(header);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(screen.queryByRole('textbox', { name: 'Message', hidden: true })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  await screen.findByRole('textbox', { name: 'Message' });
  const composer = screen.getByRole('textbox', { name: 'Message' });
  await fireEvent.input(composer, { target: { value: 'Keep after failed shrink' } });
  let fail!: (cause: Error) => void;
  view.config.setIslandView.mockImplementationOnce(() => {
    native.handlers.get('island:collapsed')?.();
    return new Promise<void>((_resolve, reject) => {
      fail = reject;
    });
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Collapse island' }));
  const calls = view.config.setIslandView.mock.calls.length;
  expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
  expect(composer.isConnected).toBe(false);
  expect(screen.queryByRole('button', { name: 'Allow once', hidden: true })).toBeNull();
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
  fail(new Error('Shrink failed'));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Session status · Show quick area' }).title).toBe('Shrink failed'),
  );
  expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
  expect(view.config.islandSend).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Show quick area' }));
  await screen.findByRole('button', { name: 'Expand sessions' });
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  const restored = await screen.findByRole('textbox', { name: 'Message' });
  expect((restored as HTMLTextAreaElement).value).toBe('Keep after failed shrink');
});

test.each(['peek', 'expanded'] as const)(
  'compact supersedes pending %s immediately, deduplicates Escape and ignores late grow success/failure',
  async (growing) => {
    const view = mount();
    if (growing === 'expanded') await peek();
    else await screen.findByText('1 pending');
    const header = screen.getByRole('button', { name: 'Show quick area' });
    let finishGrow!: () => void;
    let failGrow!: (cause: Error) => void;
    let finishCompact!: () => void;
    view.config.setIslandView
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve, reject) => {
            finishGrow = resolve;
            failGrow = reject;
          }),
      )
      .mockImplementationOnce(() => {
        native.handlers.get('island:collapsed')?.();
        return new Promise<void>((resolve) => {
          finishCompact = resolve;
        });
      });
    await fireEvent.click(growing === 'peek' ? header : screen.getByRole('button', { name: 'Expand sessions' }));
    await fireEvent.keyDown(window, { key: 'Escape' });
    const expected = growing === 'peek' ? [['peek'], ['compact']] : [['peek'], ['expanded'], ['compact']];
    expect(view.config.setIslandView.mock.calls).toEqual(expected);
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
    expect(screen.queryByRole('button', { name: 'Allow once', hidden: true })).toBeNull();
    expect(screen.queryByRole('textbox', { name: 'Message', hidden: true })).toBeNull();
    expect(screen.getByRole('button', { name: 'Show quick area' })).toBe(header);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(view.config.setIslandView.mock.calls).toEqual(expected);
    if (growing === 'peek') finishGrow();
    else failGrow(new Error('Superseded grow'));
    await tick();
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
    expect(screen.getByRole('button', { name: 'Session status · Show quick area' }).title).not.toBe('Superseded grow');
    finishCompact();
    await tick();
    expect(view.config.setIslandView.mock.calls).toEqual(expected);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    expect(view.config.islandSend).not.toHaveBeenCalled();
  },
);

test.each(['new request', 'external collapse', 'unmount'] as const)(
  'early compact error is ignored after %s supersedes its request identity',
  async (end) => {
    const view = mount();
    await peek();
    let fail!: (cause: Error) => void;
    view.config.setIslandView.mockImplementationOnce(() => {
      native.handlers.get('island:collapsed')?.();
      return new Promise<void>((_resolve, reject) => {
        fail = reject;
      });
    });
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe('compact');
    if (end === 'new request') {
      await fireEvent.click(screen.getByRole('button', { name: 'Show quick area' }));
      await screen.findByRole('button', { name: 'Allow once' });
    } else if (end === 'external collapse') native.handlers.get('island:collapsed')?.();
    else view.unmount();
    const calls = view.config.setIslandView.mock.calls.length;
    fail(new Error('Stale shrink error'));
    await tick();
    expect(screen.queryByText('Stale shrink error')).toBeNull();
    if (end !== 'unmount') {
      expect(screen.getByRole('button', { name: 'Session status · Show quick area' }).title).not.toBe(
        'Stale shrink error',
      );
      expect(view.container.querySelector('main')?.getAttribute('data-view')).toBe(
        end === 'new request' ? 'peek' : 'compact',
      );
    } else expect(view.container.querySelector('main')).toBeNull();
    expect(view.config.setIslandView).toHaveBeenCalledTimes(calls);
    expect(view.config.islandMessages).not.toHaveBeenCalled();
    expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
    expect(view.config.islandSend).not.toHaveBeenCalled();
  },
);

test('unmount during explicit expansion ignores the late result and leaves no presentation or messages behind', async () => {
  let finish!: () => void;
  const view = mount();
  await peek();
  view.config.setIslandView.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Expand sessions' }));
  const reads = view.config.getIslandState.mock.calls.length;
  view.unmount();
  finish();
  await tick();
  expect(view.container.querySelector('main')).toBeNull();
  expect(view.config.setIslandView.mock.calls).toEqual([['peek'], ['expanded']]);
  expect(view.config.getIslandState).toHaveBeenCalledTimes(reads);
  expect(view.config.islandMessages).not.toHaveBeenCalled();
  expect(view.config.islandReplyPermission).not.toHaveBeenCalled();
  expect(native.unlisten).toHaveBeenCalledTimes(5);
});

test('CSS reduced-motion overrides every rendered motion target while the native-sized backdrop never fades or moves', async () => {
  const style = document.createElement('style');
  style.textContent = islandSource.split('<style>')[1].split('</style>')[0];
  document.head.append(style);
  const rules = Array.from(style.sheet?.cssRules ?? []);
  style.remove();
  const normal = rules.filter((rule) => rule.type === CSSRule.STYLE_RULE) as CSSStyleRule[];
  const reduced = rules.find(
    (rule) =>
      rule.type === CSSRule.MEDIA_RULE && (rule as CSSMediaRule).conditionText === '(prefers-reduced-motion: reduce)',
  ) as CSSMediaRule;
  expect(reduced).toBeTruthy();
  const overrides = Array.from(reduced.cssRules) as CSSStyleRule[];
  const matches = (node: Element, rule: CSSStyleRule) =>
    node.matches(rule.selectorText.replace(/:global\(([^)]+)\)/g, '$1'));
  const view = mount();
  await screen.findByText('1 pending');
  for (const stage of ['compact', 'peek', 'expanded']) {
    if (stage === 'peek') await peek();
    else if (stage === 'expanded') await expand();
    const main = screen.getByRole('main');
    const backdrop = normal.filter((rule) => matches(main, rule));
    for (const property of ['animation', 'opacity', 'transform'])
      expect(backdrop.every((rule) => !rule.style.getPropertyValue(property))).toBe(true);
    expect(backdrop.find((rule) => rule.style.getPropertyValue('width'))?.style.getPropertyValue('width')).toBe(
      '100vw',
    );
    expect(backdrop.find((rule) => rule.style.getPropertyValue('height'))?.style.getPropertyValue('height')).toBe(
      '100dvh',
    );
    for (const node of [main, ...main.querySelectorAll('*')]) {
      for (const property of ['animation', 'transition']) {
        if (!normal.some((rule) => matches(node, rule) && rule.style.getPropertyValue(property))) continue;
        const override = overrides
          .filter((rule) => matches(node, rule) && rule.style.getPropertyValue(property))
          .at(-1);
        expect(override?.style.getPropertyValue(property)).toBe('none');
      }
    }
  }
  view.unmount();
});
