export type PluginConfigEntry = string | { package: string; options?: Record<string, unknown> };

export type PluginConfigSource =
  | { type: 'document'; path?: string; plugins: PluginConfigEntry[] }
  | { type: 'directory'; path: string };

export type PluginConfigPreview = { directory: string; data: PluginConfigSource[] };

export type PluginRemovalTarget = { type: 'package'; target: string } | { type: 'local'; path: string };

export type PluginRemovalPlan = {
  target: PluginRemovalTarget;
  sources: { path: string; expectedPath: string; expectedHash: string }[];
  serviceKey: string;
};

export type PluginRuntimeItem = {
  id?: string;
  canUpdate: boolean;
  canRemove: boolean;
  source:
    | { type: 'builtin' | 'sdk' }
    | { type: 'local'; path: string }
    | { type: 'package'; target: string; version?: string; outdated?: boolean; updating?: boolean };
  features: { server?: boolean; tui?: boolean; rpc?: boolean };
  state: { status: 'active' } | { status: 'failed'; error: string; ref?: string };
};

export type PluginRuntime = { directory: string; data: PluginRuntimeItem[] };
