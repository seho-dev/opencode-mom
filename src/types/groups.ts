import type { GroupType, MappingKind } from '$src/utils/constants.js';
import type { ModelRef } from './models.js';

export type { GroupType, MappingKind };

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
