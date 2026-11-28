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
  const input = JSON.stringify(before);
  const compare = (after, anonymous, checkCli = false) => {
    const next = JSON.stringify(after);
    const actual = JSON.parse(binding.compare_plans(input, next, anonymous));
    if (checkCli) {
      fs.writeFileSync(b, next);
      const args = ['diff', a, b, ...(anonymous ? ['--anonymize'] : [])];
      const cli = path.join(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
      assert.deepEqual(actual, JSON.parse(execFileSync(cli, args)));
    }
    if (anonymous) assert(!JSON.stringify(actual).includes('PRIVATE'));
    return actual;
  };
  for (const anonymous of [false, true]) {
    assert.deepEqual(compare(before, anonymous), {
      schema_version: 1, resources: {added: [], changed: [], removed: [], unknown: []},
      relationships: {added: [], removed: []},
    });
    const {changed} = compare(moved, anonymous, !anonymous).resources;
    assert.equal(changed.length, 1);
    if (anonymous) {
      assert(Number.isInteger(changed[0].before));
      assert.equal(changed[0].before, changed[0].after);
    } else {
      assert.equal(changed[0].before.address, 'aws_instance.PRIVATE');
      assert.equal(changed[0].after.address, 'aws_instance.renamed');
    }
    const {unknown} = compare(ambiguous, anonymous).resources;
    assert.equal(unknown.length, 2);
    if (anonymous) {
      assert(unknown.every(Number.isInteger));
      assert.equal(new Set(unknown).size, 2);
    } else {
      assert.deepEqual(unknown.map(key => key.address).sort(), ['aws_instance.PRIVATE', 'aws_instance.renamed']);
    }
    const {added, removed} = compare(relocated, anonymous, anonymous).relationships;
    assert.equal(added.length, 1);
    assert.equal(removed.length, 1);
    assert.equal(added[0].kind, 'containment');
    assert.equal(removed[0].kind, 'containment');
    assert.deepEqual(added[0].to, removed[0].to);
    assert.notDeepEqual(added[0].from, removed[0].from);
    if (anonymous) assert([added[0].from, removed[0].from, added[0].to].every(Number.isInteger));
    else assert.deepEqual([removed[0].from.address, added[0].from.address, added[0].to.address],
      ['aws_subnet.a', 'aws_subnet.b', 'aws_instance.PRIVATE']);
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
