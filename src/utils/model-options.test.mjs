import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('model choices merge catalog and providers while retaining unavailable selections', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { buildModelOptions, buildModelVariants } = await vite.ssrLoadModule('/src/utils/index.ts');
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
  } finally {
    await vite.close();
  }
});
