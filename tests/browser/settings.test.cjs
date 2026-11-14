const assert = require('node:assert/strict');
const {test} = require('node:test');
const settings = require('../../browser/settings.js');
const types = ['aws_iam_role', 'aws_ssm_parameter'];
test('browser settings round trip exact selectors, all options, false, defaults and empty arrays', () => {
  const selected = {...settings.defaults(), format: 'html', changed_only: true,
    address_format: 'terraform', hide_relationship_legend: true, omit_relationship_legend: true,
    html_diagnostics: true, html_semantic_diagnostics: true,
    modules: ['state:module.app["a b,c.d\\"e"]', 'module.missing', 'module.missing'],
    focus: 'state:module.app["a b,c.d\\"e"].aws_instance.x', focus_depth: 0, spatial_types: types};
  for (const value of [settings.defaults(), selected, {...selected, focus_depth: 4294967295}]) {
    assert.deepEqual(settings.parse(settings.stringify(value, types), types), value);
  }
  const defaults = settings.parse('{"schema_version":1,"settings":{}}', types);
  assert.deepEqual(defaults, settings.defaults());
  assert.equal(JSON.parse(settings.stringify(defaults, types)).schema_version, 1);
});
test('invalid files are rejected without mutation or raw error contents', () => {
  for (const value of [null, [], {format: 'pdf'}, {changed_only: 0}, {modules: ['']},
    {modules: Array(257).fill('module.a')}, {focus: ''}, {focus_depth: 1},
    {focus: 'aws_instance.a', focus_depth: -1}, {focus: 'x', focus_depth: 1.1},
    {focus: 'x', focus_depth: 4294967296}, {spatial_types: ['unknown']},
    {spatial_types: [types[0], types[0]]}, {html_diagnostics: true}, {plan: 'PRIVATE'},
    {remote_state_mappings: []}, {memory_budget_mib: 512}]) {
    const before = JSON.stringify(value);
    assert.throws(() => settings.validate(value, types), /Unsupported or invalid/);
    assert.equal(JSON.stringify(value), before);
  }
  for (const text of ['PRIVATE', '{}', '[]', '{"schema_version":2,"settings":{}}',
    '{"schema_version":1,"settings":{},"plan":"PRIVATE"}',
    '{"schema_version":1,"settings":{"__proto__":{}}}', ' '.repeat(settings.maxBytes + 1)]) {
    assert.throws(() => settings.parse(text, types), /Unsupported or invalid/);
  }
  assert.throws(() => settings.stringify({...settings.defaults(), focus: '界'.repeat(30000)}, types));
});
