import assert from 'node:assert/strict';
import test from 'node:test';
import { providerSchema } from './schema.ts';

test('provider schema validates required fields and string-valued headers', () => {
  const messages = {
    required: 'required',
    json: 'invalid JSON',
    object: 'expected object',
    strings: 'expected strings',
  };
  const create = providerSchema(messages);
  const valid = { name: 'Provider', package: '@ai-sdk/openai', baseURL: '', apiKey: '', headers: '{"X-Key":"secret"}' };
  assert.deepEqual(create.parse(valid).headers, { 'X-Key': 'secret' });
  assert.equal(create.parse({ ...valid, headers: '  ' }).headers, undefined);

  for (const [field, value] of [
    ['name', '  '],
    ['package', ''],
  ]) {
    const result = create.safeParse({ ...valid, [field]: value });
    assert.equal(result.success, false);
    assert.equal(result.error.issues.find((issue) => issue.path[0] === field)?.message, messages.required);
  }
  for (const [headers, error] of [
    ['{', messages.json],
    ['[]', messages.object],
    ['{"X-Key":123}', messages.strings],
  ]) {
    const result = create.safeParse({ ...valid, headers });
    assert.equal(result.success, false, headers);
    assert.equal(result.error.issues.find((issue) => issue.path[0] === 'headers')?.message, error);
  }
  assert.equal(providerSchema(messages, false).safeParse({ ...valid, package: '' }).success, true);
});
