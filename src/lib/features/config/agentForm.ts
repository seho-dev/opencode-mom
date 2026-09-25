// Pure, framework-free helpers shared by the agent create/edit pages.
import type { PermissionParse, PermissionRule, PermissionView } from './types.js';

// Minimal shape accepted from both the CLI catalog and the provider store; only these fields are read.
export type ModelOptionEntry = { ref: string; name?: string; variants?: readonly { id: string }[] | null };

export type AgentFieldValues = {
  model: string;
  mode: string;
  description: string;
  disabled: boolean;
  hidden: boolean;
  color: string;
  variant: string;
  steps: string;
  prompt: string;
};

export function pretty(value: unknown): string {
  return JSON.stringify(value ?? {}, null, 2);
}

// Splits a V2 `provider/model#variant` selector into its form pieces.
export function splitModelSelector(selector: string): { model: string; variant: string } {
  const hash = selector.lastIndexOf('#');
  if (hash <= 0) return { model: selector, variant: '' };
  return { model: selector.slice(0, hash), variant: selector.slice(hash + 1) };
}

// Joins the form pieces back into one V2 selector; a variant without a model is dropped.
export function joinModelSelector(model: string, variant: string): string {
  if (!model) return '';
  return variant ? `${model}#${variant}` : model;
}

const isRule = (value: unknown): value is PermissionRule => {
  if (!value || Array.isArray(value) || typeof value !== 'object') return false;
  const rule = value as Record<string, unknown>;
  return (
    typeof rule['action'] === 'string' && typeof rule['resource'] === 'string' && typeof rule['effect'] === 'string'
  );
};

// Parses the V2 `permissions` array: `{action, resource, effect}` entries in order.
export function parsePermission(value: string): PermissionParse {
  try {
    const parsed: unknown = JSON.parse(value || '[]');
    if (!Array.isArray(parsed)) return { ok: false };
    if (!parsed.every(isRule)) return { ok: false };
    return { ok: true, rules: parsed };
  } catch {
    return { ok: false };
  }
}

export function permissionView(permission: string): PermissionView {
  const parsed = parsePermission(permission);
  if (parsed.ok) return { mode: 'rules', rules: parsed.rules };
  // Distinguish malformed JSON from a valid array with non-rule entries.
  try {
    return Array.isArray(JSON.parse(permission || '[]')) ? { mode: 'json' } : { mode: 'invalid' };
  } catch {
    return permission.trim() === '' ? { mode: 'rules', rules: [] } : { mode: 'invalid' };
  }
}

export function serializePermissionRules(rules: PermissionRule[]): string {
  return JSON.stringify(rules, null, 2);
}

// Returns the parsed rules, or throws so callers surface the textarea's error state.
export function permissionRules(permission: string): PermissionRule[] {
  const parsed = parsePermission(permission);
  if (!parsed.ok) throw new Error('Permissions must be an array of {action, resource, effect} rules');
  return parsed.rules;
}

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
