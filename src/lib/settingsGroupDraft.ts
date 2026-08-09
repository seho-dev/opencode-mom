import type {
  ModelGroup,
  ModelGroupAgentOverride,
  ModelGroupCategoryMapping,
  OpenCodeAgentMappingPresentation,
  SettingsCommandClient,
  Uuid
} from './contracts';
import { commandErrorMessage } from './tauriClient';

export type ExactModelMatchCounts = Readonly<{
  categoryMappings: number;
  agentOverrides: number;
  openCodeAgentOverrides: number;
}>;

export type ExactModelMatchReplacement = Readonly<{
  group: ModelGroup;
  matchCounts: ExactModelMatchCounts;
}>;

export type GroupValidationResult =
  | Readonly<{ kind: 'valid'; value: ModelGroup }>
  | Readonly<{ kind: 'invalid'; message: string }>;

export const emptyMatchCounts: ExactModelMatchCounts = {
  categoryMappings: 0,
  agentOverrides: 0,
  openCodeAgentOverrides: 0
};

// Rust's `str::trim` includes U+0085 but intentionally does not trim U+FEFF.
const isRustWhitespaceForExactModel = (codePoint: number): boolean =>
  (codePoint >= 0x0009 && codePoint <= 0x000D)
  || codePoint === 0x0020
  || codePoint === 0x0085
  || codePoint === 0x00A0
  || codePoint === 0x1680
  || (codePoint >= 0x2000 && codePoint <= 0x200A)
  || codePoint === 0x2028
  || codePoint === 0x2029
  || codePoint === 0x202F
  || codePoint === 0x205F
  || codePoint === 0x3000;

export const trimExactModelValue = (value: string): string => {
  let start = 0;
  let end = value.length;
  while (start < end && isRustWhitespaceForExactModel(value.charCodeAt(start))) start += 1;
  while (start < end && isRustWhitespaceForExactModel(value.charCodeAt(end - 1))) end -= 1;
  return value.slice(start, end);
};

export const emptyOpenCodePresentation: OpenCodeAgentMappingPresentation = {
  discoveredRows: [],
  staleOverrides: [],
  preservedOverrides: [],
  discoveryError: null,
  isReadOnly: false,
  allowsCustomAgentCreation: false
};

export const matchCountTotal = (counts: ExactModelMatchCounts): number =>
  counts.categoryMappings + counts.agentOverrides + counts.openCodeAgentOverrides;

export const countExactModelMatches = (
  searchValue: string,
  group: ModelGroup
): ExactModelMatchCounts => {
  const search = trimExactModelValue(searchValue);
  if (search.length === 0) return emptyMatchCounts;
  return {
    categoryMappings: group.categoryMappings.filter((mapping) => trimExactModelValue(mapping.modelRef) === search).length,
    agentOverrides: group.agentOverrides.filter((override) => trimExactModelValue(override.modelRef) === search).length,
    openCodeAgentOverrides: group.openCodeAgentOverrides.filter((override) => trimExactModelValue(override.modelRef) === search).length
  };
};

export const replaceExactModelMatches = (
  searchValue: string,
  replaceValue: string,
  group: ModelGroup
): ExactModelMatchReplacement => {
  const matchCounts = countExactModelMatches(searchValue, group);
  const search = trimExactModelValue(searchValue);
  if (search.length === 0) return { group, matchCounts };
  const replacement = trimExactModelValue(replaceValue);
  const replace = (modelRef: string): string => trimExactModelValue(modelRef) === search ? replacement : modelRef;
  return {
    matchCounts,
    group: {
      ...group,
      categoryMappings: group.categoryMappings.map((mapping) => ({ ...mapping, modelRef: replace(mapping.modelRef) })),
      agentOverrides: group.agentOverrides.map((override) => ({ ...override, modelRef: replace(override.modelRef) })),
      openCodeAgentOverrides: group.openCodeAgentOverrides.map((override) => ({ ...override, modelRef: replace(override.modelRef) }))
    }
  };
};

export const cloneGroup = (group: ModelGroup): ModelGroup => ({
  ...group,
  categoryMappings: [...group.categoryMappings],
  agentOverrides: [...group.agentOverrides],
  openCodeAgentOverrides: [...group.openCodeAgentOverrides]
});

export const equalEditableGroup = (left: ModelGroup, right: ModelGroup): boolean =>
  left.id === right.id
  && left.name === right.name
  && left.description === right.description
  && left.isEnabled === right.isEnabled
  && equalCategoryMappings(left.categoryMappings, right.categoryMappings)
  && equalAgentOverrides(left.agentOverrides, right.agentOverrides)
  && equalAgentOverrides(left.openCodeAgentOverrides, right.openCodeAgentOverrides);

export const createDraftGroup = (existingGroups: readonly ModelGroup[]): ModelGroup => ({
  id: createUuid(),
  name: uniqueGroupName('Untitled Group', existingGroups.map((group) => group.name)),
  description: null,
  categoryMappings: [],
  agentOverrides: [],
  openCodeAgentOverrides: [],
  isEnabled: true,
  updatedAt: new Date().toISOString()
});

export const duplicateDraftGroup = (source: ModelGroup, existingGroups: readonly ModelGroup[]): ModelGroup => ({
  ...cloneGroup(source),
  id: createUuid(),
  name: uniqueGroupName(`${source.name} Copy`, existingGroups.map((group) => group.name)),
  updatedAt: new Date().toISOString()
});

