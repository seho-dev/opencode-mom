// Shared domain constants. Single source of truth for config string unions, labels and option lists.

export const GROUP_TYPE_OPENCODE = 'opencode';
export const GROUP_TYPE_SLIM = 'slim';
export const GROUP_TYPE_OMO = 'oh-my-openagent';

export const AGENT_SOURCES = ['inline', 'markdown', 'both'] as const;
export const AGENT_STORAGES = ['inline', 'global_markdown', 'project_markdown'] as const;
export const GROUP_TYPES = [GROUP_TYPE_OPENCODE, GROUP_TYPE_SLIM, GROUP_TYPE_OMO] as const;
export const BUILTIN_SYSTEMS = [GROUP_TYPE_SLIM, GROUP_TYPE_OMO] as const;
export const AGENT_MODES = ['primary', 'subagent', 'all'] as const;
export const COLOR_THEMES = ['primary', 'secondary', 'accent', 'success', 'warning', 'error', 'info'] as const;
export const PERMISSION_ACTIONS = ['allow', 'ask', 'deny'] as const;
export const PERMISSION_KEYS = [
  'read',
  'edit',
  'glob',
  'grep',
  'list',
  'bash',
  'task',
  'external_directory',
  'todowrite',
  'webfetch',
  'websearch',
  'lsp',
  'skill',
  'question',
  'doom_loop',
] as const;
export const MAPPING_KINDS = ['native', GROUP_TYPE_SLIM, 'omo', 'category'] as const;

export type AgentSource = (typeof AGENT_SOURCES)[number];
export type AgentStorage = (typeof AGENT_STORAGES)[number];
export type GroupType = (typeof GROUP_TYPES)[number];
export type BuiltinSystem = (typeof BUILTIN_SYSTEMS)[number];
export type AgentMode = (typeof AGENT_MODES)[number];
export type MappingKind = (typeof MAPPING_KINDS)[number];

export const STORAGE_LABELS: Record<AgentStorage, string> = {
  inline: 'Inline',
  global_markdown: 'Global Markdown',
  project_markdown: 'Project Markdown',
};

export const GROUP_TYPE_LABELS: Record<GroupType, string> = {
  [GROUP_TYPE_OPENCODE]: 'OpenCode',
  [GROUP_TYPE_SLIM]: 'Slim',
  [GROUP_TYPE_OMO]: 'OhMyOpenAgent',
};

export const STORAGE_OPTIONS = AGENT_STORAGES.map((value) => ({ value, label: STORAGE_LABELS[value] }));
export const GROUP_TYPE_OPTIONS = GROUP_TYPES.map((value) => ({ value, label: GROUP_TYPE_LABELS[value] }));
export const MARKDOWN_STORAGE_OPTIONS = STORAGE_OPTIONS.filter((option) => option.value !== 'inline');
