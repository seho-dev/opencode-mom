import type { AppPreferences } from './app.js';

export type IslandView = 'compact' | 'peek' | 'expanded';

export interface IslandState {
  supported: boolean;
  enabled: boolean;
  connection: 'disabled' | 'connecting' | 'connected' | 'error';
  connectionGeneration: number;
  error?: string;
  sessions: IslandSession[];
  permissions: IslandPermission[];
  questions: IslandQuestion[];
  preferences: AppPreferences;
  notchWidth: number;
  topInset: number;
}

export interface IslandSession {
  id: string;
  title: string;
  directory: string;
  updatedAt: number;
  parentId?: string;
  status: 'running' | 'idle' | 'error';
}

export interface IslandPermission {
  id: string;
  sessionId: string;
  action: string;
  resources: string[];
  save?: string[];
  message?: string;
}

export type IslandPermissionDecision = 'once' | 'always' | 'reject';

export interface IslandQuestion {
  id: string;
  sessionId: string;
  title: string;
  answerable: boolean;
  fields: IslandQuestionField[];
}

export interface IslandQuestionField {
  key: string;
  type: 'string' | 'multiselect';
  title?: string;
  description?: string;
  options: { value: string; label: string; description?: string }[];
  custom: boolean;
  required: boolean;
}

export interface IslandMessage {
  id: string;
  role: 'user' | 'assistant';
  text: string;
  time: number;
  truncated: boolean;
}

export type IslandAnswer = Record<string, string | string[]>;
