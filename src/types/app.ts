import type { AgentDefinition } from './agents.js';
import type { Group } from './groups.js';
import type { ProviderDef } from './providers.js';

export type ThemePreference = 'light' | 'dark';
export type LocalePreference = 'en' | 'zh';
export interface AppInfo {
  version: string;
  platform: 'macos' | 'windows' | 'linux' | 'other';
}
export interface UpdateCheck {
  currentVersion: string;
  latestVersion: string | null;
  updateAvailable: boolean;
}
export type ProjectPage = 'repository' | 'releases';
export interface AppPreferences {
  theme: ThemePreference;
  locale: LocalePreference;
}
export interface AppState {
  providers: ProviderDef[];
  agents: AgentDefinition[];
  groups: Group[];
  selectedGroupId?: string | null;
  preferences: AppPreferences;
  diagnostics?: string[];
}
export type CommandErrorCode =
  | 'not_found'
  | 'references_blocked'
  | 'validation_failed'
  | 'configuration_failed'
  | 'busy'
  | 'conflict'
  | 'ipc_error';
export interface CommandError {
  code: CommandErrorCode;
  message: string;
  detail?: unknown;
}
