'use strict';

const fs = require('node:fs');
const assert = require('node:assert/strict');
const plan = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const changes = [];

function mapFields(object, transform) {
  return Object.fromEntries(Object.entries(object || {}).map(([key, value]) => [key, transform(value, key)]));
}

function expression(value) {
  if (Array.isArray(value)) return value.map(expression);
  if (value.references?.length) {
    const refs = value.references.filter(ref => !value.references.some(longer =>
      longer.startsWith(ref + '.') || longer.startsWith(ref + '[')));
    return {references: [...new Set(refs)].sort()};
  }
  if ('constant_value' in value) return {constant_value: value.constant_value};
  return mapFields(value, expression);
}

function constantTree(value) {
  if (Array.isArray(value)) return value.map(constantTree);
  if ('constant_value' in value) return value.constant_value;
  assert.ok(!('references' in value), 'Expected a constant filter');
  return mapFields(value, constantTree);
}

function equalityValue(value, scope, output) {
  if (Array.isArray(value)) return value.map(item => equalityValue(item, scope, output));
  if (value.references) {
    assert.equal(value.references.length, 1);
    const [ref] = value.references;
    assert.ok(ref.endsWith('.' + output), 'Expected exact ' + output + ' reference');
    return syntheticId(scope + ref.slice(0, -output.length - 1), output);
  }
  if ('constant_value' in value) return value.constant_value;
  return mapFields(value, field => equalityValue(field, scope, output));
}

function syntheticId(address, output) {
  return output === 'id' ? 'fixture-id:' + address :
    'arn:aws:elasticloadbalancing:us-east-1:000000000000:fixture/' + address;
}

function addChange(instance, resource, scope) {
  const {address, type, mode, name} = instance;
  const change = {actions: [mode === 'managed' ? 'create' : 'no-op']};
  if (['aws_lb', 'aws_lb_listener', 'aws_lb_listener_rule', 'aws_lb_target_group'].includes(type)) {
    change.after = {arn: syntheticId(address, 'arn')};
    for (const key of ['load_balancer_arn', 'listener_arn', 'default_action', 'action']) {
      if (resource.expressions[key]) change.after[key] = equalityValue(resource.expressions[key], scope, 'arn');
    }
  }
  if (['aws_route_table', 'aws_internet_gateway', 'aws_nat_gateway'].includes(type)) {
    change.after = {id: syntheticId(address, 'id')};
  }
  if (type === 'aws_route') {
    change.after = {};
    for (const key of ['route_table_id', 'gateway_id', 'nat_gateway_id']) {
      if (resource.expressions[key]) change.after[key] = equalityValue(resource.expressions[key], scope, 'id');
    }
  }
  changes.push({address, type, mode, name, change});
}

function readData(module) {
  return [...(module?.resources || []).filter(r => r.mode === 'data'),
    ...(module?.child_modules || []).flatMap(readData)];
}
const instances = [...plan.resource_changes.filter(r => r.mode === 'managed'), ...readData(plan.prior_state?.values.root_module)];
function projectModule(input, scope) {
  const resources = [...input.resources].sort((a, b) => a.address < b.address ? -1 : Number(a.address > b.address)).map(raw => {
    const {address, mode, type, name} = raw;
    assert.ok(!raw.for_each_expression, "Stress fixture does not model resource for_each");
    const expressions = mapFields(raw.expressions, expression);
    if (type === 'aws_ami' && expressions.filter) expressions.filter = {constant_value: constantTree(expressions.filter)};
    const resource = {address, mode, type, name, expressions};
    if (raw.count_expression) resource.count_expression = expression(raw.count_expression);
    if (raw.depends_on) resource.depends_on = raw.depends_on;
    for (const instance of instances.filter(r => r.address === scope + address || r.address.startsWith(scope + address + '['))) {
      addChange(instance, resource, scope);
    }
    return resource;
  });
  const calls = {};
  for (const name of Object.keys(input.module_calls || {}).sort()) {
    const call = input.module_calls[name];
    for (const key of ["count_expression", "for_each_expression", "depends_on", "providers"]) {
      assert.ok(!(key in call), "Unsupported stress module field: " + key);
    }
    calls[name] = {expressions: mapFields(call.expressions, expression), module: projectModule(call.module, scope + 'module.' + name + '.')};
  }
  return {resources, module_calls: calls, outputs: mapFields(input.outputs, o => ({expression: expression(o.expression)}))};
}

function sorted(value) {
  if (Array.isArray(value)) return value.map(sorted);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map(k => [k, sorted(value[k])]));
  return value;
}
assert.equal(plan.terraform_version, '1.9.8');
assert.ok(plan.complete && !plan.errored);
assert.ok(plan.resource_changes.every(r => r.mode === 'managed' && r.change.actions.join() === 'create'));
const root_module = projectModule(plan.configuration.root_module, '');
assert.deepEqual(changes.map(r => r.address).sort(), instances.map(r => r.address).sort());
const stress = {format_version: plan.format_version, terraform_version: plan.terraform_version,
  resource_changes: changes, configuration: {root_module}};
process.stdout.write(JSON.stringify(sorted(stress), null, 2) + '\n');
