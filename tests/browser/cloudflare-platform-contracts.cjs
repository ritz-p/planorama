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
};