export const validateGroup = (draftGroup: ModelGroup, existingGroups: readonly ModelGroup[]): GroupValidationResult => {
  const name = draftGroup.name.trim();
  if (name.length === 0) return { kind: 'invalid', message: 'Group name is required.' };
  const duplicate = existingGroups.some(
    (group) => group.id !== draftGroup.id && group.name.trim().toLowerCase() === name.toLowerCase()
  );
  if (duplicate) return { kind: 'invalid', message: `A group named "${name}" already exists.` };
  return {
    kind: 'valid',
    value: {
      ...draftGroup,
      name,
      description: optionalText(draftGroup.description ?? ''),
      categoryMappings: cleanCategoryMappings(draftGroup.categoryMappings),
      agentOverrides: cleanAgentOverrides(draftGroup.agentOverrides),
      openCodeAgentOverrides: cleanAgentOverrides(draftGroup.openCodeAgentOverrides),
      updatedAt: new Date().toISOString()
    }
  };
};

export const discoverPresentation = async (
  client: SettingsCommandClient,
  group: ModelGroup
): Promise<OpenCodeAgentMappingPresentation> => client.discoverOpenCodeAgents(group.openCodeAgentOverrides).then(
  (response) => response.presentation,
  (error: unknown) => degradedOpenCodePresentation(group, commandErrorMessage(error))
);

export const updateCategoryMapping = (
  mappings: readonly ModelGroupCategoryMapping[],
  index: number,
  patch: Partial<ModelGroupCategoryMapping>
): readonly ModelGroupCategoryMapping[] =>
  mappings.map((mapping, mappingIndex) => mappingIndex === index ? { ...mapping, ...patch } : mapping);

export const updateAgentOverride = (
  overrides: readonly ModelGroupAgentOverride[],
  index: number,
  patch: Partial<ModelGroupAgentOverride>
): readonly ModelGroupAgentOverride[] =>
  overrides.map((override, overrideIndex) => overrideIndex === index ? { ...override, ...patch } : override);

export const updateOpenCodeOverride = (
  overrides: readonly ModelGroupAgentOverride[],
  agentName: string,
  modelRef: string
): readonly ModelGroupAgentOverride[] => {
  const trimmedModel = modelRef.trim();
  const existing = overrides.find((override) => override.agentName === agentName);
  const otherOverrides = overrides.filter((override) => override.agentName !== agentName);
  if (trimmedModel.length === 0) return otherOverrides;
  const updated = { agentName, modelRef: trimmedModel };
  return existing ? [...otherOverrides, updated] : [...overrides, updated];
};

const degradedOpenCodePresentation = (
  group: ModelGroup,
  discoveryError: string
): OpenCodeAgentMappingPresentation => ({
  discoveredRows: [],
  staleOverrides: [],
  preservedOverrides: group.openCodeAgentOverrides.map((override) => ({
    id: `preserved:${override.agentName}`,
    agentName: override.agentName,
    modelRef: override.modelRef,
    status: 'Preserved',
    message: 'Editing disabled until OpenCode agent discovery succeeds.'
  })),
  discoveryError,
  isReadOnly: true,
  allowsCustomAgentCreation: false
});

const cleanCategoryMappings = (mappings: readonly ModelGroupCategoryMapping[]): readonly ModelGroupCategoryMapping[] =>
  mappings
    .map((mapping) => ({ categoryName: mapping.categoryName.trim(), modelRef: mapping.modelRef.trim() }))
    .filter((mapping) => mapping.categoryName.length > 0 || mapping.modelRef.length > 0);

const equalCategoryMappings = (
  left: readonly ModelGroupCategoryMapping[],
  right: readonly ModelGroupCategoryMapping[]
): boolean => left.length === right.length && left.every((mapping, index) => {
  const candidate = right[index];
  return candidate?.categoryName === mapping.categoryName && candidate.modelRef === mapping.modelRef;
});

const cleanAgentOverrides = (overrides: readonly ModelGroupAgentOverride[]): readonly ModelGroupAgentOverride[] =>
  overrides
    .map((override) => ({ agentName: override.agentName.trim(), modelRef: override.modelRef.trim() }))
    .filter((override) => override.agentName.length > 0 || override.modelRef.length > 0);

const equalAgentOverrides = (
  left: readonly ModelGroupAgentOverride[],
  right: readonly ModelGroupAgentOverride[]
): boolean => left.length === right.length && left.every((override, index) => {
  const candidate = right[index];
  return candidate?.agentName === override.agentName && candidate.modelRef === override.modelRef;
});

const optionalText = (value: string): string | null => {
  const trimmed = value.trim();
  return trimmed.length === 0 ? null : trimmed;
};

const uniqueGroupName = (baseName: string, existingNames: readonly string[]): string => {
  const normalized = new Set(existingNames.map((name) => name.trim().toLowerCase()));
  const base = baseName.trim() || 'Untitled Group';
  if (!normalized.has(base.toLowerCase())) return base;
  let suffix = 2;
  while (normalized.has(`${base} ${suffix}`.toLowerCase())) suffix += 1;
  return `${base} ${suffix}`;
};

const createUuid = (): Uuid => {
  const randomUuid = globalThis.crypto?.randomUUID?.();
  return randomUuid ?? `fixture-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`;
};
