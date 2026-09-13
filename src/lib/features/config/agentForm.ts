// Pure, framework-free helpers shared by the agent create/edit pages.
import type { OptionRow, PermissionParse, PermissionView } from './types.js';

// Minimal shape accepted from both the CLI catalog and the provider store; only these fields are read.
export type ModelOptionEntry = { ref: string; name?: string; variants?: Record<string, unknown> | null };

export type AgentFieldValues = {
  model: string;
  mode: string;
  description: string;
  disable: boolean;
  hidden: boolean;
  color: string;
  variant: string;
  temperature: string;
  topP: string;
  steps: string;
  prompt: string;
};

export function pretty(value: unknown): string {
  return JSON.stringify(value ?? {}, null, 2);
}

export function optionRows(value: unknown): OptionRow[] {
  return Object.entries(value && typeof value === 'object' && !Array.isArray(value) ? value : {}).map(
    ([key, item]) => ({
      key,
      value: typeof item === 'string' ? item : JSON.stringify(item),
    }),
  );
}

export function optionObject(rows: OptionRow[]): Record<string, unknown> {
  return Object.fromEntries(
    rows
      .filter((row) => row.key.trim())
      .map((row) => {
        try {
          return [row.key.trim(), JSON.parse(row.value)];
        } catch {
          return [row.key.trim(), row.value];
        }
      }),
  );
}

export function parsePermission(value: string): PermissionParse {
  try {
    const parsed = JSON.parse(value || '{}');
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') return { ok: false };
    return { ok: true, object: parsed as Record<string, unknown> };
  } catch {
    return { ok: false };
  }
}

export function permissionView(permission: string): PermissionView {
  const parsed = parsePermission(permission);
  if (!parsed.ok) return { mode: 'invalid' };
  const values: Record<string, string> = {};
  for (const [key, value] of Object.entries(parsed.object)) {
    if (typeof value !== 'string') return { mode: 'json' };
    values[key] = value;
  }
  return { mode: 'rows', values };
}

// Returns the updated JSON string; invalid input is returned unchanged so the textarea keeps the user's text.
export function setPermissionAction(permission: string, key: string, action: string): string {
  const parsed = parsePermission(permission);
  if (!parsed.ok) return permission;
  const next = { ...parsed.object };
  if (action) next[key] = action;
  else delete next[key];
  return JSON.stringify(next, null, 2);
}

export function permissionObject(permission: string): Record<string, unknown> {
  const parsed = parsePermission(permission);
  if (!parsed.ok) throw new Error('Permissions must be a valid JSON object');
  return parsed.object;
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
  const value =
    catalog.find((entry) => entry.ref === current)?.variants ?? models.find((entry) => entry.ref === current)?.variants;
  return value && typeof value === 'object' ? Object.keys(value) : [];
}
