const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { normalizeProviderOutput } = require('./compatibility-provider.cjs');
const { execFileSync } = require('node:child_process');

const root = path.resolve(__dirname, '..');
const matrix = require('../tests/fixtures/compatibility/matrix.json');
const selected = process.argv.slice(2);
for (const name of selected) assert(matrix.cases.some(item => item.name === name), `unknown case: ${name}`);
const requested = new Set(selected);
for (const item of matrix.cases) if (requested.has(item.name) && item.compare) requested.add(item.compare);
const output = path.join(root, 'output/compatibility-output');
fs.mkdirSync(output, { recursive: true });
const binary = process.env.PLANORAMA_BIN || path.join(root, 'target/debug/planorama');
const report = { revision: process.env.COMPATIBILITY_REVISION || execFileSync('git', ['-c', `safe.directory=${root}`, 'rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), results: [] };
let failed = false;
const baselines = new Map();
for (const item of matrix.cases) {
  if (selected.length && !requested.has(item.name)) continue;
  const key = item.compare || item.name;
  if (!baselines.has(key)) baselines.set(key, new Map());
  const baseline = baselines.get(key);
  for (const version of item.cores || Object.keys(matrix.terraform)) {
    for (const format of ['svg', 'html']) {
      const entry = { version, case: item.name, format };
      const started = Date.now();
      const destination = path.join(output, `${version}-${item.name}.${format}`);
      try {
        fs.rmSync(destination, { force: true });
        const input = path.join(root, 'tests/fixtures/compatibility', version, item.name, 'plan.json');
        entry.input_sha256 = crypto.createHash('sha256').update(fs.readFileSync(input)).digest('hex');
        execFileSync(binary, [input, '--format', format, '-o', destination], { cwd: root, timeout: 300000 });
        let content = fs.readFileSync(destination, 'utf8');
        if (item.provider === '6.70.0') content = normalizeProviderOutput(content);
        if (item.name === 'language') {
          const plan = JSON.parse(fs.readFileSync(input, 'utf8'));
          const broad = plan.relevant_attributes.filter(entry =>
            ['data.terraform_remote_state.after_ready', 'data.terraform_remote_state.existing'].includes(entry.resource));
          assert.equal(broad.length, 2);
          for (const entry of broad) assert.deepEqual(entry.attribute,
            broad[0].attribute.length === 1 ? ['outputs'] : ['outputs', 'network_id']);
          if (broad[0].attribute.length === 1) {
            assert.equal(content.split('[[&quot;outputs&quot;]]').length - 1, 4);
            content = content.replaceAll('[[&quot;outputs&quot;]]', '[[&quot;outputs&quot;,&quot;network_id&quot;]]');
          }
          content = content.replace(/(<script id="[^"]+" type="application\/json">)(\[[^\n]+)(<\/script>)/g,
            (match, start, json, end) => {
              const values = JSON.parse(json);
              for (const node of values) {
                if (node.type !== 'terraform_data' || !node.changed_paths) continue;
                const change = plan.resource_changes.find(change => change.address === node.address);
                if (Object.hasOwn(change.change.after, 'store')) {
                  assert.equal(change.change.after.store, null);
                  assert(node.changed_paths.paths.includes('$["store"]'));
                  node.changed_paths.paths = node.changed_paths.paths.filter(value => value !== '$["store"]');
                }
              }
              return start + JSON.stringify(values) + end;
            });
        }
        assert(content.includes('<svg'), 'missing diagram');
        if (!baseline.has(format)) baseline.set(format, content);
        assert.equal(content, baseline.get(format), `${version}/${item.name}/${format}: output differs`);
        entry.status = 'passed';
      } catch (error) {
        entry.status = 'failed';
        entry.error = error.message.slice(0, 4000);
        failed = true;
      }
      entry.duration_ms = Date.now() - started;
      report.results.push(entry);
      fs.writeFileSync(path.join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
      console.log(`${entry.status}: ${version}/${item.name}/${format}`);
    }
  }
}
if (failed) process.exitCode = 1;
