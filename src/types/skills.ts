export type SkillEntry = {
  id: string;
  kind: 'local' | 'remote';
  name: string;
  description?: string;
  autoinvoke: boolean;
  path: string;
  content: string;
  source: string;
};

export type SkillList = { data: SkillEntry[]; diagnostics: string[] };

export type SkillDraft = { id: string; content: string };
export type SkillUpdate = SkillDraft & { expectedContent: string; expectedPath: string };
