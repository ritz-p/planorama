const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'local', 'unknown', 'destination', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/imagebuilder-distribution-plan.json', 'utf8'));
  if (scenario === 'local') {
    for (const resource of plan.configuration.root_module.resources) resource.provider_config_key = 'aws';
    plan.configuration.provider_config.aws.expressions.region.constant_value = 'us-west-2';
    const destination = plan.resource_changes[9].change.after.distribution[0];
    destination.launch_template_configuration[0].account_id = null;
    destination.ssm_parameter_configuration[0].ami_account_id = null;
  }
  if (scenario === 'unknown') plan.resource_changes[10].change.after_unknown = {resource_selection: true};
  if (scenario === 'destination') plan.resource_changes[9].change.after_unknown = {distribution: true};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_imagebuilder_distribution_configuration.build'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    if (scenario === 'local') assert.ok(result.content.includes('Distribution repositories'));
    assert.ok(!result.content.includes('PRIVATE_'));
  }
}
console.log('Image Builder distribution native/Wasm SVG and HTML parity passed');
