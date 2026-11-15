const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown-version', 'public-network', 'ambiguous-tool', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/agentcore-runtime-plan.json', 'utf8'));
  if (scenario === 'unknown-version') plan.resource_changes[15].change.after_unknown = {agent_runtime_version: true};
  if (scenario === 'public-network') plan.resource_changes[14].change.after.network_configuration[0].network_mode = 'PUBLIC';
  if (scenario === 'ambiguous-tool') plan.resource_changes[18].change.after.tool[0].config[0].remote_mcp = [{}];
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_bedrockagentcore_agent_runtime_endpoint.main'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), scenario + '/' + format);
    assert.ok(!result.content.includes('PRIVATE_'));
    if (scenario === 'complete') assert.ok(result.content.includes('Version-qualified runtime'));
  }
}
console.log('AgentCore runtime native/Wasm SVG and HTML parity passed');
