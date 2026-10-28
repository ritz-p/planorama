'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {extract,buildEvidence,render}=require('../../scripts/aws-semantic-audit.cjs');
const read=p=>JSON.parse(fs.readFileSync(p));

test('analytics batch 2 covers 31 exact types without borrowing DMS implementation',()=>{
 const services=new Set('dms datazone dataexchange datapipeline cleanrooms'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===723);
 assert.equal(expected.length,31);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.implementationCommit,'c50d830c5737a51a362708b71f67e6c407884dbe');
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  assert.deepEqual(Object.keys(review.contracts),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#723/);
  if(['placement','ownership','network','references','scope'].some(k=>review[k].includes('Missing:'))) assert.ok(issue===699||(issue>=857&&issue<=866),type);
 }
 const d=Object.fromEntries(entries);
 assert.match(d.aws_dms_replication_instance.review.placement,/Spatial/);
 assert.deepEqual(d.aws_dms_replication_subnet_group.review.shapes.subnet_ids,[['set','string']]);
 assert.equal(d.aws_dms_replication_config.review.contracts['compute_config.replication_subnet_group_id'],'required');
 assert.match(d.aws_dms_replication_task.review.references,/endpoint_arn/);
 assert.match(d.aws_dataexchange_revision.review.references,/combines dataset and revision/);
 assert.match(d.aws_datapipeline_pipeline_definition.review.references,/internal pipeline objects/);
 assert.equal(d.aws_datazone_environment_profile.review.contracts.aws_account_id,'optional/computed');
 assert.deepEqual(d.aws_cleanrooms_membership.review.shapes['default_result_configuration.output_configuration.s3.bucket'],['list','list','list','string']);
});

