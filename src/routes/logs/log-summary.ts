import type { MessageKey } from '$src/i18n/dictionaries/en.js';
import type { I18n } from '$src/i18n/i18n.svelte.js';

const SUMMARY_KEYS = new Map<string, MessageKey>([
  ['Operation completed', 'logs.summary.operationCompleted'],
  ['Operation failed', 'logs.summary.operationFailed'],
  ['Checkpoint reached', 'logs.summary.checkpointReached'],
  ['Event completed', 'logs.summary.eventCompleted'],
  ['Event failed', 'logs.summary.eventFailed'],
]);

export function logSummary(message: string, i18n: I18n): string {
  const key = SUMMARY_KEYS.get(message);
  return key ? i18n.t(key) : message;
}
