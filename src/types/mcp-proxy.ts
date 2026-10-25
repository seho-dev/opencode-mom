export type McpProxyProtocol = 'legacy' | 'modern' | 'auto';

export interface McpProxyServerSummaryDto {
  id: string;
  name: string;
  enabled: boolean;
  protocol: string;
  activeSessions: number;
}

export interface McpProxyStateDto {
  actual: boolean;
  desired: boolean;
  actualState: 'running' | 'stopped';
  desiredState: boolean;
  boundAddr: string | null;
  port: number;
  allowLan: boolean;
  totalServices: number;
  enabledServices: number;
  activeUnits: number;
  activeSessions: number;
  availableUnits: number;
  maxUnits: number;
  securityErrors: string[];
  services: McpProxyServerSummaryDto[];
  revision: number;
  generation: number;
}

export interface McpProxySettingsDto {
  enabled: boolean;
  idleTimeout: number;
  maxUnits: number;
  revision: number;
}

export interface MomHttpSettings {
  port: number;
  allowLan: boolean;
}

export interface McpProxyServerDto {
  id: string;
  name: string;
  enabled: boolean;
  command: string[];
  cwd: string;
  env: Record<string, string>;
  protocol: string;
  startupTimeout: number;
  executionTimeout: number;
}

export interface ImportMcpProxyServerRequest {
  sourceScope: string;
  projectId?: string | null;
  serverName: string;
  expectedSourcePath?: string | null;
}

export interface CloseMcpProxyUnitRequest {
  unitId: string;
  expectedGeneration: number;
}
