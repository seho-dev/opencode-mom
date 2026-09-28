import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('model limits accept numeric token counts without unit conversion', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { isValidTokenCount } = await vite.ssrLoadModule('/src/utils/index.ts');
    for (const valid of ['256', '32000', '1000000', '65536']) assert.equal(isValidTokenCount(valid), true);
    for (const invalid of ['32K', '1M', '1.5K', '-256', 'Infinity', '9007199254740992']) {
      assert.equal(isValidTokenCount(invalid), false);
    }
  } finally {
    await vite.close();
  }
});
