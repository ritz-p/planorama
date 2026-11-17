const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {join} = require('node:path');

module.exports = ({root, run, native}) => {
  for (const scenario of ['valid', 'role', 'directory-user', 'directory-group', 'unknown', 'sensitive', 'account', 'wrong-output']) {
    const input = JSON.parse(readFileSync(join(root, 'tests/fixtures/s3-grants-plan.json'), 'utf8'));
    if (scenario === 'role') {
      input.resource_changes[3].change.after.grantee[0].grantee_identifier = input.resource_changes[0].change.after.arn;
      input.configuration.root_module.resources[3].expressions.grantee[0].grantee_identifier.references = ['aws_iam_role.location.arn'];
    }
    if (scenario === 'directory-user') input.resource_changes[3].change.after.grantee[0].grantee_type = 'DIRECTORY_USER';
    if (scenario === 'directory-group') input.resource_changes[3].change.after.grantee[0].grantee_type = 'DIRECTORY_GROUP';
    if (scenario === 'unknown') input.resource_changes[3].change.after_unknown = {grantee: true};
    if (scenario === 'sensitive') input.resource_changes[1].change.after_sensitive = {arn: true};
    if (scenario === 'account') input.resource_changes[3].change.after.account_id = '111111111111';
    if (scenario === 'wrong-output') input.configuration.root_module.resources[3].expressions.access_grants_location_id.references = ['aws_s3control_access_grants_location.main.id'];
    const json = JSON.stringify(input);
    for (const format of ['svg', 'html']) {
      const result = run(json, {format, icon_mode: 'service-name'});
      assert.equal(result.ok, true, scenario);
      assert.equal(result.content, native(json, ['--format', format]), `Access Grants ${scenario} ${format}`);
      assert.ok(!result.content.includes('private-location-sentinel'));
    }
  }
  for (const scenario of ['valid', 'same-provider', 'unknown', 'sensitive', 'foreign-account', 'missing-reference']) {
    const input = JSON.parse(readFileSync(join(root, 'tests/fixtures/s3-mrap-plan.json'), 'utf8'));
    if (scenario === 'same-provider') {
      input.configuration.root_module.resources[1].provider_config_key = 'aws';
      for (const row of input.resource_changes[2].change.after.details[0].region) delete row.bucket_account_id;
      delete input.resource_changes[4].change.after.account_id;
      input.resource_changes[4].change.after.route[1].region = 'us-east-1';
    }
    if (scenario === 'unknown') input.resource_changes[2].change.after_unknown = {details: true};
    if (scenario === 'sensitive') input.resource_changes[0].change.after_sensitive = {bucket: true};
    if (scenario === 'foreign-account') input.resource_changes[3].change.after.account_id = '222222222222';
    if (scenario === 'missing-reference') input.configuration.root_module.resources[2].expressions = {};
    const json = JSON.stringify(input);
    for (const format of ['svg', 'html']) {
      const result = run(json, {format, icon_mode: 'service-name'});
      assert.equal(result.ok, true, scenario);
      assert.equal(result.content, native(json, ['--format', format]), `MRAP ${scenario} ${format}`);
      assert.ok(!result.content.includes('private-policy-sentinel'));
    }
  }
  for (const kind of ['aws_s3_bucket_versioning', 'aws_s3_bucket_metric', 'aws_s3_bucket_inventory', 'aws_s3_bucket_logging', 'aws_s3_object', 'aws_s3_bucket_object', 'aws_s3_object_copy']) {
    const input = JSON.parse(readFileSync(join(root, 'tests/fixtures/s3-policy-plan.json'), 'utf8'));
    input.resource_changes[1].type = kind;
    if (kind === 'aws_s3_bucket_metric') input.resource_changes[0].type = 'aws_s3_directory_bucket';
    const json = JSON.stringify(input);
    for (const format of ['svg', 'html']) {
      const result = run(json, {format, icon_mode: 'service-name'});
      assert.equal(result.ok, true, kind);
      assert.equal(result.content, native(json, ['--format', format]), `${kind} ${format}`);
    }
  }
};