test('analytics batch 1 covers 81 exact types and preserves qualified identities',()=>{
 const services=new Set('athena redshift redshiftserverless redshiftdata glue lakeformation emr emrcontainers emrserverless'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===722);
 assert.equal(expected.length,81);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.implementationCommit,'c50d830c5737a51a362708b71f67e6c407884dbe');
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  assert.deepEqual(Object.keys(review.contracts),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#722/);
  if(['placement','ownership','network','references'].some(k=>review[k].includes('Missing:'))) assert.ok(issue>=840&&issue<=855,type);
 }
 const d=Object.fromEntries(entries);
 assert.equal(d.aws_athena_database.review.contracts.bucket,'optional');
 assert.deepEqual(d.aws_emr_cluster.review.shapes['ec2_attributes.additional_master_security_groups'],['list','string']);
 assert.match(d.aws_emr_cluster.review.placement,/Spatial/);
 assert.match(d.aws_emrserverless_application.review.placement,/Spatial/);
 assert.match(d.aws_glue_catalog_table.review.ownership,/#545/);
 assert.equal(d.aws_glue_catalog.review.contracts['catalog_properties.data_lake_access_properties.managed_workgroup_name'],'computed');
 assert.deepEqual(d.aws_lakeformation_resource_lf_tag.review.shapes['lf_tag.key'],['list','string']);
 assert.deepEqual(d.aws_lakeformation_resource_lf_tags.review.shapes['lf_tag.key'],['set','string']);
 assert.deepEqual(d.aws_redshift_cluster.review.shapes.encrypted,['string']);
 assert.match(d.aws_redshiftserverless_workgroup.review.network,/#695/);
 assert.equal(d.aws_redshiftserverless_snapshot.review.contracts.namespace_arn,'computed');
});

test('filesystem audit covers 70 exact types and distinguishes configured references from service outputs',()=>{
 const services=new Set('efs fsx storagegateway datasync transfer backup dlm rbin drs'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===720);
 assert.equal(expected.length,70);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.equal(review.implementationCommit,'514c17316e596fbbd0a8c7aa4942a91e3462e09d');
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  assert.deepEqual(Object.keys(review.contracts),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#720/);
  if(['placement','ownership','network','references'].some(k=>review[k].includes('Missing:'))) assert.ok(issue===702||(issue>=820&&issue<=836),type);
 }
 const d=Object.fromEntries(entries);
 assert.deepEqual(d.aws_fsx_lustre_file_system.review.shapes.subnet_ids,[['list','string']]);
 assert.deepEqual(d.aws_fsx_lustre_file_system.review.shapes.security_group_ids,[['set','string']]);
 assert.match(d.aws_storagegateway_cached_iscsi_volume.review.references,/IPv4 address, NOT EC2 ENI ID/);
 assert.equal(d.aws_datasync_location_fsx_ontap_file_system.review.contracts.fsx_filesystem_arn,'computed');
 assert.equal(d.aws_datasync_location_fsx_lustre_file_system.review.contracts.security_group_arns,'required');
 assert.equal(d.aws_drs_replication_configuration_template.review.contracts.ebs_encryption_key_arn,'optional');
 assert.equal(d.aws_transfer_web_app.review.contracts['endpoint_details.vpc.vpc_endpoint_id'],'computed');
 assert.match(d.aws_transfer_ssh_key.review.references,/server ID AND user_name/);
 assert.match(d.aws_efs_access_point.review.ownership,/#548/);
 assert.match(d.aws_backup_plan.review.references,/#508.*one rule/);
});

test('compute audit covers 52 types and keeps names, ARNs and collection evidence distinct',()=>{
 const services=new Set('autoscaling autoscalingplans appautoscaling batch imagebuilder lightsail outposts computeoptimizer'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===712);
 assert.equal(expected.length,52);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.equal(review.implementationCommit,'2bef52d045db4c5cf72374a0dca24d0629ed84e3');
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#712/);
  if(['placement','ownership','network','references'].some(k=>review[k].startsWith('Missing:'))) assert.ok(issue===700||(issue>=810&&issue<=816),type);
 }
 const d=Object.fromEntries(entries);
 assert.match(d.aws_autoscaling_group.review.placement,/Spatial Controller/);
 assert.match(d.aws_autoscaling_policy.review.references,/#362/);
 assert.deepEqual(d.aws_batch_compute_environment.review.shapes['compute_resources.subnets'],['list',['set','string']]);
 assert.deepEqual(d.aws_batch_job_queue.review.shapes['compute_environment_order.compute_environment'],['list','string']);
 assert.match(d.aws_batch_job_definition.review.references,/revision-qualified/);
 assert.match(d.aws_lightsail_instance.review.references,/id\/arn are ARN/);
 assert.match(d.aws_lightsail_lb_certificate.review.references,/LB-qualified/);
 assert.match(d.aws_outposts_capacity_task.review.references,/exclusion-label/);
});

test('search and BI review covers 65 types without borrowing adjacent-service semantics',()=>{
 const services=new Set('opensearch opensearchserverless elasticsearch cloudsearch osis kendra qbusiness quicksight finspace'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===724);
 assert.equal(expected.length,65);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#724/);
  if(Object.values(review).some(v=>typeof v==='string'&&v.startsWith('Missing:'))) assert.ok(issue===696||(issue>=797&&issue<=806),type);
 }
 const d=Object.fromEntries(entries);
 assert.match(d.aws_opensearch_domain.review.network,/#181.*#182/);
 assert.match(d.aws_elasticsearch_domain.review.placement,/not a verified alias/);
 assert.match(d.aws_opensearchserverless_collection.review.network,/no collection VPC/);
 assert.deepEqual(d.aws_opensearchserverless_vpc_endpoint.review.shapes.subnet_ids,[['set','string']]);
 assert.deepEqual(d.aws_opensearchserverless_collection.review.shapes.encryption_config,[["list",["object",{"aws_owned_key":"bool","kms_key_arn":"string"}]]]);
 assert.match(d.aws_osis_pipeline.review.references,/id is deprecated/);
 assert.match(d.aws_quicksight_role_membership.review.references,/not an IAM role/);
 assert.match(d.aws_quicksight_ip_restriction.review.references,/map-key reference provenance/);
});

test('network audit covers 68 types and preserves typed endpoint contracts',()=>{
 const services=new Set('directconnect networkmanager networkfirewall vpclattice networkflowmonitor networkmonitor'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===716);
 assert.equal(expected.length,68);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#716/);
  assert.ok(issue===693 || (issue>=786 && issue<=793));
 }
 const d=Object.fromEntries(entries);
 assert.deepEqual(d.aws_networkfirewall_firewall.review.shapes['subnet_mapping.subnet_id'],['set','string']);
 assert.deepEqual(d.aws_networkfirewall_logging_configuration.review.shapes['logging_configuration.log_destination_config.log_destination'],['list','set',['map','string']]);
 assert.match(d.aws_networkfirewall_tls_inspection_configuration.review.references,/not ACM PCA/);
 assert.match(d.aws_dx_hosted_connection.review.scope,/deprecated region/);
 assert.match(d.aws_vpclattice_target_group_attachment.review.references,/IP values intentionally unsupported/);
 assert.match(d.aws_networkmanager_vpc_attachment.review.network,/core network stays global/);
});

test('container audit covers 57 managed types with pinned dimensions and distinct network contracts',()=>{
 const services=new Set('ecs eks ecr ecrpublic apprunner elasticbeanstalk appmesh servicediscovery'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===713);
 assert.equal(expected.length,57);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap}] of entries) {
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#713/);
 }
 const d=Object.fromEntries(entries);
 assert.deepEqual(d.aws_apprunner_vpc_connector.review.shapes.subnets,[['set','string']]);
 assert.deepEqual(d.aws_ecs_daemon.review.shapes.capacity_provider_arns,[['set','string']]);
 assert.match(d.aws_apprunner_service.review.network,/do not contain service/);
 assert.match(d.aws_eks_access_policy_association.review.references,/not IAM policy/);
 assert.match(d.aws_ecrpublic_repository.review.scope,/us-east-1/);
 assert.match(d.aws_service_discovery_service.review.references,/direct\/DNS/);
});

