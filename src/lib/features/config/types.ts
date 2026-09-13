export type ModelRef = `${string}/${string}`;
// Model config mirrors provider.<id>.models.<model_id> in the opencode JSON schema.
// String unions are derived from their constant arrays in constants.ts to avoid duplicate literals.
import type { AgentSource, AgentStorage, GroupType, AgentMode, MappingKind } from './constants.js';
export type { AgentSource, AgentStorage, GroupType, AgentMode, MappingKind };

export interface ProviderOptions {
  apiKey?: string;
  baseURL?: string;
  headers?: Record<string, string>;
}
export interface ProviderDef {
  name: string;
  npm?: string;
  options?: ProviderOptions;
  models: Record<string, ModelDef>;
}
export type ModelModality = 'text' | 'audio' | 'image' | 'video' | 'pdf';
export type ModelStatus = 'alpha' | 'beta' | 'deprecated' | 'active';
export type ModelInterleaved = boolean | 'reasoning' | 'reasoning_content' | 'reasoning_text' | { field: string };
export interface ModelCost {
  input: number;
  output: number;
  cache_read?: number;
  cache_write?: number;
  context_over_200k?: { input: number; output: number; cache_read?: number; cache_write?: number };
}
export interface ModelLimit {
  context: number;
  output: number;
  input?: number;
}
export interface ModelDef {
  id: string;
  name?: string;
  family?: string;
  release_date?: string;
  status?: ModelStatus;
  reasoning?: boolean;
  temperature?: boolean;
  tool_call?: boolean;
  attachment?: boolean;
  experimental?: boolean;
  interleaved?: ModelInterleaved;
  cost?: ModelCost;
  limit?: ModelLimit;
  modalities?: { input?: ModelModality[]; output?: ModelModality[] };
  options?: Record<string, unknown>;
  headers?: Record<string, string>;
  variants?: Record<string, Record<string, unknown>>;
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
  disable?: boolean;
  hidden?: boolean;
  color?: string;
  variant?: string;
  prompt?: string;
  temperature?: number;
  top_p?: number;
  steps?: number;
  permission?: Record<string, unknown>;
  tools?: Record<string, boolean>;
  options?: Record<string, unknown>;
}
// Payload for agent create/update. `clearFields` requests explicit removal of previously set keys.
export type AgentMutation = { fields: Record<string, unknown>; clearFields?: string[] };
export type AgentWrite = AgentDefinition & { mutation?: AgentMutation };

export type OptionRow = { key: string; value: string };
export type PermissionParse = { ok: true; object: Record<string, unknown> } | { ok: false };
export type PermissionView = { mode: 'rows'; values: Record<string, string> } | { mode: 'json' } | { mode: 'invalid' };
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
  status?: string;
  cost?: unknown;
  limit?: { context?: number; input?: number; output?: number } | null;
  capabilities?: unknown;
  variants?: Record<string, unknown> | null;
  api?: unknown;
  releaseDate?: string;
}
export type CommandErrorCode =
  'not_found' | 'references_blocked' | 'validation_failed' | 'configuration_failed' | 'busy' | 'conflict' | 'ipc_error';
export interface CommandError {
  code: CommandErrorCode;
  message: string;
  detail?: unknown;
}
export type Result<T> = Promise<T>;
