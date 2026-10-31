import type { AgentDefinition } from './agents.js';
import type { Group, GroupType } from './groups.js';
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
export interface OpenCodeInstallation {
  binaryPath: string | null;
  version: string | null;
  method:
    | 'curl'
    | 'npm'
    | 'bun'
    | 'pnpm'
    | 'yarn'
    | 'vite-plus'
    | 'brew'
    | 'manual'
    | 'bundled'
    | 'unknown'
    | 'not-found';
  canUpgrade: boolean;
  upgradeReason:
    | 'not_found'
    | 'unsupported'
    | 'bundled'
    | 'not_v2'
    | 'unstable'
    | 'manager_unavailable'
    | 'ambiguous'
    | null;
  manualCommand: string | null;
}
export interface OpenCodeUpdateCheck {
  installation: OpenCodeInstallation;
  latestVersion: string;
  updateAvailable: boolean;
}
export interface OpenCodeServiceTarget {
  mode: 'shared' | 'explicit';
  url: string | null;
  hasPassword: boolean;
}
export interface OpenCodeServiceStatus {
  target: OpenCodeServiceTarget;
  connected: boolean;
  url: string | null;
  pid: number | null;
  version: string | null;
  error: string | null;
}
export type ProjectPage = 'repository' | 'releases';
export interface AppPreferences {
  theme: ThemePreference;
  locale: LocalePreference;
  defaultPlugin?: GroupType;
}
export interface ProjectFolder {
  id: string;
  path: string;
}
export interface AppState {
  providers: ProviderDef[];
  agents: AgentDefinition[];
  groups: Group[];
  selectedGroupId?: string | null;
  projects?: ProjectFolder[];
  selectedProjectId?: string | null;
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
