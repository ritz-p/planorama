const {readFileSync, writeFileSync} = require('node:fs');
const {resolve} = require('node:path');
const arn = (service, id) => `arn:aws:${service}:us-east-1:111111111111:${id}`;
const key = arn('kms', 'key/example');
const role = 'arn:aws:iam::111111111111:role/dms';
const secret = arn('secretsmanager', 'secret:database-abcdef');
const certificate = arn('dms', 'cert:example');
const stream = arn('kinesis', 'stream/changes');
const ref = output => ({references: [output]});
const plan = {
  format_version: '1.2',
  terraform_version: '1.9.8',
  complete: true,
  resource_changes: [],
  configuration: {
    provider_config: {aws: {full_name: 'registry.terraform.io/hashicorp/aws', expressions: {region: {constant_value: 'us-east-1'}}}},
    root_module: {resources: []},
  },
};
function add(type, name, after, expressions = {}) {
  const address = `${type}.${name}`;
  plan.resource_changes.push({address, mode: 'managed', type, provider_name: 'registry.terraform.io/hashicorp/aws', change: {actions: ['create'], after}});
  plan.configuration.root_module.resources.push({address, mode: 'managed', type, name, provider_config_key: 'aws', expressions});
}
add('aws_kms_key', 'key', {arn: key, key_id: 'example', id: 'example'});
add('aws_iam_role', 'role', {arn: role, name: 'dms'});
add('aws_secretsmanager_secret', 'credentials', {arn: secret, id: secret, name: 'database', description: 'PRIVATE_SECRET_DESCRIPTION'});
add('aws_dms_certificate', 'certificate', {certificate_arn: certificate, certificate_id: 'certificate', certificate_pem: 'PRIVATE_CERTIFICATE'});
add('aws_s3_bucket', 'storage', {bucket: 'migration-storage', id: 'migration-storage', arn: 'arn:aws:s3:::migration-storage'});
add('aws_kinesis_stream', 'stream', {arn: stream, name: 'changes'});
add('aws_dms_endpoint', 'source', {
  endpoint_id: 'source', endpoint_arn: arn('dms', 'endpoint:source'), endpoint_type: 'source', engine_name: 'mysql',
  certificate_arn: certificate, kms_key_arn: key, secrets_manager_arn: secret, secrets_manager_access_role_arn: role,
  extra_connection_attributes: 'PRIVATE_CONNECTION',
}, {
  certificate_arn: ref('aws_dms_certificate.certificate.certificate_arn'),
  kms_key_arn: ref('aws_kms_key.key.arn'), secrets_manager_arn: ref('aws_secretsmanager_secret.credentials.arn'),
  secrets_manager_access_role_arn: ref('aws_iam_role.role.arn'),
});
add('aws_dms_endpoint', 'stream', {
  endpoint_id: 'stream', endpoint_arn: arn('dms', 'endpoint:stream'), endpoint_type: 'target', engine_name: 'kinesis',
  kinesis_settings: [{stream_arn: stream, service_access_role_arn: role}],
}, {kinesis_settings: [{stream_arn: ref('aws_kinesis_stream.stream.arn'), service_access_role_arn: ref('aws_iam_role.role.arn')}]});
add('aws_dms_endpoint', 'warehouse', {
  endpoint_id: 'warehouse', endpoint_arn: arn('dms', 'endpoint:warehouse'), endpoint_type: 'target', engine_name: 'redshift',
  redshift_settings: [{bucket_name: 'migration-storage', service_access_role_arn: role, encryption_mode: 'SSE_KMS', server_side_encryption_kms_key_id: 'example'}],
}, {redshift_settings: [{bucket_name: ref('aws_s3_bucket.storage.bucket'), service_access_role_arn: ref('aws_iam_role.role.arn'), server_side_encryption_kms_key_id: ref('aws_kms_key.key.key_id')}]});
add('aws_dms_s3_endpoint', 'storage', {
  endpoint_id: 'storage', endpoint_arn: arn('dms', 'endpoint:storage'), endpoint_type: 'target',
  bucket_name: 'migration-storage', service_access_role_arn: role, encryption_mode: 'SSE_KMS',
  server_side_encryption_kms_key_id: key, kms_key_arn: key, certificate_arn: certificate,
  external_table_definition: 'PRIVATE_TABLE',
}, {
  bucket_name: ref('aws_s3_bucket.storage.bucket'), service_access_role_arn: ref('aws_iam_role.role.arn'),
  server_side_encryption_kms_key_id: ref('aws_kms_key.key.arn'), kms_key_arn: ref('aws_kms_key.key.arn'),
  certificate_arn: ref('aws_dms_certificate.certificate.certificate_arn'),
});
add('aws_dms_endpoint', 'dynamodb', {
  endpoint_id: 'dynamodb', endpoint_arn: arn('dms', 'endpoint:dynamodb'), endpoint_type: 'target', engine_name: 'dynamodb', service_access_role: role,
}, {service_access_role: ref('aws_iam_role.role.arn')});
add('aws_dms_replication_task', 'migration', {
  replication_task_id: 'migration', source_endpoint_arn: arn('dms', 'endpoint:source'), target_endpoint_arn: arn('dms', 'endpoint:storage'),
  table_mappings: 'PRIVATE_MAPPINGS',
}, {source_endpoint_arn: ref('aws_dms_endpoint.source.endpoint_arn'), target_endpoint_arn: ref('aws_dms_s3_endpoint.storage.endpoint_arn')});

const path = resolve(__dirname, '../tests/fixtures/dms-endpoints-plan.json');
const text = JSON.stringify(plan, null, 2) + '\n';
if (process.argv.includes('--check')) {
  if (readFileSync(path, 'utf8') !== text) throw new Error('DMS endpoint fixture is stale');
} else {
  writeFileSync(path, text);
}
