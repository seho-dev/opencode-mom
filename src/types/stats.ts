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
