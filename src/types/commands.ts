export type SlashCommandEntry = {
  id: string;
  name: string;
  description?: string;
  agent?: string;
  model?: string;
  subagent?: boolean;
  sourcePath: string;
  resolvedPath: string;
  content: string;
};

export type SlashCommandList = { data: SlashCommandEntry[]; diagnostics: string[] };
export type SlashCommandDraft = { name: string; content: string };
export type SlashCommandUpdate = { id: string; content: string; expectedPath: string; expectedContent: string };
export type SlashCommandDelete = { id: string; expectedPath: string; expectedContent: string };
