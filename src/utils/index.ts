import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import catalog from '../../shared/agents.catalog.json';
import type { BuiltinSystem } from './constants.js';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export type WithoutChild<T> = T extends { child?: unknown } ? Omit<T, 'child'> : T;
export type WithoutChildren<T> = T extends { children?: unknown } ? Omit<T, 'children'> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };

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

export function isValidTokenCount(value: string): boolean {
  return /^\d+$/.test(value.trim()) && Number.isSafeInteger(Number(value));
}

// Minimal shape accepted from both the CLI catalog and the provider store; only these fields are read.
export type ModelOptionEntry = { ref: string; name?: string; variants?: readonly { id: string }[] | null };

// Catalog covers builtin + custom; `models` adds custom-only refs. Catalog label wins.
export function buildModelOptions(
  catalog: ModelOptionEntry[],
  models: ModelOptionEntry[],
  current: string,
  emptyLabel: string,
): { value: string; label: string }[] {
  const labels = new Map<string, string>();
  for (const entry of catalog) labels.set(entry.ref, entry.name ?? entry.ref);
  for (const entry of models) if (!labels.has(entry.ref)) labels.set(entry.ref, entry.name ?? entry.ref);
  const known = [...labels.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([value, label]) => ({ value, label }));
  return [
    { value: '', label: emptyLabel },
    ...known,
    ...(current && !labels.has(current) ? [{ value: current, label: `${current} (unavailable)` }] : []),
  ];
}

export function buildModelVariants(catalog: ModelOptionEntry[], models: ModelOptionEntry[], current: string): string[] {
  const entry = catalog.find((entry) => entry.ref === current) ?? models.find((entry) => entry.ref === current);
  return entry?.variants?.map((variant) => variant.id) ?? [];
}
