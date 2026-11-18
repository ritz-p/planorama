const assert = require('node:assert/strict');

module.exports = function workerContracts(binding, native) {
  const provider = 'registry.terraform.io/cloudflare/cloudflare';
  const account = 'a'.repeat(32);
  function plan() {
    return {
      format_version: '1.2',
      terraform_version: '1.9.8',
      resource_changes: [],
      configuration: {
        provider_config: {cloudflare: {name: 'cloudflare', full_name: provider}},
        root_module: {resources: []},
      },
    };
  }
  function resource(input, type, name, values, references = {}) {
    const common = {address: type + '.' + name, mode: 'managed', type, name};
    input.resource_changes.push({...common, provider_name: provider, change: {
      actions: ['create'], before: null, after: {account_id: account, ...values},
      after_unknown: {}, after_sensitive: {},
    }});
    input.configuration.root_module.resources.push({...common, provider_config_key: 'cloudflare',
      expressions: Object.fromEntries(Object.entries(references).map(([field, refs]) =>
        [field, {references: refs}]))});
  }
  function verify(input, expected, excluded = []) {
    const json = JSON.stringify(input);
    for (const format of ['svg', 'html']) {
      const result = JSON.parse(binding.generate(json, JSON.stringify({format}))).content;
      assert.equal(result, native(json, ['--format', format]));
      for (const label of expected) assert.ok(result.includes(label), label);
      for (const label of [...excluded, 'PRIVATE_']) assert.ok(!result.includes(label), label);
    }
  }

  for (const source of ['cloudflare_worker_version', 'cloudflare_workers_script']) {
    const input = plan();
    resource(input, 'cloudflare_worker', 'objects', {name: 'objects', id: 'b'.repeat(32)});
    const row = {name: 'OBJECTS', type: 'durable_object_namespace', class_name: 'PRIVATE_Class',
      script_name: 'objects'};
    resource(input, source, 'caller', {bindings: [row]}, {bindings: ['cloudflare_worker.objects.name']});
    verify(input, ['Durable Object scripts'], ['Service bindings']);
    row.dispatch_namespace = 'PRIVATE_dispatch';
    verify(input, [], ['Durable Object scripts']);
    delete row.dispatch_namespace;
    input.resource_changes[1].change.after.bindings.push({type: 'service', service: 'objects'});
    verify(input, [], ['Durable Object scripts', 'Service bindings']);
  }
};
