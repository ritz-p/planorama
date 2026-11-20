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
  {"fixture":"lakeformation-selectors-plan","focus":"aws_lakeformation_permissions.sample4","target":4,"mask":{"table":true}},
  {"fixture":"glue-encryption-plan","focus":"aws_glue_data_catalog_encryption_settings.sample3","target":3,"mask":{"data_catalog_encryption_settings":true}},
  {"fixture":"config-aggregation-plan","focus":"aws_config_aggregate_authorization.sample0","target":0,"mask":{"authorized_aws_region":true}},
  {"fixture":"audit-manager-plan","focus":"aws_auditmanager_account_registration.sample0","target":0,"mask":{"kms_key":true}},
  {"fixture":"access-analyzer-plan","focus":"aws_accessanalyzer_archive_rule.sample0","target":0,"mask":{"analyzer_name":true}},
  {"fixture":"verified-access-plan","focus":"aws_verifiedaccess_endpoint.sample0","target":0,"mask":{"endpoint_type":true}},
  {"fixture":"transit-multicast-plan","focus":"aws_ec2_transit_gateway_multicast_domain.sample0","target":0,"mask":{"transit_gateway_id":true}},
  {"fixture":"transit-policies-plan","focus":"aws_ec2_transit_gateway_metering_policy.sample0","target":0,"mask":{"transit_gateway_id":true}},
  {"fixture":"transit-routes-plan","focus":"aws_ec2_transit_gateway_route.sample0","target":0,"mask":{"blackhole":true}},
  {"fixture":"transit-attachments-plan","focus":"aws_ec2_transit_gateway_vpc_attachment.sample0","target":0,"mask":{"transit_gateway_id":true}},
  {"fixture":"client-vpn-plan","focus":"aws_ec2_client_vpn_endpoint.sample0","target":0,"mask":{"authentication_options":true}},
  {"fixture":"local-gateways-plan","focus":"aws_ec2_carrier_gateway.sample0","target":0,"mask":{"vpc_id":true}},
  {"fixture":"site-vpn-plan","focus":"aws_vpn_connection.sample0","target":0,"mask":{"static_routes_only":true}},
  {"fixture":"template-launch-plan","focus":"aws_launch_template.sample0","target":0,"mask":{"block_device_mappings":true}},
  {"fixture":"instance-launch-plan","focus":"aws_instance.sample0","target":0,"mask":{"ami":true}},
  {"fixture":"ec2-fleets-plan","focus":"aws_ec2_fleet.sample0","target":0,"mask":{"launch_template_config":true}},
  {"fixture":"ec2-capacity-plan","focus":"aws_ec2_capacity_reservation.sample0","target":0,"mask":{"placement_group_arn":true}},
  {"fixture":"ebs-provenance-plan","focus":"aws_ebs_snapshot_copy.sample0","target":0,"mask":{"source_region":true}},
  {"fixture":"ec2-images-plan","focus":"aws_ami_copy.sample0","target":0,"mask":{"source_ami_region":true}},
  {"fixture":"finspace-kx-configuration-plan","focus":"aws_finspace_kx_environment.sample0","target":0,"mask":{"kms_key_id":true}},
  {"fixture":"ai-service-configuration-plan","focus":"aws_comprehend_document_classifier.sample0","target":0,"mask":{"data_access_role_arn":true}},
  {"fixture":"datapipeline-configuration-plan","focus":"aws_datapipeline_pipeline_definition.sample1","target":0,"mask":{"id":true}},
  {"fixture":"dataexchange-configuration-plan","focus":"aws_dataexchange_revision.sample2","target":0,"mask":{"asset_type":true}},
  {"fixture":"cleanrooms-configuration-plan","focus":"aws_cleanrooms_configured_table.sample1","target":0,"mask":{"arn":true}},
  {"fixture":"ecr-configuration-plan","focus":"aws_ecrpublic_repository_policy.sample1","target":0,"mask":{"repository_name":true}},
  {"fixture":"glacier-configuration-plan","focus":"aws_glacier_vault.sample0","target":0,"mask":{"notification":true}},
  {"fixture":"flink-configuration-plan","focus":"aws_kinesisanalyticsv2_application.sample0","target":0,"mask":{"runtime_environment":true}},
  {"fixture":"mwaa-configuration-plan","focus":"aws_mwaa_environment.sample0","target":0,"mask":{"network_configuration":true}},
  {"fixture":"transfer-workflow-configuration-plan","focus":"aws_transfer_workflow.sample0","target":0,"mask":{"steps":true}},
  {"fixture":"lightsail-configuration-plan","focus":"aws_lightsail_disk_attachment.sample9","target":0,"mask":{"name":true}},
  {"fixture":"recovery-configuration-plan","focus":"aws_dlm_lifecycle_policy.sample0","target":0,"mask":{"policy_details":true}},
  {"fixture":"batch-configuration-plan","focus":"aws_batch_compute_environment.sample0","target":0,"mask":{"type":true}},
  {"fixture":"signer-configuration-plan","focus":"aws_signer_signing_profile.sample0","target":0,"mask":{"name":true}},
  {"fixture":"paymentcryptography-configuration-plan","focus":"aws_paymentcryptography_key_alias.sample1","target":0,"mask":{"arn":true}},
  {"fixture":"cloudhsm-configuration-plan","focus":"aws_cloudhsm_v2_cluster.sample1","target":0,"mask":{"id":true}},
  {"fixture":"accountaccess-configuration-plan","focus":"aws_accountaccess_application.sample0","target":0,"mask":{"arn":true}},
  {"fixture":"verifiedpermissions-configuration-plan","focus":"aws_verifiedpermissions_policy.sample1","target":0,"mask":{"policy_store_id":true}},
  {"fixture":"network-insights-configuration-plan","focus":"aws_ec2_network_insights_path.sample1","target":0,"mask":{"id":true}},
  {"fixture":"appmesh-configuration-plan","focus":"aws_appmesh_route.sample1","target":0,"mask":{"name":true}},
  {"fixture":"keyspaces-configuration-plan","focus":"aws_keyspaces_table.sample1","target":0,"mask":{"name":true}},
  {"fixture":"dynamodb-transfer-plan","focus":"aws_dynamodb_table_export.sample1","target":0,"mask":{"arn":true}},
  {"fixture":"lustre-sources-plan","focus":"aws_fsx_lustre_file_system.sample1","target":0,"mask":{"id":true}},
  {"fixture":"storagegateway-config-plan","focus":"aws_storagegateway_gateway.sample0","target":0,"mask":{"arn":true}},
  {"fixture":"kendra-index-plan","focus":"aws_kendra_experience.sample5","target":3,"mask":{"data_source_id":true}},
  {"fixture":"qbusiness-application-plan","focus":"aws_qbusiness_application.sample1","target":0,"mask":{"customer_master_key_spec":true}},
  {"fixture":"beanstalk-configuration-plan","focus":"aws_elastic_beanstalk_environment.sample1","target":0,"mask":{"name":true}},
  {"fixture":"apprunner-configuration-plan","focus":"aws_apprunner_service.sample1","target":0,"mask":{"arn":true}},
  {"fixture":"cloudwatch-omni-plan","focus":"aws_cloudwatchomni_space.sample1","target":0,"mask":{"domain_id":true}},
  {"fixture":"observability-logs-plan","focus":"aws_observabilityadmin_s3_table_integration.sample1","target":0,"mask":{"arn":true}},
  {"fixture":"observability-monitors-plan","focus":"aws_applicationinsights_application.sample1","target":0,"mask":{"name":true}},
  {"fixture":"athena-queries-plan","focus":"aws_athena_workgroup.sample2","target":2,"mask":{"configuration":true}},
  {"fixture":"timestream-queries-plan","focus":"aws_timestreamquery_scheduled_query.row3","target":3,"mask":{"target_configuration":true}},
  {"fixture":"timestream-influx-plan","focus":"aws_timestreaminfluxdb_db_instance.row5","target":5,"mask":{"vpc_security_group_ids":true}},
  {"fixture":"firehose-msk-source-plan","focus":"aws_kinesis_firehose_delivery_stream.row2","target":2,"mask":{"msk_source_configuration":true}},
  {"fixture":"msk-delivery-plan","focus":"aws_msk_channel.row4","target":4,"mask":{"topic_configuration":true}},
  {"fixture":"msk-connect-plan","focus":"aws_mskconnect_connector.row3","target":3,"mask":{"plugin":true}},
  {fixture: "quicksight-identity-plan", focus: "aws_quicksight_group_membership.g", target: 3, mask: {member_name: true}},
  {fixture: "quicksight-content-plan", focus: "aws_quicksight_dashboard.d", target: 3, mask: {source_entity: true}},
  {fixture: "quicksight-connection-plan", focus: "aws_quicksight_vpc_connection.one", target: 6, mask: {parameters: true}},
  {"fixture":"sagemaker-workforce-plan","focus":"aws_sagemaker_workforce.private","target":2,"mask":{"cognito_config":true}},
  {"fixture":"sagemaker-catalog-plan","focus":"aws_sagemaker_model_card_export_job.export","target":3,"mask":{"model_card_name":true}},
  {"fixture":"sagemaker-training-jobs-plan","focus":"aws_sagemaker_training_job.job","target":3,"mask":{"input_data_config":true}},
  {"fixture":"sagemaker-studio-plan","focus":"aws_sagemaker_app.ui","target":5,"mask":{"resource_spec":true}},
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
