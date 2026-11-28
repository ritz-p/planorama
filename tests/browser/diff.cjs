const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(fs.readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module:fs.readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib:512});
const plan = subnet => ({format_version:'1.2', resource_changes:[
  {address:'aws_instance.PRIVATE', type:'aws_instance'},
  {address:'aws_subnet.a', type:'aws_subnet'},
  {address:'aws_subnet.b', type:'aws_subnet'},
], configuration:{root_module:{resources:[{address:'aws_instance.PRIVATE', expressions:{subnet_id:{references:[`aws_subnet.${subnet}.id`]}}}]}}});
const before = plan('a');
const relocated = plan('b');
const moved = plan('a');
moved.resource_changes[0].previous_address = 'aws_instance.PRIVATE';
moved.resource_changes[0].address = 'aws_instance.renamed';
moved.configuration.root_module.resources[0].address = 'aws_instance.renamed';
const ambiguous = structuredClone(moved);
ambiguous.resource_changes.push(before.resource_changes[0]);
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-diff-'));
try {
  const a = path.join(temp, 'a.json');
  const b = path.join(temp, 'b.json');
  fs.writeFileSync(a, JSON.stringify(before));
  for (const after of [before, relocated, moved, ambiguous]) {
    fs.writeFileSync(b, JSON.stringify(after));
    for (const anonymous of [false, true]) {
      const expected = JSON.parse(execFileSync(path.join(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'), ['diff', a, b, ...(anonymous ? ['--anonymize'] : [])]));
      const actual = JSON.parse(binding.compare_plans(JSON.stringify(before), JSON.stringify(after), anonymous));
      assert.deepEqual(actual, expected);
      if (anonymous) {
        assert(!JSON.stringify(actual).includes('PRIVATE'));
        if (after === moved) {
          const [change] = actual.resources.changed;
          assert.equal(change.before, change.after);
        }
        if (after === ambiguous) assert(actual.resources.unknown.length > 0);
        if (after === relocated) {
          const added = actual.relationships.added.find(edge => edge.kind === 'containment');
          const removed = actual.relationships.removed.find(edge => edge.kind === 'containment');
          assert(added && removed);
          assert(added.from === removed.from || added.to === removed.to);
        }
      }
    }
  }
  const paddedPlan = length => JSON.stringify({
    format_version: '1.2',
    resource_changes: [{address: 'aws_instance.app', type: 'aws_instance'}],
    padding: Array(length).fill(0),
  });
  const halfBudget = 32_768 / 2;
  const unpaddedUnits = 12;
  const small = paddedPlan(0);
  const half = paddedPlan(halfBudget - unpaddedUnits);
  const halfPlusOne = paddedPlan(halfBudget - unpaddedUnits + 1);
  const unchanged = binding.compare_plans(small, small, true);
  assert.equal(binding.compare_plans(half, small, true), unchanged);
  assert.equal(binding.compare_plans(small, halfPlusOne, true), unchanged);
  assert.equal(binding.compare_plans(half, half, true), unchanged);
  assert.throws(() => binding.compare_plans(half, halfPlusOne, true),
    error => JSON.parse(String(error)).code === 'resource_limit');
  for (const side of [0, 1]) {
    for (const [input, code] of [['PRIVATE{', 'invalid_json'], [' '.repeat(2 * 1024 * 1024 + 1), 'resource_limit'], [JSON.stringify({extra:Array(32768).fill(0)}), 'resource_limit']]) {
      const inputs = [JSON.stringify(before), JSON.stringify(before)];
      inputs[side] = input;
      assert.throws(() => binding.compare_plans(...inputs, true), error => {
        assert.equal(JSON.parse(String(error)).code, code);
        assert(!String(error).includes('PRIVATE'));
        return true;
      });
    }
  }
} finally {
  fs.rmSync(temp, {recursive:true, force:true});
}
console.log('CLI/Wasm diff parity, anonymization and limits passed');
