export type ModelRef = `${string}/${string}`;
export type GroupType = 'opencode' | 'slim' | 'oh-my-openagent';
export type AgentSource = 'inline' | 'markdown' | 'both';
export type AgentStorage = 'inline' | 'global_markdown' | 'project_markdown';

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
export interface ModelDef {
  id: string;
  name?: string;
  family?: string;
  release_date?: string;
  status?: 'alpha' | 'beta' | 'deprecated' | 'active';
  reasoning?: boolean;
  temperature?: number;
  tool_call?: boolean;
  attachment?: boolean;
  interleaved?: boolean;
  cost?: { input?: number; output?: number; cache_read?: number; cache_write?: number; context_over_200k?: number };
  limit?: { context?: number; input?: number; output?: number };
  modalities?: { input?: string[]; output?: string[] };
  experimental?: boolean;
  options?: Record<string, unknown>;
  headers?: Record<string, string>;
  variants?: Record<string, { disabled?: boolean; options?: Record<string, unknown> }>;
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
export interface AppState {
  providers: ProviderDef[];
  agents: AgentDefinition[];
  groups: Group[];
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
