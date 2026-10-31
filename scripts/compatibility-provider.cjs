const assert = require('node:assert/strict');

const ipam = ['ipv4_ipam_pool_id', 'ipv4_netmask_length', 'ipv6_ipam_pool_id', 'ipv6_netmask_length'];
function remove(object, key, expected) {
  assert.deepEqual(object[key], expected, key);
  delete object[key];
}
function values(type, object) {
  if (type === 'aws_subnet') {
    for (const key of ipam) remove(object, key, null);
    assert.equal(object.ipv6_cidr_block, undefined);
    object.ipv6_cidr_block = null;
  }
  if (type === 'aws_lb') remove(object, 'health_check_logs', []);
}
function normalizeProvider(plan) {
  assert.equal(plan.configuration.provider_config.aws.version_constraint, '6.70.0');
  plan.configuration.provider_config.aws.version_constraint = '6.0.0';
  function module(value) {
    for (const resource of value.resources || []) {
      values(resource.type, resource.values);
      if (resource.type === 'aws_lb') remove(resource.sensitive_values, 'health_check_logs', []);
    }
    for (const child of value.child_modules || []) module(child);
  }
  module(plan.planned_values.root_module);
  for (const resource of plan.resource_changes) {
    values(resource.type, resource.change.after);
    const unknown = resource.change.after_unknown;
    if (resource.type === 'aws_subnet') remove(unknown, 'ipv6_cidr_block', true);
    if (resource.type === 'aws_cloudwatch_log_group') remove(unknown, 'deletion_protection_enabled', true);
    if (resource.type === 'aws_lb') {
      remove(unknown, 'enable_prefix_for_ipv6_source_nat', true);
      remove(unknown, 'secondary_ips_auto_assigned_per_subnet', true);
      remove(unknown, 'health_check_logs', []);
      remove(resource.change.after_sensitive, 'health_check_logs', []);
    }
  }
}

function normalizeProviderOutput(content) {
  return content.replace(/(<script id="[^"]+" type="application\/json">)(\[[^\n]+)(<\/script>)/g,
    (match, start, json, end) => {
      const nodes = JSON.parse(json);
      for (const node of nodes) {
        if (!node.changed_paths) continue;
        const paths = node.changed_paths;
        function removePath(list, key) {
          const value = `$["${key}"]`;
          assert(paths[list].includes(value), `${node.address}: ${value}`);
          paths[list] = paths[list].filter(path => path !== value);
        }
        if (node.type === 'aws_subnet') {
          for (const key of ipam) removePath('paths', key);
          removePath('unknown_paths', 'ipv6_cidr_block');
          paths.paths.push('$["ipv6_cidr_block"]');
          paths.paths.sort();
        }
        if (node.type === 'aws_lb') {
          removePath('paths', 'health_check_logs');
          removePath('unknown_paths', 'enable_prefix_for_ipv6_source_nat');
          removePath('unknown_paths', 'secondary_ips_auto_assigned_per_subnet');
        }
        if (node.type === 'aws_cloudwatch_log_group') removePath('unknown_paths', 'deletion_protection_enabled');
      }
      return start + JSON.stringify(nodes) + end;
    });
}

module.exports = { normalizeProvider, normalizeProviderOutput };
