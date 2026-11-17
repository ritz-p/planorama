const assert = require('node:assert/strict');
const fs = require('node:fs');
const {resolve, join} = require('node:path');
const {tmpdir} = require('node:os');
const {execFileSync} = require('node:child_process');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(fs.readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: fs.readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const binary = resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
const native = require('../support/native-reference.cjs')(binary);
const cases = [
  {fixture: 'bedrock-models-plan', focus: 'aws_bedrock_custom_model.derived', target: 16, mask: {vpc_config: true}},
  {fixture: 'backup-restore-plan', focus: 'aws_backup_restore_testing_selection.main', target: 3, mask: {recovery_point_selection: true}},
  {fixture: 'backup-report-plan', focus: 'aws_backup_report_plan.main', target: 3, mask: {report_setting: true}},
];
for (const entry of cases) {
  const source = JSON.parse(fs.readFileSync(`tests/fixtures/${entry.fixture}.json`, 'utf8'));
  for (const scenario of ['complete', 'unknown', 'filtered', 'anonymized', 'named']) {
    const plan = structuredClone(source);
    if (scenario === 'unknown') plan.resource_changes[entry.target].change.after_unknown = entry.mask;
    const input = JSON.stringify(plan);
    for (const format of ['svg', 'html']) {
      const options = {format, icon_mode: 'service-name'};
      const args = ['--format', format, '--icon-mode', 'service-name'];
      if (scenario === 'filtered') { options.focus = entry.focus; options.focus_depth = 0; args.push('--focus', entry.focus, '--focus-depth', '0'); }
      if (scenario === 'anonymized') { options.anonymize = true; args.push('--anonymize'); }
      let result, expected;
      if (scenario === 'named') {
        const temp = fs.mkdtempSync(join(tmpdir(), 'planorama-service-audit-'));
        try {
          const file = join(temp, 'plan.json'); fs.writeFileSync(file, input);
          result = JSON.parse(binding.generate_named(JSON.stringify(['first', 'second'].map(state => ({state, json: input}))), JSON.stringify(options)));
          expected = execFileSync(binary, ['--state', `first=${file}`, '--state', `second=${file}`, '-o', '-', ...args], {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
        } finally { fs.rmSync(temp, {recursive: true, force: true}); }
      } else {
        result = JSON.parse(binding.generate(input, JSON.stringify(options)));
        expected = native(input, args);
      }
      assert.equal(result.content, expected, `${entry.fixture}/${scenario}/${format}`);
      assert.ok(!result.content.includes('PRIVATE_'), `${entry.fixture}: private values omitted`);
    }
  }
  console.log(`${entry.fixture}: native/Wasm SVG/HTML, filtered, anonymized and named-state parity passed`);
}
