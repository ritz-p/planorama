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
  {
    const input = plan(), zone = 'f'.repeat(32);
    resource(input, 'cloudflare_authenticated_origin_pulls_hostname_certificate', 'tls', {zone_id: zone, id: 'cert', private_key: 'PRIVATE_PEM'});
    const config = [{hostname: 'PRIVATE_origin', enabled: false, cert_id: 'cert'}];
    resource(input, 'cloudflare_authenticated_origin_pulls', 'pulls', {zone_id: zone, config},
      {config: ['cloudflare_authenticated_origin_pulls_hostname_certificate.tls.id']});
    resource(input, 'cloudflare_api_shield_operation', 'api', {zone_id: zone, id: 'op', operation_id: 'op'});
    resource(input, 'cloudflare_api_shield_operation_schema_validation_settings', 'settings', {zone_id: zone, operation_id: 'op'},
      {operation_id: ['cloudflare_api_shield_operation.api.operation_id']});
    resource(input, 'cloudflare_mtls_certificate', 'client', {id: 'mtls'});
    resource(input, 'cloudflare_workers_script', 'caller', {bindings: [{type: 'mtls_certificate', certificate_id: 'mtls'}]},
      {bindings: ['cloudflare_mtls_certificate.client.id']});
    verify(input, ['mTLS certificate bindings'], [], rows => {
      assert.deepEqual(target(rows, 'cloudflare_authenticated_origin_pulls.pulls', 'config'), ['cloudflare_authenticated_origin_pulls_hostname_certificate.tls']);
      assert.deepEqual(target(rows, 'cloudflare_api_shield_operation_schema_validation_settings.settings', 'operation_id'), ['cloudflare_api_shield_operation.api']);
    });
    config[0].enabled = null;
    input.resource_changes[2].change.after.zone_id = 'e'.repeat(32);
    verify(input, ['cloudflare_unsupported_contract','cloudflare_scope_mismatch'], [], rows => {
      assert.deepEqual(target(rows, 'cloudflare_authenticated_origin_pulls.pulls', 'config'), []);
      assert.deepEqual(target(rows, 'cloudflare_api_shield_operation_schema_validation_settings.settings', 'operation_id'), []);
    });
  }
  {
    const input = plan();
    resource(input, 'cloudflare_account', 'team', {id: account});
    resource(input, 'cloudflare_turnstile_widget', 'widget', {sitekey: 'PRIVATE_key', secret: 'PRIVATE_secret',
      domains: ['PRIVATE_hostname'], name: 'widget'}, {account_id: ['cloudflare_account.team.id']});
    resource(input, 'cloudflare_pages_project', 'app', {name: 'app', subdomain: 'PRIVATE_hostname'});
    verify(input, ['Account administration'], [], rows => {
      assert.deepEqual(target(rows, 'cloudflare_turnstile_widget.widget', 'account_id'), ['cloudflare_account.team']);
      assert.ok(!rows.some(row => row.targets.some(t => t.address === 'cloudflare_turnstile_widget.widget')));
    });
    input.resource_changes[1].change.after_unknown = {account_id: true};
    verify(input, ['cloudflare_unknown_value'], [], rows => {
      assert.deepEqual(target(rows, 'cloudflare_turnstile_widget.widget', 'account_id'), []);
    });
  }
  {
    const input = plan();
    resource(input, 'cloudflare_worker', 'logger', {id: 'worker-id', name: 'logger'});
    for (const [kind, row] of [['cloudflare_worker',{name:'logger'}], ['cloudflare_workers_script',{service:'logger'}]]) {
      resource(input, kind, 'emitter', {name:'emitter', tail_consumers:[row]},
        {tail_consumers:['cloudflare_worker.logger.name']});
    }
    resource(input, 'cloudflare_account', 'team', {id:account});
    resource(input, 'cloudflare_logpush_job', 'logs', {destination_conf:'PRIVATE_url', ownership_challenge:'PRIVATE_token', dataset:'workers_trace_events'},
      {account_id:['cloudflare_account.team.id']});
    verify(input, ['Configured log tail Workers'], [], rows => {
      for (const type of ['cloudflare_worker','cloudflare_workers_script']) {
        assert.deepEqual(target(rows, type+'.emitter', 'tail_consumers'), ['cloudflare_worker.logger']);
      }
      assert.deepEqual(target(rows, 'cloudflare_logpush_job.logs','account_id'), ['cloudflare_account.team']);
    });
    input.resource_changes[2].change.after.tail_consumers[0].namespace = 'PRIVATE_space';
    input.resource_changes[4].change.after.zone_id = 'c'.repeat(32);
    verify(input, ['cloudflare_unsupported_contract','cloudflare_scope_unavailable'], [], rows => {
      assert.deepEqual(target(rows,'cloudflare_workers_script.emitter','tail_consumers'), []);
      assert.deepEqual(target(rows,'cloudflare_logpush_job.logs','account_id'), []);
    });
  }
};
