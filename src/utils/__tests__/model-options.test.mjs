import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('model choices merge catalog and providers while retaining unavailable selections', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const {
      buildModelOptions,
      buildModelVariants,
      splitModelSelector,
      joinModelSelector,
      isJsonObject,
      safeParseJsonObject,
      prettyJson,
    } = await vite.ssrLoadModule('/src/utils/index.ts');
    const catalog = [{ ref: 'a/one', name: 'Catalog name', variants: [{ id: 'fast' }] }];
    const models = [
      { ref: 'a/one', name: 'Provider name', variants: [{ id: 'slow' }] },
      { ref: 'b/two', name: 'Provider only', variants: [{ id: 'balanced' }] },
    ];
    assert.deepEqual(buildModelOptions(catalog, models, 'z/missing', 'None'), [
      { value: '', label: 'None' },
      { value: 'a/one', label: 'Catalog name' },
      { value: 'b/two', label: 'Provider only' },
      { value: 'z/missing', label: 'z/missing (unavailable)' },
    ]);
    assert.equal(buildModelOptions(catalog, models, 'a/one', 'None').length, 3);
    assert.deepEqual(buildModelVariants(catalog, models, 'a/one'), ['fast']);
    assert.deepEqual(buildModelVariants(catalog, models, 'b/two'), ['balanced']);
    assert.deepEqual(buildModelVariants(catalog, models, 'z/missing'), []);

    assert.deepEqual(splitModelSelector('openai/gpt-4o#fast'), { model: 'openai/gpt-4o', variant: 'fast' });
    assert.deepEqual(splitModelSelector('openai/gpt-4o'), { model: 'openai/gpt-4o', variant: '' });
    assert.deepEqual(splitModelSelector(''), { model: '', variant: '' });

    assert.equal(joinModelSelector('openai/gpt-4o', 'fast'), 'openai/gpt-4o#fast');
    assert.equal(joinModelSelector('openai/gpt-4o', ''), 'openai/gpt-4o');
    assert.equal(joinModelSelector('', 'fast'), '');

    assert.equal(isJsonObject({ a: 1 }), true);
    assert.equal(isJsonObject([]), false);
    assert.equal(isJsonObject(null), false);
    assert.equal(isJsonObject('text'), false);

    assert.deepEqual(safeParseJsonObject('{"key":"value"}'), { key: 'value' });
    assert.equal(safeParseJsonObject(''), null);
    assert.equal(safeParseJsonObject('   '), null);
    assert.equal(safeParseJsonObject('[]'), null);
    assert.equal(safeParseJsonObject('{bad json'), null);
    assert.equal(safeParseJsonObject('123'), null);

    assert.equal(prettyJson({ a: 1 }), '{\n  "a": 1\n}');
    assert.equal(prettyJson(undefined), '{}');
  } finally {
    await vite.close();
  }
});
