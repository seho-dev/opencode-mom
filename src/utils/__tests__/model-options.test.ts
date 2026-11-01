import { describe, expect, test } from 'vitest';
import {
  buildModelOptions,
  buildModelVariants,
  isJsonObject,
  isValidTokenCount,
  joinModelSelector,
  prettyJson,
  safeParseJsonObject,
  splitModelSelector,
} from '../index.js';

describe('model utilities', () => {
  test('model choices merge catalog and providers while retaining unavailable selections', () => {
    const catalog = [{ ref: 'a/one', name: 'Catalog name', variants: [{ id: 'fast' }] }];
    const models = [
      { ref: 'a/one', name: 'Provider name', variants: [{ id: 'slow' }] },
      { ref: 'b/two', name: 'Provider only', variants: [{ id: 'balanced' }] },
    ];

    expect(buildModelOptions(catalog, models, 'z/missing', 'None')).toEqual([
      { value: '', label: 'None' },
      { value: 'a/one', label: 'Catalog name' },
      { value: 'b/two', label: 'Provider only' },
      { value: 'z/missing', label: 'z/missing (unavailable)' },
    ]);
    expect(buildModelOptions(catalog, models, 'a/one', 'None').length).toBe(3);
    expect(buildModelVariants(catalog, models, 'a/one')).toEqual(['fast']);
    expect(buildModelVariants(catalog, models, 'b/two')).toEqual(['balanced']);
    expect(buildModelVariants(catalog, models, 'z/missing')).toEqual([]);

    expect(splitModelSelector('openai/gpt-4o#fast')).toEqual({ model: 'openai/gpt-4o', variant: 'fast' });
    expect(splitModelSelector('openai/gpt-4o')).toEqual({ model: 'openai/gpt-4o', variant: '' });
    expect(splitModelSelector('')).toEqual({ model: '', variant: '' });

    expect(joinModelSelector('openai/gpt-4o', 'fast')).toBe('openai/gpt-4o#fast');
    expect(joinModelSelector('openai/gpt-4o', '')).toBe('openai/gpt-4o');
    expect(joinModelSelector('', 'fast')).toBe('');

    expect(isJsonObject({ a: 1 })).toBe(true);
    expect(isJsonObject([])).toBe(false);
    expect(isJsonObject(null)).toBe(false);
    expect(isJsonObject('text')).toBe(false);

    expect(safeParseJsonObject('{"key":"value"}')).toEqual({ key: 'value' });
    expect(safeParseJsonObject('')).toBe(null);
    expect(safeParseJsonObject('   ')).toBe(null);
    expect(safeParseJsonObject('[]')).toBe(null);
    expect(safeParseJsonObject('{bad json')).toBe(null);
    expect(safeParseJsonObject('123')).toBe(null);

    expect(prettyJson({ a: 1 })).toBe('{\n  "a": 1\n}');
    expect(prettyJson(undefined)).toBe('{}');
  });

  test('model limits accept numeric token counts without unit conversion', () => {
    for (const valid of ['256', '32000', '1000000', '65536']) {
      expect(isValidTokenCount(valid)).toBe(true);
    }
    for (const invalid of ['32K', '1M', '1.5K', '-256', 'Infinity', '9007199254740992']) {
      expect(isValidTokenCount(invalid)).toBe(false);
    }
  });
});
