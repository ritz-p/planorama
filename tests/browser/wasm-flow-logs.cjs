const assert = require('node:assert/strict');

module.exports = function({run, native}) {
  const kinds = ['aws_vpc', 'aws_cloudwatch_log_group', 'aws_iam_role', 'aws_flow_log'];
  const addresses = kinds.map(kind => `${kind}.main`);
  const values = [
    {id: 'vpc-example'},
    {arn: 'arn:aws:logs:us-east-1:123456789012:log-group:flow'},
    {arn: 'arn:aws:iam::123456789012:role/flow'},
    {vpc_id: 'vpc-example', log_destination_type: 'cloud-watch-logs', log_format: 'PRIVATE_PACKET_PAYLOAD'},
  ];
  values[3].log_destination = values[1].arn;
  values[3].iam_role_arn = values[2].arn;
  const input = {format_version: '1.2', resource_changes: kinds.map((type, i) => ({
    address: addresses[i], type, mode: 'managed', provider_name: 'registry.terraform.io/hashicorp/aws',
    change: {actions: ['create'], after: values[i]},
  })), configuration: {provider_config: {aws: {name: 'aws', full_name: 'registry.terraform.io/hashicorp/aws',
    expressions: {region: {constant_value: 'us-east-1'}}}}, root_module: {resources: kinds.map((type, i) => ({
      address: addresses[i], type, mode: 'managed', provider_config_key: 'aws', expressions: i === 3 ? {
        vpc_id: {references: [`${addresses[0]}.id`]}, log_destination: {references: [`${addresses[1]}.arn`]},
        iam_role_arn: {references: [`${addresses[2]}.arn`]},
      } : {},
    }))}}};
  for (const format of ['svg', 'html']) {
    const json = JSON.stringify(input);
    const result = run(json, {format});
    assert.equal(result.ok, true);
    assert.equal(result.content, native(json, ['--format', format]));
    assert.ok(result.content.includes('Flow logs'));
    assert.ok(!result.content.includes('PRIVATE_PACKET_PAYLOAD'));
  }
};
