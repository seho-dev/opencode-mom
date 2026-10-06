export type McpServer = {
  name: string;
  type: 'local' | 'remote';
  disabled: boolean;
  target: string;
  sourcePath: string;
  sourcePaths: string[];
  config: Record<string, unknown>;
};

export type McpList = { data: McpServer[]; diagnostics: string[] };

export type McpDraft = { name: string; config: Record<string, unknown> };
export type McpUpdate = McpDraft & {
  expectedConfig: Record<string, unknown>;
  expectedSourcePath: string;
};