test('relational audit covers 59 exact managed types and preserves field distinctions',()=>{
 const services=new Set('rds docdb docdbelastic neptune neptunegraph dsql odb'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===717);
 assert.equal(expected.length,59);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap}] of entries) {
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#717/);
 }
 const decisions=Object.fromEntries(entries);
 assert.deepEqual(decisions.aws_docdb_subnet_group.review.shapes.subnet_ids,[['set','string']]);
 assert.deepEqual(decisions.aws_dsql_cluster.review.shapes['multi_region_properties.clusters'],['list',['set','string']]);
 assert.ok(decisions.aws_db_event_subscription.review.fields.includes('sns_topic'));
 assert.ok(decisions.aws_docdb_event_subscription.review.fields.includes('sns_topic_arn'));
 assert.match(decisions.aws_db_snapshot.review.network,/Not applicable:.*VPC/);
 assert.match(decisions.aws_rds_cluster_instance.review.ownership,/Supported:.*members.rs/);
});
test('candidate extraction respects nested collections and sensitive boundaries',()=>{
 assert.deepEqual(extract({attributes:{role_arn:{optional:true,type:'string'},computed_id:{computed:true},secret_id:{optional:true,sensitive:true},write_id:{optional:true,write_only:true},secret:{sensitive:true,nested_type:{attributes:{target_id:{optional:true}}}}},block_types:{network:{block:{attributes:{subnets:{required:true,type:['set','string']},security_groups:{optional:true}}}}}}),['network.security_groups','network.subnets','role_arn']);
});
test('untrusted schema bytes cannot replace pinned evidence',()=>{
 assert.throws(()=>buildEvidence(Buffer.from('{}')),/SHA-256/);
});
test('register covers every exact managed type once and rejects incomplete evidence',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const evidence=read('docs/aws-semantic-evidence.json');
 const output=render(evidence);
 const names=[...output.matchAll(/^\| \[`(aws_[^`]+)`\]/gm)].map(m=>m[1]);
 assert.deepEqual(names.sort(),inventory.resources.map(r=>r.type).sort());
 assert.equal(output,fs.readFileSync('docs/aws-semantic-gaps.md','utf8'));
 delete evidence.candidates.aws_msk_cluster;
 assert.throws(()=>render(evidence),/inventory mismatch/);
});
test('representative evidence includes nested network paths without asserting implementation',()=>{
 const {candidates}=read('docs/aws-semantic-evidence.json');
 assert.ok(candidates.aws_msk_cluster.includes('broker_node_group_info.client_subnets'));
 assert.ok(candidates.aws_apprunner_service.includes('network_configuration.egress_configuration.vpc_connector_arn'));
 const decisions=read('docs/aws-semantic-decisions.json').types;
 assert.equal(decisions.aws_apprunner_service.status,'Partial');
 assert.equal(decisions.aws_dynamodb_table.status,'Partial');
 assert.match(decisions.aws_dynamodb_table.gap,/NOT implemented/);
});

test('NoSQL audit covers its exact managed denominator with six dimensions and pinned contracts',()=>{
 const services=new Set(['dynamodb','dax','elasticache','memorydb','keyspaces','qldb','timestreaminfluxdb','timestreamquery','timestreamwrite']);
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===718);
 assert.equal(expected.length,39);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review}] of entries) {
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.ok(review[key]?.trim().length>0,type+': '+key);
 }
});

test('streaming audit covers its exact managed denominator and preserves bounded evidence',()=>{
 const services=new Set('kafka kafkaconnect mq kinesis kinesisanalytics kinesisanalyticsv2 kinesisvideo firehose events scheduler pipes schemas sns sqs sfn swf mwaa'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===721);
 assert.equal(expected.length,55);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap}] of entries) {
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.match(review.implementationCommit,/^[a-f0-9]{40}$/);
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  assert.match(gap,/#721/);
 }
 const decisions=Object.fromEntries(entries);
 assert.deepEqual(decisions.aws_mq_broker.review.shapes.subnet_ids,[['set','string']]);
 assert.deepEqual(decisions.aws_msk_serverless_cluster.review.shapes['vpc_config.subnet_ids'],['list',['set','string']]);
 assert.match(decisions.aws_sns_topic_subscription.review.references,/SQS\/Lambda/);
 assert.match(decisions.aws_sqs_queue.review.summaries,/Supported:.*#601/);
});
