import type { LocalePreference } from '$lib/features/config/types.js';
import { en, type MessageKey } from './dictionaries/en.js';
import { zh } from './dictionaries/zh.js';

const dictionaries: Record<LocalePreference, Record<MessageKey, string>> = { en, zh };

export function createI18n(getLocale: () => LocalePreference) {
  return {
    get locale(): LocalePreference {
      return getLocale();
    },
    t(key: MessageKey, params?: Record<string, string | number>): string {
      const template = dictionaries[getLocale()][key] ?? en[key] ?? key;
      return params ? template.replace(/\{(\w+)\}/g, (_, name: string) => String(params[name] ?? '')) : template;
    },
  };
}
export type I18n = ReturnType<typeof createI18n>;
