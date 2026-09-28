import type { ModelDef } from './models.js';

export interface ProviderSettings {
  apiKey?: string;
  baseURL?: string;
}

export interface ProviderDef {
  name: string;
  package?: string;
  settings?: ProviderSettings;
  headers?: Record<string, string>;
  models: Record<string, ModelDef>;
}
