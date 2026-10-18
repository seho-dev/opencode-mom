import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('island IPC and lazy store preserve exact arguments without config preload or write retries', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    const state = {
      supported: true,
      enabled: false,
      connection: 'disabled',
      connectionGeneration: 7,
      sessions: [],
      permissions: [{ id: 'perm_3', sessionId: 'ses_child', action: 'edit', resources: ['src/file'], save: ['src/*'] }],
      questions: [],
      preferences: { theme: 'light', locale: 'zh' },
      notchWidth: 180,
      topInset: 32,
    };
    const messages = [{ id: 'msg_1', role: 'user', text: 'Hello', time: 1, truncated: false }];
    let failure;
    const store = createConfigStore(
      createTauriAdapter(async (command, args) => {
        calls.push([command, args]);
        if (failure) throw failure;
        if (['get_island_state', 'set_island_enabled', 'refresh_island'].includes(command)) return state;
        if (command === 'island_messages') return messages;
      }),
      false,
      false,
    );
    await new Promise(setImmediate);
    assert.deepEqual(calls, []);
    assert.equal(store.loading, false);
    assert.equal(store.splashLoading, false);
    store.applyPreferences(state.preferences);
    assert.deepEqual(store.preferences, state.preferences);
    assert.deepEqual(calls, []);
    assert.strictEqual(await store.getIslandState(), state);
    assert.strictEqual(await store.setIslandEnabled(true), state);
    assert.strictEqual(await store.refreshIsland(), state);
    for (const view of ['compact', 'peek', 'expanded']) {
      assert.strictEqual(await store.setIslandView(view), undefined);
    }
    await store.setIslandExpanded(true);
    await store.setIslandExpanded(false);
    assert.strictEqual(await store.islandMessages('ses_1', 7), messages);
    await store.islandSend('ses_1', 'Hello\nworld', 'msg_stable', 7);
    await store.islandReplyPermission('ses_1', 'perm_1', 'once', 7);
    await store.islandReplyPermission('ses_1', 'perm_2', 'reject', 7);
    await store.islandReplyPermission('ses_child', 'perm_3', 'always', 7);
    const answer = { field_key: 'option_value', multiple_key: ['first', 'custom'] };
    await store.islandReplyQuestion('ses_1', 'form_1', answer, 7);
    await store.islandCancelQuestion('ses_1', 'form_2', 7);
    await store.islandInterrupt('ses_1', 7);
    assert.deepEqual(calls, [
      ['get_island_state', {}],
      ['set_island_enabled', { enabled: true }],
      ['refresh_island', {}],
      ['set_island_view', { view: 'compact' }],
      ['set_island_view', { view: 'peek' }],
      ['set_island_view', { view: 'expanded' }],
      ['set_island_expanded', { expanded: true }],
      ['set_island_expanded', { expanded: false }],
      ['island_messages', { sessionId: 'ses_1', expectedConnectionGeneration: 7 }],
      [
        'island_send',
        { sessionId: 'ses_1', text: 'Hello\nworld', messageId: 'msg_stable', expectedConnectionGeneration: 7 },
      ],
      [
        'island_reply_permission',
        { sessionId: 'ses_1', requestId: 'perm_1', decision: 'once', expectedConnectionGeneration: 7 },
      ],
      [
        'island_reply_permission',
        { sessionId: 'ses_1', requestId: 'perm_2', decision: 'reject', expectedConnectionGeneration: 7 },
      ],
      [
        'island_reply_permission',
        { sessionId: 'ses_child', requestId: 'perm_3', decision: 'always', expectedConnectionGeneration: 7 },
      ],
      ['island_reply_question', { sessionId: 'ses_1', formId: 'form_1', answer, expectedConnectionGeneration: 7 }],
      ['island_cancel_question', { sessionId: 'ses_1', formId: 'form_2', expectedConnectionGeneration: 7 }],
      ['island_interrupt', { sessionId: 'ses_1', expectedConnectionGeneration: 7 }],
    ]);
    failure = { code: 'ipc_error', message: 'Reply lost' };
    let beforeSend = calls.length;
    await assert.rejects(store.islandSend('ses_1', 'Hello', 'msg_stable', 7), failure);
    assert.equal(calls.length, beforeSend + 1);
    failure = { code: 'conflict', message: 'Message ID conflict (409)' };
    beforeSend = calls.length;
    await assert.rejects(store.islandSend('ses_1', 'Hello', 'msg_stable', 7), failure);
    assert.equal(calls.length, beforeSend + 1);
    assert.deepEqual(calls.at(-1), [
      'island_send',
      { sessionId: 'ses_1', text: 'Hello', messageId: 'msg_stable', expectedConnectionGeneration: 7 },
    ]);
    for (const view of ['compact', 'peek', 'expanded']) {
      const before = calls.length;
      await assert.rejects(store.setIslandView(view), (error) => {
        assert.strictEqual(error, failure);
        return true;
      });
      assert.equal(calls.length, before + 1);
      assert.deepEqual(calls.at(-1), ['set_island_view', { view }]);
    }
    for (const decision of ['once', 'always']) {
      const before = calls.length;
      await assert.rejects(store.islandReplyPermission('ses_child', 'perm_3', decision, 7), failure);
      assert.equal(calls.length, before + 1);
      assert.deepEqual(calls.at(-1), [
        'island_reply_permission',
        { sessionId: 'ses_child', requestId: 'perm_3', decision, expectedConnectionGeneration: 7 },
      ]);
    }
    assert.equal(store.error, null);
    assert.equal(store.saving, false);
    assert.equal(store.draftRecovery, null);
  } finally {
    await vite.close();
  }
});
