export const LOG_TYPES = [
  'app',
  'config',
  'group',
  'resource',
  'opencode',
  'http',
  'mcp_proxy',
  'power',
  'island',
  'logging',
] as const;

export type LogType = (typeof LOG_TYPES)[number];

export type LogStatus = 'success' | 'error';

export interface LoggingState {
  enabled: boolean;
  recording: boolean;
  directory: string;
  error: string | null;
  droppedRecords: number;
}

export interface LogRecord {
  schemaVersion: number;
  timestamp: string;
  sessionId: string;
  sequence: number;
  operationId: string;
  type: LogType;
  status: LogStatus;
  event: string;
  message: string;
  durationMs?: number | null;
  errorCode?: string | null;

  context?: {
    step?: string;
    count?: number;
    targetWritten?: boolean;
  } | null;
}

export interface LogQuery {
  type?: LogType;
  status?: LogStatus;
  from?: string;
  to?: string;
  operationId?: string;
  cursor?: string;
  limit?: number;
}

export interface LogPage {
  records: LogRecord[];
  nextCursor: string | null;
  invalidLines: number;
}
