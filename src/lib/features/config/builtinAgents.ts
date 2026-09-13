// Static catalog of agents shipped by the slim and oh-my-openagent presets.
// Read-only reference data; never merged with the user's config.agents.
import { GROUP_TYPE_SLIM, GROUP_TYPE_OMO, type BuiltinSystem } from './constants.js';

export type BuiltinAgentType = 'primary' | 'subagent' | 'internal';
export interface BuiltinAgent {
  id: string;
  system: BuiltinSystem;
  type: BuiltinAgentType;
  description: string;
}

export const BUILTIN_AGENTS: BuiltinAgent[] = [
  {
    id: 'orchestrator',
    system: GROUP_TYPE_SLIM,
    type: 'primary',
    description: 'Main orchestrator that dispatches all subagents.',
  },
  {
    id: 'explorer',
    system: GROUP_TYPE_SLIM,
    type: 'subagent',
    description: 'Explores the codebase to locate files and symbols.',
  },
  { id: 'librarian', system: GROUP_TYPE_SLIM, type: 'subagent', description: 'Retrieves research and documentation.' },
  { id: 'oracle', system: GROUP_TYPE_SLIM, type: 'subagent', description: 'Consultant for high-level advice.' },
  { id: 'designer', system: GROUP_TYPE_SLIM, type: 'subagent', description: 'Handles UI and UX design work.' },
  { id: 'fixer', system: GROUP_TYPE_SLIM, type: 'subagent', description: 'Fixes bugs and debugs failing behavior.' },
  { id: 'observer', system: GROUP_TYPE_SLIM, type: 'subagent', description: 'Read-only monitoring of running work.' },
  {
    id: 'council',
    system: GROUP_TYPE_SLIM,
    type: 'subagent',
    description: 'Coordinates multi-agent council sessions.',
  },
  { id: 'councillor', system: GROUP_TYPE_SLIM, type: 'subagent', description: 'Participates as a council member.' },
  {
    id: 'sisyphus',
    system: GROUP_TYPE_OMO,
    type: 'primary',
    description: 'Main orchestrator for the oh-my-openagent system.',
  },
  { id: 'hephaestus', system: GROUP_TYPE_OMO, type: 'primary', description: 'Deep worker reserved for GPT models.' },
  { id: 'atlas', system: GROUP_TYPE_OMO, type: 'primary', description: 'Orchestrates tasks across the system.' },
  {
    id: 'sisyphus-junior',
    system: GROUP_TYPE_OMO,
    type: 'subagent',
    description: 'Lightweight variant of sisyphus for smaller tasks.',
  },
  { id: 'oracle', system: GROUP_TYPE_OMO, type: 'subagent', description: 'Consultant for high-level advice.' },
  {
    id: 'librarian',
    system: GROUP_TYPE_OMO,
    type: 'subagent',
    description: 'Retrieves research and documentation.',
  },
  {
    id: 'explore',
    system: GROUP_TYPE_OMO,
    type: 'subagent',
    description: 'Explores the codebase to locate files and symbols.',
  },
  {
    id: 'multimodal-looker',
    system: GROUP_TYPE_OMO,
    type: 'subagent',
    description: 'Inspects images and other multimodal inputs.',
  },
  { id: 'metis', system: GROUP_TYPE_OMO, type: 'subagent', description: 'Advises on plans before execution.' },
  { id: 'momus', system: GROUP_TYPE_OMO, type: 'subagent', description: 'Critiques plans and surfaces risks.' },
  {
    id: 'prometheus',
    system: GROUP_TYPE_OMO,
    type: 'internal',
    description: 'Builds plans and orchestrates subagents.',
  },
];
