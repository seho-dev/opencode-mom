export interface TokenUsageRecord {
  time: number;
  input: number;
  output: number;
}

export interface DailyTokenUsage {
  date: string;
  input: number;
  output: number;
}

export interface TokenUsageSnapshot {
  records: TokenUsageRecord[];
  updatedAt: number | null;
  refreshing: boolean;
  stale: boolean;
  error: { message: string } | null;
  targetRevision: number;
  resultRevision: number;
  cacheScope: string;
}
