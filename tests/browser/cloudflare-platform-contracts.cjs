const assert = require('node:assert/strict');

module.exports = function platformContracts({plan, resource, verify, account}) {
  function target(rows, address, field) {
    const consumer = 'terraform:' + Buffer.from(address).toString('hex') + '-current';
    const row = rows.find(r => r.consumer === consumer && !r.dependency && r.attribute === field);
    return row?.complete ? row.targets.map(t => t.address) : [];
  }
  {
    const input = plan();
    resource(input, 'cloudflare_ai_search_namespace', 'space', {name: 'space'});
    resource(input, 'cloudflare_ai_search_token', 'auth', {id: 'token-id', cf_api_key: 'PRIVATE_key'});
    resource(input, 'cloudflare_ai_gateway', 'ai', {id: 'gateway'});
    resource(input, 'cloudflare_r2_bucket', 'files', {name: 'files', id: 'files', jurisdiction: 'eu'});
    resource(input, 'cloudflare_ai_search_instance', 'search', {
      id: 'index', namespace: 'space', type: 'r2', source: 'files', token_id: 'token-id',
      source_params: {r2_jurisdiction: 'eu'}, ai_gateway_id: 'gateway',
    }, {source: ['cloudflare_r2_bucket.files.name'], token_id: ['cloudflare_ai_search_token.auth.id'],
      ai_gateway_id: ['cloudflare_ai_gateway.ai.id']});
    const bindings = [{type: 'ai_search', instance_name: 'index', namespace: 'space'},
      {type: 'ai_search_namespace', namespace: 'space'}];
    resource(input, 'cloudflare_worker_version', 'caller', {bindings}, {bindings: [
      'cloudflare_ai_search_instance.search.id', 'cloudflare_ai_search_namespace.space.name']});
    verify(input, ['AI Search instance bindings', 'AI Search namespace bindings'], [], rows => {
      for (const [field, address] of [['source','cloudflare_r2_bucket.files'],
        ['token_id','cloudflare_ai_search_token.auth'], ['ai_gateway_id','cloudflare_ai_gateway.ai']]) {
        assert.deepEqual(target(rows, 'cloudflare_ai_search_instance.search', field), [address]);
      }
    });
    bindings[0].namespace = 'another';
    verify(input, [], ['AI Search instance bindings', 'AI Search namespace bindings']);
  }
  {
    const input = plan();
    resource(input, 'cloudflare_ai_search_namespace', 'space', {name: 'tenant'});
    resource(input, 'cloudflare_ai_search_instance', 'instance', {id: 'search', namespace: 'tenant'},
      {namespace: ['cloudflare_ai_search_namespace.space.name']});
    input.configuration.provider_config['cloudflare.search'] = {
      name: 'cloudflare', full_name: 'registry.terraform.io/cloudflare/cloudflare', alias: 'search',
      version_constraint: '5.29.0',
    };
    input.configuration.root_module.resources[1].provider_config_key = 'cloudflare.search';
    const endpoint = 'cloudflare_ai_search_namespace.space';
    const check = expected => rows => assert.deepEqual(target(rows, 'cloudflare_ai_search_instance.instance', 'namespace'), expected);
    verify(input, ['Instance namespace'], [], check([endpoint]));
    for (const constraint of ['5.28.1', '>= 5.29.0', undefined]) {
      input.configuration.provider_config['cloudflare.search'].version_constraint = constraint;
      verify(input, [], [], check([]));
    }
    input.configuration.provider_config['cloudflare.search'].version_constraint = '5.29.0';
    input.resource_changes[1].change.after_sensitive = {namespace: true};
    verify(input, ['cloudflare_sensitive_value'], [], check([]));
  }
  {
    const input = plan();
    resource(input, 'cloudflare_pipeline_stream', 'events', {id: 'stream-id', name: 'events'});
    resource(input, 'cloudflare_workers_script', 'writer', {bindings: [{type: 'pipelines', pipeline: 'stream-id'}]},
      {bindings: ['cloudflare_pipeline_stream.events.id']});
    resource(input, 'cloudflare_r2_bucket', 'files', {id: 'files', name: 'files'});
    const config = {account_id: account, bucket: 'files', credentials: {secret_access_key: 'PRIVATE_secret'}};
    resource(input, 'cloudflare_pipeline_sink', 'sink', {type: 'r2', config}, {config: ['cloudflare_r2_bucket.files.name']});
    resource(input, 'cloudflare_r2_data_catalog', 'catalog', {bucket_name: 'files'}, {bucket_name: ['cloudflare_r2_bucket.files.name']});
    verify(input, ['Pipeline stream bindings'], [], rows => {
      for (const [node, field] of [['cloudflare_pipeline_sink.sink','config'], ['cloudflare_r2_data_catalog.catalog','bucket_name']]) {
        assert.deepEqual(target(rows, node, field), ['cloudflare_r2_bucket.files']);
      }
    });
    config.account_id = 'b'.repeat(32);
    input.resource_changes[0].change.after_unknown = {id: true};
    verify(input, ['cloudflare_scope_mismatch'], ['Pipeline stream bindings'], rows => {
      assert.deepEqual(target(rows, 'cloudflare_pipeline_sink.sink', 'config'), []);
    });
  }
};
