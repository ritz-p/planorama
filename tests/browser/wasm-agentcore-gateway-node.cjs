const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown-gateway', 'foreign-stage', 'unknown-network', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/agentcore-gateway-plan.json', 'utf8'));
  if (scenario === 'unknown-gateway') plan.resource_changes[29].change.after_unknown = {gateway_identifier: true};
  if (scenario === 'foreign-stage') plan.configuration.root_module.resources[29].expressions.target_configuration[0].mcp[0].api_gateway[0].stage = {references: ['aws_api_gateway_stage.other.stage_name']};
  if (scenario === 'unknown-network') plan.resource_changes[21].change.after_unknown = {discovery_configuration: true};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_bedrockagentcore_gateway_rule.static'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), scenario + '/' + format);
    assert.ok(!result.content.includes('PRIVATE_'));
    if (scenario === 'complete') assert.ok(result.content.includes('Gateway-qualified route targets'));
  }
}
console.log('AgentCore gateway native/Wasm SVG and HTML parity passed');
