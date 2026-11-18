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
  function verify(input, expected, excluded = [], checkRows) {
    const json = JSON.stringify(input);
    for (const format of ['svg', 'html']) {
      const result = JSON.parse(binding.generate(json, JSON.stringify({format}))).content;
      assert.equal(result, native(json, ['--format', format]));
      if (checkRows) {
        const metadata = result.match(/<metadata id="resource-references">([\s\S]*?)<\/metadata>/)[1];
        checkRows(JSON.parse(metadata.replaceAll('&quot;', '"').replaceAll('&amp;', '&')));
      }
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
  for (const source of ['cloudflare_worker_version', 'cloudflare_workers_script']) {
    const input = plan();
    resource(input, 'cloudflare_worker', 'owner', {name: 'owner'});
    resource(input, 'cloudflare_workflow', 'flow', {
      workflow_name: 'flow', class_name: 'PRIVATE_Flow', script_name: 'owner',
    }, {script_name: ['cloudflare_worker.owner.name']});
    input.resource_changes[1].change.after_unknown = {id: true, name: true};
    const row = {type: 'workflow', name: 'FLOW', workflow_name: 'flow', class_name: 'PRIVATE_Flow'};
    resource(input, source, 'caller', {bindings: [row]}, {bindings: ['cloudflare_workflow.flow.workflow_name']});
    verify(input, ['Workflow script', 'Workflow bindings']);
    const flow = input.resource_changes[1].change;
    flow.after_unknown = {};
    Object.assign(flow.after, {id: 'flow', name: 'flow'});
    const refs = input.configuration.root_module.resources[2].expressions.bindings;
    refs.references = ['cloudflare_workflow.flow.id'];
    verify(input, ['Workflow bindings']);
    flow.after_sensitive = {name: true};
    verify(input, [], ['Workflow bindings']);
    flow.after_sensitive = {};
    row.class_name = 'PRIVATE_other';
    verify(input, ['Workflow script'], ['Workflow bindings']);
  }
  const events = plan();
  const queueId = 'c'.repeat(32);
  resource(events, 'cloudflare_queue', 'events', {queue_name: 'events', id: queueId, queue_id: queueId});
  resource(events, 'cloudflare_r2_bucket', 'files', {name: 'files', id: 'files', jurisdiction: 'eu'});
  resource(events, 'cloudflare_worker', 'consumer', {name: 'consumer'});
  resource(events, 'cloudflare_queue_consumer', 'worker', {
    type: 'worker', queue_id: queueId, script_name: 'consumer',
  }, {queue_id: ['cloudflare_queue.events.id'], script_name: ['cloudflare_worker.consumer.name']});
  resource(events, 'cloudflare_r2_bucket_event_notification', 'notify', {
    bucket_name: 'files', jurisdiction: 'eu', queue_id: queueId,
    rules: [{actions: ['PutObject'], prefix: 'PRIVATE_filter'}],
  }, {bucket_name: ['cloudflare_r2_bucket.files.name'], queue_id: ['cloudflare_queue.events.queue_id']});
  function notificationTargets(rows, attribute) {
    const consumer = 'terraform:' + Buffer.from('cloudflare_r2_bucket_event_notification.notify').toString('hex') + '-current';
    const row = rows.find(r => !r.dependency && r.consumer === consumer && r.attribute === attribute);
    assert.equal(row.complete, row.targets.length > 0);
    return row.targets.map(target => target.address);
  }
  verify(events, ['Notification bucket', 'Notification Queue', 'Consumed Queue', 'Consumer Worker'], [], rows => {
    assert.deepEqual(notificationTargets(rows, 'bucket_name'), ['cloudflare_r2_bucket.files']);
    assert.deepEqual(notificationTargets(rows, 'queue_id'), ['cloudflare_queue.events']);
  });
  const notification = events.resource_changes[4].change;
  notification.after.jurisdiction = 'default';
  notification.after_sensitive = {queue_id: true};
  verify(events, ['cloudflare_identity_mismatch', 'cloudflare_sensitive_value'], [], rows => {
    assert.deepEqual(notificationTargets(rows, 'bucket_name'), []);
    assert.deepEqual(notificationTargets(rows, 'queue_id'), []);
  });
  notification.after_sensitive = {};
  notification.after.jurisdiction = 'eu';
  events.configuration.root_module.resources[4].expressions.queue_id.references = ['cloudflare_queue.events.queue_name'];
  verify(events, ['cloudflare_output_mismatch']);
  const ai = plan();
  resource(ai, 'cloudflare_ai_gateway', 'gateway', {id: 'example'});
  resource(ai, 'cloudflare_ai_gateway_dynamic_routing', 'route', {
    gateway_id: 'example', name: 'my-route', elements: [{id: 'PRIVATE_model', type: 'model'}],
  }, {gateway_id: ['cloudflare_ai_gateway.gateway.id']});
  resource(ai, 'cloudflare_worker_version', 'caller', {
    bindings: [{name: 'AI', type: 'ai'}], main_script_base64: 'PRIVATE_payload',
  });
  verify(ai, ['Configured AI Gateway'], [], rows => {
    const gateway = rows.find(row => row.attribute === 'gateway_id' && !row.dependency);
    assert.ok(gateway.complete);
    assert.deepEqual(gateway.targets.map(target => target.address), ['cloudflare_ai_gateway.gateway']);
    assert.ok(!rows.some(row => row.attribute === 'bindings' && row.complete));
  });
  const ambiguous = structuredClone(ai);
  resource(ambiguous, 'cloudflare_ai_gateway', 'duplicate', {id: 'example'});
  verify(ambiguous, ['cloudflare_ambiguous_endpoint']);
  const hidden = structuredClone(ai);
  hidden.resource_changes[0].change.after_sensitive = {id: true};
  verify(hidden, ['cloudflare_sensitive_value']);
  const bindingRef = structuredClone(ai);
  bindingRef.configuration.root_module.resources[2].expressions.bindings = {
    references: ['cloudflare_ai_gateway.gateway.id'],
  };
  verify(bindingRef, ['cloudflare_unsupported_contract'], [], rows => {
    const bindingRow = rows.find(row => row.attribute === 'bindings' && !row.dependency);
    assert.ok(!bindingRow.complete);
    assert.deepEqual(bindingRow.targets, []);
  });
  ai.resource_changes[0].change.after.account_id = 'b'.repeat(32);
  verify(ai, ['cloudflare_scope_mismatch'], [], rows => {
    assert.deepEqual(rows.find(row => row.attribute === 'gateway_id' && !row.dependency).targets, []);
  });
  delete ai.configuration.root_module.resources[1].expressions.gateway_id;
  verify(ai, [], ['Configured AI Gateway']);
  require('./cloudflare-platform-contracts.cjs')({plan, resource, verify, account});
};
