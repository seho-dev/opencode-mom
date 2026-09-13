// Single source of truth for the built-in agent catalog, shared with the Rust backend.
// data lives in agents.catalog.json; this module only derives typed views over it.
import catalog from './agents.catalog.json';
import type { BuiltinSystem } from './constants.js';

export type BuiltinAgentType = 'primary' | 'subagent' | 'internal';
export interface BuiltinAgent {
  id: string;
  system: BuiltinSystem;
  type: BuiltinAgentType;
  description: string;
}

export const BUILTIN_AGENTS: BuiltinAgent[] = catalog.agents.map((agent) => ({
  id: agent.id,
  system: agent.system as BuiltinSystem,
  type: agent.type as BuiltinAgentType,
  description: agent.description,
}));
export const BUILTIN_AGENT_IDS: ReadonlySet<string> = new Set(BUILTIN_AGENTS.map((agent) => agent.id));
export function isBuiltinAgentId(id: string): boolean {
  return BUILTIN_AGENT_IDS.has(id);
}
