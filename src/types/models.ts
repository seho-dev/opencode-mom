export type ModelRef = `${string}/${string}`;

export type ModelModality = 'text' | 'audio' | 'image' | 'video' | 'pdf';
export interface ModelCapabilities {
  tools?: boolean;
  input?: ModelModality[];
  output?: ModelModality[];
}
export interface ModelCost {
  input: number;
  output: number;
  cache?: { read?: number; write?: number };
}
export interface ModelLimit {
  context: number;
  output: number;
  input?: number;
}
export interface ModelVariant {
  id: string;
  settings?: Record<string, unknown>;
  headers?: Record<string, string>;
  body?: unknown;
}
export interface ModelDef {
  id: string;
  name?: string;
  family?: string;
  disabled?: boolean;
  capabilities?: ModelCapabilities;
  cost?: ModelCost;
  limit?: ModelLimit;
  settings?: Record<string, unknown>;
  headers?: Record<string, string>;
  variants?: ModelVariant[];
}

export interface ModelCatalogEntry {
  providerId: string;
  modelId: string;
  ref: ModelRef;
  name: string;
  isCustom: boolean;
  limit?: { context?: number; input?: number; output?: number } | null;
  variants?: ModelVariant[] | null;
}
