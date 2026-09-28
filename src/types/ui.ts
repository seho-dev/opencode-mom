import type { PermissionRule } from './agents.js';

export type PermissionParse = { ok: true; rules: PermissionRule[] } | { ok: false };
export type PermissionView = { mode: 'rules'; rules: PermissionRule[] } | { mode: 'json' } | { mode: 'invalid' };
