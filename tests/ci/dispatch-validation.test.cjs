const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const path = require('node:path');
const {test} = require('node:test');

const workflow = readFileSync(path.join(__dirname, '../../.github/workflows/update_examples.yml'), 'utf8');
const start = workflow.indexOf('            async function verifyDispatch(');
const end = workflow.indexOf('            async function dispatchChecks(', start);
assert.ok(start >= 0 && end > start, 'The SVG writeback must verify each dispatched run');
const code = `${workflow.slice(start, end)}\nreturn verifyDispatch;`;
const sha = 'a'.repeat(40);
const valid = {
  id: 123, head_sha: sha, event: 'workflow_dispatch', head_branch: 'feature/topic',
  path: '.github/workflows/cargo_test.yml', display_title: `Validate PR #333 at ${sha}`,
};

function setup(responses) {
  const calls = [];
  const delays = [];
  const messages = [];
  const request = async (method, resource) => {
    assert.equal(method, 'GET');
    assert.equal(resource, 'actions/runs/123');
    const response = responses[Math.min(calls.length, responses.length - 1)];
    calls.push({method, resource});
    if (response instanceof Error) throw response;
    return response;
  };
  const verify = new Function('request', 'delay', 'core', 'number', code)(
    request, async delay => delays.push(delay), {info: message => messages.push(message)}, 333,
  );
  return {calls, delays, messages, invoke: () => verify(123, 'cargo_test.yml', sha, 'feature/topic')};
}

test('verifies an immediately available run without waiting', async () => {
  const environment = setup([{...valid, path: `${valid.path}@feature/topic`}]);
  await environment.invoke();
  assert.equal(environment.calls.length, 1);
  assert.deepEqual(environment.delays, []);
});

test('retries absent and incomplete metadata before accepting the exact run', async () => {
  const missing = Object.assign(new Error('Not found'), {status: 404});
  const environment = setup([missing, {...valid, display_title: 'Workflow requested'}, valid]);
  await environment.invoke();
  assert.equal(environment.calls.length, 3);
  assert.deepEqual(environment.delays, [2000, 2000]);
  assert.match(environment.messages[0], /HTTP 404/);
  assert.match(environment.messages[1], /display_title=.*Workflow requested.*expected.*Validate PR/);
});

for (const [field, value] of Object.entries({
  id: 124, head_sha: 'b'.repeat(40), event: 'push', head_branch: 'feature/other',
  path: '.github/workflows/unrelated.yml', display_title: `Validate PR #334 at ${sha}`,
})) {
  test(`never accepts a persistent ${field} mismatch`, async () => {
    const environment = setup([{...valid, [field]: value}]);
    await assert.rejects(environment.invoke(), new RegExp(`expected PR validation: ${field}=`));
    assert.equal(environment.calls.length, 6);
    assert.equal(environment.delays.length, 5);
    assert.ok(environment.messages.every(message => message.includes(`${field}=`)));
  });
}

test('propagates an API denial without treating it as initialization delay', async () => {
  const environment = setup([Object.assign(new Error('Forbidden'), {status: 403})]);
  await assert.rejects(environment.invoke(), /Forbidden/);
  assert.equal(environment.calls.length, 1);
  assert.deepEqual(environment.delays, []);
});
