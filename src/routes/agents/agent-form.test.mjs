import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('agent form rejects invalid fields and preserves valid permission rules', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { agentSchema, emptyAgentValues, fieldError } = await vite.ssrLoadModule('/src/routes/agents/agentForm.ts');
    const messages = {
      idRequired: 'id required',
      idReserved: 'reserved',
      descriptionRequired: 'description required',
      stepsInvalid: 'invalid steps',
      permissionsInvalid: 'invalid permissions',
    };
    const schema = agentSchema(messages, true);
    const valid = { ...emptyAgentValues, id: 'my-agent', description: 'Description' };
    assert.equal(schema.safeParse(valid).success, true);
    assert.equal(
      schema.safeParse({ ...valid, permission: '[{"action":"*","resource":"*","effect":"ask"}]' }).success,
      true,
    );

    for (const [name, value, error] of [
      ['id', '   ', messages.idRequired],
      ['description', '  ', messages.descriptionRequired],
      ['steps', '0', messages.stepsInvalid],
      ['steps', '9007199254740992', messages.stepsInvalid],
      ['permission', '{', messages.permissionsInvalid],
      ['permission', '[1]', messages.permissionsInvalid],
    ]) {
      const result = schema.safeParse({ ...valid, [name]: value });
      assert.equal(result.success, false, `${name}=${value}`);
      assert.equal(result.error.issues.find((issue) => issue.path[0] === name)?.message, error);
    }
    const reserved = schema.safeParse({ ...valid, id: 'build' });
    assert.equal(reserved.success, false);
    assert.equal(reserved.error.issues.find((issue) => issue.path[0] === 'id')?.message, messages.idReserved);
    assert.equal(fieldError([{ message: 'bad field' }]), 'bad field');
  } finally {
    await vite.close();
  }
});
