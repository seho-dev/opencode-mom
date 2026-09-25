export type ModelRef = `${string}/${string}`;

// Model config mirrors providers.<id>.models.<model_id> in the opencode V2 JSON schema.
// String unions are derived from their constant arrays in constants.ts to avoid duplicate literals.
import type { AgentMode, AgentSource, AgentStorage, GroupType, MappingKind } from './constants.js';

export type { AgentMode, AgentSource, AgentStorage, GroupType, MappingKind };

export interface ProviderSettings {
  apiKey?: string;
  baseURL?: string;
}
export interface ProviderDef {
  name: string;
  package?: string;
  settings?: ProviderSettings;
  headers?: Record<string, string>;
  models: Record<string, ModelDef>;
}
export type ModelModality = 'text' | 'audio' | 'image' | 'video' | 'pdf';
export interface ModelCapabilities {
  tools?: boolean;
  input?: ModelModality[];
  output?: ModelModality[];
}
export interface ModelCost {
  input: number;
  output: number;
  cache?: { read?: number; write?: number };
}
export interface ModelLimit {
  context: number;
  output: number;
  input?: number;
}
// One V2 variant entry: `{ id }` plus optional `settings` / `headers` / `body` payloads.
export interface ModelVariant {
  id: string;
  settings?: Record<string, unknown>;
  headers?: Record<string, string>;
  body?: unknown;
}
export interface ModelDef {
  id: string;
  name?: string;
  family?: string;
  disabled?: boolean;
  capabilities?: ModelCapabilities;
  cost?: ModelCost;
  limit?: ModelLimit;
  settings?: Record<string, unknown>;
  headers?: Record<string, string>;
  variants?: ModelVariant[];
}
export type PermissionEffect = 'allow' | 'ask' | 'deny';
export interface PermissionRule {
  action: string;
  resource: string;
  effect: PermissionEffect | string;
}
export interface AgentDefinition {
  id: string;
  source: AgentSource;
  storage?: AgentStorage;
  inline?: Record<string, unknown>;
  markdown?: Record<string, unknown>;
  effective?: Record<string, unknown>;
  modelRef?: ModelRef;
  mode?: string;
  description?: string;
  disabled?: boolean;
  hidden?: boolean;
  color?: string;
  prompt?: string;
  steps?: number;
  permissions?: PermissionRule[];
}
// Payload for agent create/update. `clearFields` requests explicit removal of previously set keys.
export type AgentMutation = { fields: Record<string, unknown>; clearFields?: string[] };
export type AgentWrite = AgentDefinition & { mutation?: AgentMutation };

export type PermissionParse = { ok: true; rules: PermissionRule[] } | { ok: false };
export type PermissionView = { mode: 'rules'; rules: PermissionRule[] } | { mode: 'json' } | { mode: 'invalid' };
export interface AgentModelBinding {
  agentName: string;
  modelRef: ModelRef;
  variant?: string;
}
export interface CategoryMapping {
  categoryName: string;
  modelRef: ModelRef;
  variant?: string;
}
export interface Group {
  id: string;
  name: string;
  description: string;
  type: GroupType;
  openCodeAgentOverrides: AgentModelBinding[];
  slimAgentOverrides: AgentModelBinding[] | null;
  omoAgentOverrides: AgentModelBinding[] | null;
  omoCategoryMappings: CategoryMapping[] | null;
  isEnabled: boolean;
  updatedAt: string;
}
export type ThemePreference = 'light' | 'dark';
export type LocalePreference = 'en' | 'zh';
export interface AppPreferences {
  theme: ThemePreference;
  locale: LocalePreference;
}
export interface AppState {
  providers: ProviderDef[];
  agents: AgentDefinition[];
  groups: Group[];
  preferences: AppPreferences;
  diagnostics?: string[];
}
export interface ModelCatalogEntry {
  providerId: string;
  modelId: string;
  ref: ModelRef;
  name: string;
  isCustom: boolean;
  limit?: { context?: number; input?: number; output?: number } | null;
  variants?: ModelVariant[] | null;
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
export type Result<T> = Promise<T>;
