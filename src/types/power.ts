export type LidPhase = 'disabled' | 'idle' | 'checking' | 'protected' | 'unknown' | 'error';

export interface LidState {
  enabled: boolean;
  phase: LidPhase;
  activeSessions?: number;
  error?: string;
}
