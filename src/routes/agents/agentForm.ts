import { createForm } from '@tanstack/svelte-form';
import { z } from 'zod';
import type { PermissionRule } from '$src/types/agents.js';
import type { PermissionParse, PermissionView } from '$src/types/ui.js';
import { isBuiltinAgentId } from '$src/utils/index.js';

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

export type AgentFormValues = AgentFieldValues & { id: string; permission: string };

export const emptyAgentValues: AgentFormValues = {
  id: '',
  model: '',
  mode: '',
  description: '',
  disabled: false,
  hidden: false,
  color: '',
  variant: '',
  steps: '',
  prompt: '',
  permission: '[]',
};

export function agentSchema(
  messages: {
    idRequired: string;
    idReserved: string;
    descriptionRequired: string;
    stepsInvalid: string;
    permissionsInvalid: string;
  },
  newAgent = false,
) {
  return z.object({
    id: z
      .string()
      .trim()
      .min(1, messages.idRequired)
      .refine((id) => !newAgent || !isBuiltinAgentId(id), messages.idReserved),
    model: z.string(),
    mode: z.string(),
    description: z.string().trim().min(1, messages.descriptionRequired),
    disabled: z.boolean(),
    hidden: z.boolean(),
    color: z.string(),
    variant: z.string(),
    steps: z
      .string()
      .refine(
        (value) => value === '' || (/^[1-9]\d*$/.test(value) && Number.isSafeInteger(Number(value))),
        messages.stepsInvalid,
      ),
    prompt: z.string(),
    permission: z.string().refine((value) => parsePermission(value).ok, messages.permissionsInvalid),
  });
}

export function createAgentForm(
  messages: Parameters<typeof agentSchema>[0],
  onSubmit: (value: AgentFormValues) => Promise<void>,
  newAgent = false,
) {
  return createForm(() => ({
    defaultValues: { ...emptyAgentValues },
    validators: { onSubmit: agentSchema(messages, newAgent) },
    onSubmit: async ({ value }) => onSubmit(value),
  }));
}

export type AgentForm = ReturnType<typeof createAgentForm>;

export function fieldError(errors: unknown[]): string {
  return errors
    .map((error) =>
      typeof error === 'string'
        ? error
        : error && typeof error === 'object' && 'message' in error
          ? String(error.message)
          : '',
    )
    .filter(Boolean)
    .join(', ');
}

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

export function agentMutationFields(values: AgentFormValues): Record<string, unknown> {
  const rules = permissionRules(values.permission);
  return {
    model: values.model ? joinModelSelector(values.model, values.variant) : undefined,
    mode: values.mode || undefined,
    description: values.description || undefined,
    disabled: values.disabled,
    hidden: values.hidden,
    color: values.color || undefined,
    steps: values.steps ? Number(values.steps) : undefined,
    prompt: values.prompt || undefined,
    ...(rules.length ? { permissions: rules } : {}),
  };
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
