const assert = require('node:assert/strict');
const fs = require('node:fs');
const {resolve, join} = require('node:path');
const {tmpdir} = require('node:os');
const {execFileSync} = require('node:child_process');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(fs.readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: fs.readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const binary = resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
const native = require('../support/native-reference.cjs')(binary);
const cases = [
  {"fixture":"emr-serverless-plan","focus":"aws_emrserverless_application.main","target":7,"mask":{"network_configuration":true}},
  {"fixture":"emr-containers-plan","focus":"aws_emrcontainers_virtual_cluster.main","target":5,"mask":{"container_provider":true}},
  {"fixture":"emr-studio-plan","focus":"aws_emr_studio.main","target":8,"mask":{"user_role":true}},
  {"fixture":"dsql-plan","focus":"aws_dsql_cluster.local","target":1,"mask":{"multi_region_properties":true}},
  {"fixture":"emr-cluster-plan","focus":"aws_emr_cluster.main","target":11,"mask":{"ec2_attributes":true}},
  {fixture: "analytics-databases-plan", focus: "aws_neptunegraph_private_graph_endpoint.main", target: 7, mask: {subnet_ids: true}},
  {"fixture":"redshift-serverless-helpers-plan","focus":"aws_redshiftserverless_namespace.main","target":5,"mask":{"iam_roles":true}},
  {"fixture":"redshift-catalog-plan","focus":"aws_redshift_integration.table","target":10,"mask":{"target_arn":true}},
  {"fixture":"redshift-helpers-plan","focus":"aws_redshift_scheduled_action.resize_cluster","target":10,"mask":{"target_action":true}},
  {fixture: 'dms-serverless-plan', focus: 'aws_dms_replication_config.main', target: 9, mask: {compute_config: true}},
  {fixture: 'dms-migration-plan', focus: 'aws_dms_instance_profile.main', target: 10, mask: {vpc_security_group_ids: true}},
  {fixture: 'dms-events-plan', focus: 'aws_dms_replication_instance.first', target: 4, mask: {kerberos_authentication_settings: true}},
  {fixture: 'bedrock-agents-plan', focus: 'aws_bedrockagent_agent.main', target: 6, mask: {guardrail_configuration: true}},
  {fixture: 'bedrock-models-plan', focus: 'aws_bedrock_custom_model.derived', target: 16, mask: {vpc_config: true}},
  {fixture: 'backup-restore-plan', focus: 'aws_backup_restore_testing_selection.main', target: 3, mask: {recovery_point_selection: true}},
  {fixture: 'backup-report-plan', focus: 'aws_backup_report_plan.main', target: 3, mask: {report_setting: true}},
];
for (const entry of cases) {
  const source = JSON.parse(fs.readFileSync(`tests/fixtures/${entry.fixture}.json`, 'utf8'));
  for (const scenario of ['complete', 'unknown', 'filtered', 'anonymized', 'named']) {
    const plan = structuredClone(source);
    if (scenario === 'unknown') plan.resource_changes[entry.target].change.after_unknown = entry.mask;
    const input = JSON.stringify(plan);
    for (const format of ['svg', 'html']) {
      const options = {format, icon_mode: 'service-name'};
      const args = ['--format', format, '--icon-mode', 'service-name'];
      if (scenario === 'filtered') { options.focus = entry.focus; options.focus_depth = 0; args.push('--focus', entry.focus, '--focus-depth', '0'); }
      if (scenario === 'anonymized') { options.anonymize = true; args.push('--anonymize'); }
      let result, expected;
      if (scenario === 'named') {
        const temp = fs.mkdtempSync(join(tmpdir(), 'planorama-service-audit-'));
        try {
          const file = join(temp, 'plan.json'); fs.writeFileSync(file, input);
          result = JSON.parse(binding.generate_named(JSON.stringify(['first', 'second'].map(state => ({state, json: input}))), JSON.stringify(options)));
          expected = execFileSync(binary, ['--state', `first=${file}`, '--state', `second=${file}`, '-o', '-', ...args], {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
        } finally { fs.rmSync(temp, {recursive: true, force: true}); }
      } else {
        result = JSON.parse(binding.generate(input, JSON.stringify(options)));
        expected = native(input, args);
      }
      assert.equal(result.content, expected, `${entry.fixture}/${scenario}/${format}`);
      assert.ok(!result.content.includes('PRIVATE_'), `${entry.fixture}: private values omitted`);
    }
  }
  console.log(`${entry.fixture}: native/Wasm SVG/HTML, filtered, anonymized and named-state parity passed`);
}
