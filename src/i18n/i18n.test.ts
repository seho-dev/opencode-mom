import { expect, test } from 'vitest';
import { en, type MessageKey } from './dictionaries/en.js';
import { createI18n } from './i18n.svelte.js';

test('interpolates translations, follows locale changes, and falls back for missing keys', () => {
  let locale: 'en' | 'zh' = 'en';
  const i18n = createI18n(() => locale);
  expect(i18n.t('common.pageOf', { page: 2, pages: 5 })).toBe('Page 2 of 5');
  expect(i18n.t('common.optionUnknown', { name: 'custom' })).toBe('custom (unknown)');
  expect(i18n.t('common.optionUnknown', {})).toBe(' (unknown)');

  locale = 'zh';
  expect(i18n.locale).toBe('zh');
  expect(i18n.t('common.pageOf', { page: 2, pages: 5 })).not.toBe(en['common.pageOf']);
  expect(i18n.t('missing.key' as MessageKey)).toBe('missing.key');
});
