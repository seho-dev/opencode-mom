export type NotificationPermission = 'prompt' | 'granted' | 'denied' | 'unavailable' | 'unsupported';

export interface NotificationSettings {
  enabled: boolean;
  agentCompleted: boolean;
  approvalRequired: boolean;
  agentError: boolean;
}
