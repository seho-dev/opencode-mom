import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import {
  commandErrorMessage
} from './tauriClient';

describe('Tauri command client', () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it('Given command and native errors When normalized Then the most specific message is returned', () => {
    expect(commandErrorMessage({ code: 'writeFailed', message: 'Write failed.', detail: 'disk full' })).toBe('disk full');
    expect(commandErrorMessage(new Error('native failure'))).toBe('native failure');
    expect(commandErrorMessage(null)).toBe('Command failed.');
  });
});
