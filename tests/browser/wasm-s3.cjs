const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {join} = require('node:path');

module.exports = ({root, run, native}) => {
  for (const kind of ['aws_s3_bucket_versioning', 'aws_s3_bucket_metric', 'aws_s3_bucket_inventory', 'aws_s3_bucket_logging']) {
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
