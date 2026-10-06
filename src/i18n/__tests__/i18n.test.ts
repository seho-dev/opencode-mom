import { expect, test } from 'vitest';
import { en, type MessageKey } from '../dictionaries/en.js';
import { zh } from '../dictionaries/zh.js';
import { createI18n } from '../i18n.svelte.js';

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

test('tray dictionaries cover matching keys and interpolation parameters in both locales', () => {
  const keys = Object.keys(en).filter((key) => key.startsWith('tray.')) as MessageKey[];
  expect(keys.length).toBeGreaterThan(0);
  for (const key of keys) {
    expect(zh[key]).toBeTruthy();
    expect([...zh[key].matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort()).toEqual(
      [...en[key].matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort(),
    );
  }
  const i18n = createI18n(() => 'zh');
  expect(i18n.t('dashboard.periodTotal', { period: '2026' })).toBe(
    zh['dashboard.periodTotal'].replace('{period}', '2026'),
  );
  expect(zh['dashboard.periodTotal']).toContain(' · {period}');
});
