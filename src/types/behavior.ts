export type BehaviorDocument = {
  path: string;
  resolvedPath: string | null;
  content: string;
  exists: boolean;
};

export type BehaviorUpdate = { content: string; expectedPath: string | null; expectedContent: string | null };
