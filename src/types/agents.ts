import type { AgentMode, AgentSource, AgentStorage } from '$src/utils/constants.js';
import type { ModelRef } from './models.js';

export type { AgentMode, AgentSource, AgentStorage };

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
export type AgentMutation = { fields: Record<string, unknown>; clearFields?: string[] };
export type AgentWrite = AgentDefinition & { mutation?: AgentMutation };
