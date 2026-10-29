'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {spawnSync}=require('node:child_process');
const {extract,buildEvidence,validate,query}=require('../../scripts/aws-semantic-audit.cjs');
const read=p=>JSON.parse(fs.readFileSync(p));
const ec2VpcExpected=[
  "aws_default_network_acl",
  "aws_default_route_table",
  "aws_default_security_group",
  "aws_default_subnet",
  "aws_default_vpc",
  "aws_default_vpc_dhcp_options",
  "aws_ec2_instance_connect_endpoint",
  "aws_ec2_managed_prefix_list",
  "aws_ec2_managed_prefix_list_entry",
  "aws_ec2_network_insights_access_scope",
  "aws_ec2_network_insights_analysis",
  "aws_ec2_network_insights_path",
  "aws_ec2_secondary_network",
  "aws_ec2_secondary_subnet",
  "aws_ec2_subnet_cidr_reservation",
  "aws_ec2_traffic_mirror_filter",
  "aws_ec2_traffic_mirror_filter_rule",
  "aws_ec2_traffic_mirror_session",
  "aws_ec2_traffic_mirror_target",
  "aws_egress_only_internet_gateway",
  "aws_eip",
  "aws_eip_association",
  "aws_eip_domain_name",
  "aws_flow_log",
  "aws_internet_gateway",
  "aws_internet_gateway_attachment",
  "aws_main_route_table_association",
  "aws_nat_gateway",
  "aws_nat_gateway_eip_association",
  "aws_network_acl",
  "aws_network_acl_association",
  "aws_network_acl_rule",
  "aws_network_interface",
  "aws_network_interface_attachment",
  "aws_network_interface_permission",
  "aws_network_interface_sg_attachment",
  "aws_route",
  "aws_route_table",
  "aws_route_table_association",
  "aws_security_group",
  "aws_security_group_rule",
  "aws_subnet",
  "aws_vpc",
  "aws_vpc_block_public_access_exclusion",
  "aws_vpc_block_public_access_options",
  "aws_vpc_dhcp_options",
  "aws_vpc_dhcp_options_association",
  "aws_vpc_encryption_control",
  "aws_vpc_endpoint",
  "aws_vpc_endpoint_connection_accepter",
  "aws_vpc_endpoint_connection_notification",
  "aws_vpc_endpoint_policy",
  "aws_vpc_endpoint_private_dns",
  "aws_vpc_endpoint_route_table_association",
  "aws_vpc_endpoint_security_group_association",
  "aws_vpc_endpoint_service",
  "aws_vpc_endpoint_service_allowed_principal",
  "aws_vpc_endpoint_service_private_dns_verification",
  "aws_vpc_endpoint_subnet_association",
  "aws_vpc_ipam",
  "aws_vpc_ipam_organization_admin_account",
  "aws_vpc_ipam_pool",
  "aws_vpc_ipam_pool_cidr",
  "aws_vpc_ipam_pool_cidr_allocation",
  "aws_vpc_ipam_preview_next_cidr",
  "aws_vpc_ipam_resource_discovery",
  "aws_vpc_ipam_resource_discovery_association",
  "aws_vpc_ipam_scope",
  "aws_vpc_ipv4_cidr_block_association",
  "aws_vpc_ipv6_cidr_block_association",
  "aws_vpc_network_performance_metric_subscription",
  "aws_vpc_peering_connection",
  "aws_vpc_peering_connection_accepter",
  "aws_vpc_peering_connection_options",
  "aws_vpc_route_server",
  "aws_vpc_route_server_endpoint",
  "aws_vpc_route_server_peer",
  "aws_vpc_route_server_propagation",
  "aws_vpc_route_server_vpc_association",
  "aws_vpc_security_group_egress_rule",
  "aws_vpc_security_group_ingress_rule",
  "aws_vpc_security_group_rules_exclusive",
  "aws_vpc_security_group_vpc_association"
];
const dimensions=['placement','ownership','network','references','scope','summaries'];

test('#719 covers the 61 storage types with separate contracts and dispositions',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const services='s3 s3control s3outposts s3tables s3vectors s3files glacier'.split(' ');
 const expected=inventory.resources.filter(r=>services.includes(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===719);
 assert.equal(expected.length,61);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review:r,gap,issue}] of entries){
  assert.equal(r.implementationCommit,'00bf0c990596acc196f37c10baf11d2b379c7eaa');
  assert.equal(r.providerCommit,inventory.commit);
  assert.equal(r.officialDocumentation,expected.find(x=>x.type===type).documentation);
  assert.deepEqual(Object.keys(r.shapes),r.fields);
  assert.deepEqual(Object.keys(r.contracts),r.fields);
  for(const key of dimensions)assert.match(r[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  for(const path of r.implementationEvidence)assert.ok(fs.existsSync(path),path);
  assert.ok(r.managedDataContract);
  assert.ok(r.dataContracts);
  assert.match(gap,/#719 audit:/);
  if(dimensions.some(k=>/Missing:/.test(r[k])))assert.ok(issue>=973&&issue<=982,type);
 }
});

test('#719 preserves storage identity and shape boundaries',()=>{
 const d=read('docs/aws-semantic-decisions.json').types;
 const r=t=>d['aws_'+t].review;
 assert.match(r('s3_bucket_replication_configuration').references,/Partial: #511.*Missing: role ARN/);
 assert.match(r('s3_bucket_server_side_encryption_configuration').ownership,/Supported: #601.*unique/);
 assert.match(r('s3_bucket_object').managedDataContract,/Deprecated resource, not a verified runtime alias/);
 assert.equal(r('s3_bucket_metadata_configuration').contracts['metadata_configuration.destination'],'computed');
 assert.equal(r('s3tables_table').shapes.encryption_configuration[0][0],'object');
 assert.equal(r('s3vectors_index').shapes.encryption_configuration[0][0],'list');
 assert.deepEqual(r('s3tables_table_replication').shapes['rule.destination.destination_table_bucket_arn'],['list','set','string']);
 assert.match(r('s3control_bucket_policy').references,/Outposts bucket ARN, not general-purpose bucket name/);
 assert.match(r('s3files_access_point').references,/incoming Lambda #950/);
 assert.match(r('s3files_mount_target').network,/Missing:.*one compatible VPC/);
 assert.match(r('s3_directory_bucket').network,/cannot place.*subnet/);
 assert.match(r('s3control_multi_region_access_point').scope,/Unknown/);
});

test('#715 covers exactly 86 edge and DNS types with pinned evidence and bounded follow-ups',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const services=['elb','elbv2','cloudfront','cloudfrontkeyvaluestore','globalaccelerator','route53','route53domains','route53profiles','route53resolver'];
 const expected=inventory.resources.filter(r=>services.includes(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===715);
 assert.equal(expected.length,86);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review:r,gap,issue}] of entries){
  assert.equal(r.implementationCommit,'f7f5d8bfb39ded416c786a21037935232b8f1eda');
  assert.equal(r.providerCommit,inventory.commit);
  assert.equal(r.officialDocumentation,expected.find(x=>x.type===type).documentation);
  assert.ok(r.fields.length>0);
  assert.deepEqual(Object.keys(r.shapes),r.fields);
  assert.deepEqual(Object.keys(r.contracts),r.fields);
  for(const key of dimensions)assert.match(r[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  for(const p of r.implementationEvidence)assert.ok(fs.existsSync(p),type+': '+p);
  assert.ok(r.managedDataContract);
  assert.ok(r.dataContracts);
  assert.match(gap,/#715 audit:/);
  assert.doesNotMatch(gap,/defer until an explicit endpoint contract|require field-level review/);
  if(dimensions.some(k=>/Missing:|missing[: ]/i.test(r[k])))assert.ok(Number.isInteger(issue)&&issue>=906&&issue<=927,type+' missing follow-up');
 }
});

test('#715 preserves alias, collection, identity and managed/data distinctions',()=>{
 const d=read('docs/aws-semantic-decisions.json').types;
 const r=t=>d['aws_'+t].review;
 for(const suffix of ['','_listener','_listener_certificate','_listener_rule','_target_group','_target_group_attachment']){
  assert.deepEqual(r('alb'+suffix).shapes,r('lb'+suffix).shapes);
  assert.equal(d['aws_alb'+suffix].issue,d['aws_lb'+suffix].issue);
  assert.match(r('alb'+suffix).managedDataContract,/Verified managed alias/);
 }
 assert.deepEqual(r('alb_listener_rule').dataContracts,{});
 assert.ok(r('lb_listener_rule').dataContracts.arn);
 assert.match(r('elb').placement,/Missing:.*Logical/);
 assert.match(r('elb').references,/not an alias/);
 assert.equal(r('load_balancer_listener_policy').contracts.policy_names,'optional');
 assert.deepEqual(r('proxy_protocol_policy').shapes.instance_ports,[['set','string']]);
 assert.deepEqual(r('lb').shapes['subnet_mapping.subnet_id'],['set','string']);
 assert.match(r('lb_listener').network,/not containment/);
 assert.match(r('lb_target_group_attachment').references,/Supported: Implemented #910:.*matching known values.*target_type/);
 assert.match(r('lb_target_group').ownership,/Supported:.*one LB/);
 assert.deepEqual(r('cloudfront_distribution').shapes['origin.domain_name'],['set','string']);
 assert.deepEqual(r('cloudfront_multitenant_distribution').shapes['default_cache_behavior.trusted_key_groups.items'],['list','list',['list','string']]);
 assert.match(r('cloudfront_multitenant_distribution').references,/lambda_function_arn differ/);
 assert.equal(r('cloudfront_cache_policy').contracts.min_ttl,'optional');
 assert.equal(r('cloudfront_continuous_deployment_policy').contracts['staging_distribution_dns_names.items'],'optional');
 assert.ok(r('cloudfront_connection_function').sourceContracts);
 assert.equal(r('cloudfront_distribution_tenant').contracts.connection_group_id,'optional/computed');
 assert.match(r('cloudfrontkeyvaluestore_keys_exclusive').references,/destroy does not delete keys/);
 assert.match(r('globalaccelerator_custom_routing_endpoint_group').references,/subnet ID/);
 assert.match(r('globalaccelerator_endpoint_group').scope,/endpoint_group_region/);
 assert.match(r('route53_record').references,/#489.*paired/);
 assert.deepEqual(r('route53_records_exclusive').shapes['resource_record_set.alias_target.dns_name'],['set','list','string']);
 assert.match(r('route53_vpc_association_authorization').references,/not evidence the association exists/);
 assert.equal(r('route53_health_check').contracts.request_interval,'optional');
 assert.match(r('route53_key_signing_key').references,/hostedZoneID and KSK name/);
 assert.equal(r('route53domains_domain').contracts.hosted_zone_id,'computed');
 assert.equal(r('route53domains_domain').shapes.name_server[0][0],'list');
 assert.deepEqual(r('route53domains_registered_domain').shapes.name_server,['list']);
 assert.deepEqual(r('route53_resolver_endpoint').shapes['ip_address.subnet_id'],['set','string']);
 assert.equal(r('route53_resolver_firewall_config').contracts.firewall_fail_open,'optional/computed');
 assert.match(r('route53_resolver_firewall_rule').references,/Advanced rules have no domain list/);
 assert.equal(r('route53profiles_resource_association').contracts.resource_type,'computed');
});

test('#710 retains exactly its 83 managed types and six evidenced decisions',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===710);
 assert.equal(ec2VpcExpected.length,83);
 assert.deepEqual(entries.map(([t])=>t).sort(),[...ec2VpcExpected].sort());
 for(const [type,{review:r,gap,issue}] of entries){
  assert.equal(r.implementationCommit,'a874b24ecfe4f0361ac81dc5afcb87e6570cd1d3');
  assert.equal(r.providerCommit,inventory.commit);
  assert.equal(r.officialDocumentation,inventory.resources.find(x=>x.type===type).documentation);
  assert.deepEqual(Object.keys(r.shapes),r.fields);
  assert.deepEqual(Object.keys(r.contracts),r.fields);
  for(const key of dimensions)assert.match(r[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  for(const path of r.implementationEvidence)assert.ok(fs.existsSync(path),type+': '+path);
  assert.ok(r.managedDataContract);
  assert.match(gap,/#710 audit:|Implemented #(897|895|899|902):/);
  if(dimensions.some(k=>/Missing:|missing[: ]/i.test(r[k])))assert.ok(Number.isInteger(issue)&&issue>0,type+' missing follow-up');
 }
});

test('#710 preserves adoption, mode, collection and identity distinctions',()=>{
 const d=read('docs/aws-semantic-decisions.json').types;
 const r=t=>d['aws_'+t].review;
 assert.equal(r('default_subnet').contracts.vpc_id,'computed');
 assert.match(r('default_vpc').placement,/Missing:.*Logical/);
 assert.deepEqual(r('default_vpc_dhcp_options').shapes.netbios_name_servers,['string']);
 assert.equal(r('default_vpc_dhcp_options').contracts.netbios_name_servers,'computed');
 assert.deepEqual(r('nat_gateway').shapes['availability_zone_address.allocation_ids'],['set',['set','string']]);
 assert.equal(r('nat_gateway').contracts['availability_zone_address.allocation_ids'],'optional');
 assert.match(r('nat_gateway').network,/regional/);
 assert.ok(r('nat_gateway_eip_association').sourceContracts.association_id);
 assert.deepEqual(r('network_interface').shapes['attachment.instance'],['set','string']);
 assert.match(r('network_interface').references,/incoming.*route/);
 assert.equal(r('route').contracts.instance_id,'computed');
 assert.match(r('route').ownership,/Supported:.*matching planned value/);
 assert.equal(r('route_table').shapes.route[0][0],'set');
 assert.equal(r('route_table').shapes.route[0][1][0],'object');
 assert.match(r('route_table_association').references,/Partial:.*gateway_id/);
 assert.match(r('vpc_endpoint_subnet_association').references,/no lowering rule/);
 assert.match(r('vpc_endpoint').references,/service_region differs/);
 assert.deepEqual(r('vpc_security_group_rules_exclusive').shapes.ingress_rule_ids,[['set','string']]);
 assert.match(r('vpc_ipam_organization_admin_account').scope,/Global/);
 assert.match(r('vpc_peering_connection').scope,/no dedicated peering region/);
 assert.match(r('vpc_route_server').references,/no generic id/);
 assert.equal(r('vpc_ipam_pool_cidr_allocation').contracts.resource_id,'computed');
 assert.deepEqual(r('ec2_network_insights_access_scope').shapes['match_paths.source.resource_statement.resources'],['list','list','list',['list','string']]);
 assert.match(r('ec2_network_insights_analysis').network,/Intentionally unsupported:.*computed/);
 assert.match(r('vpc').managedDataContract,/default \(optional\/computed\)/);
 assert.match(r('vpc_network_performance_metric_subscription').references,/region or AZ selectors/);
});




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
test('ML audit covers all 90 managed contracts without conflating model and runtime identities',()=>{
 const services=new Set('sagemaker bedrock bedrockagent bedrockagentcore agentregistry comprehend rekognition transcribe'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===725);
 assert.equal(expected.length,90);
 assert.deepEqual(entries.map(([type])=>type).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,issue,gap}] of entries){
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.equal(review.implementationCommit,'f5c320438d8abf81de31c884bbd03dde5369773e');
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  assert.deepEqual(Object.keys(review.contracts),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries']) assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  if(Object.values(review).some(v=>typeof v==='string'&&v.startsWith('Missing:'))) assert.ok(issue===701||(issue>=872&&issue<=881),type);
  assert.match(gap,/#725/);
 }
 const d=Object.fromEntries(entries);
 assert.deepEqual(d.aws_sagemaker_model.review.shapes['vpc_config.subnets'],['list',['set','string']]);
 assert.deepEqual(d.aws_sagemaker_training_job.review.shapes['vpc_config.subnets'],['list',['list','string']]);
 assert.equal(d.aws_sagemaker_endpoint_configuration.review.contracts['production_variants.model_name'],'optional');
 assert.match(d.aws_sagemaker_endpoint.review.network,/do not inherit model network/);
 assert.match(d.aws_bedrock_custom_model.review.references,/customization-job ARNs/);
 assert.match(d.aws_bedrockagentcore_registry.review.references,/distinct service factory/);
 assert.equal(d.aws_bedrockagent_agent.review.shapes.guardrail_configuration[0][0],'list');
 assert.match(d.aws_bedrockagentcore_api_key_credential_provider.review.references,/EXTERNAL.*MANAGED/);
 assert.match(d.aws_bedrock_use_case_for_model_access.review.scope,/Unknown.*disables region override/);
});

test('candidate extraction respects nested collections and sensitive boundaries',()=>{
 assert.deepEqual(extract({attributes:{role_arn:{optional:true,type:'string'},computed_id:{computed:true},secret_id:{optional:true,sensitive:true},write_id:{optional:true,write_only:true},secret:{sensitive:true,nested_type:{attributes:{target_id:{optional:true}}}}},block_types:{network:{block:{attributes:{subnets:{required:true,type:['set','string']},security_groups:{optional:true}}}}}}),['network.security_groups','network.subnets','role_arn']);
});

test('application audit covers 71 exact managed types with six decisions and source evidence',()=>{
 const services=new Set('lambda lambdacore lambdamicrovms apigateway apigatewayv2 appsync serverlessrepo'.split(' '));
 const expected=read('tests/fixtures/aws-inventory/6.70.0.json').resources.filter(r=>services.has(r.service));
 const entries=Object.entries(read('docs/aws-semantic-decisions.json').types).filter(([,d])=>d.review?.issue===714);
 assert.equal(expected.length,71);
 assert.deepEqual(entries.map(([t])=>t).sort(),expected.map(r=>r.type).sort());
 for(const [type,{review,gap,issue}] of entries){
  assert.equal(review.providerCommit,'5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f');
  assert.equal(review.implementationCommit,'80b70c8fec8b5a3c75b0457bf0f4061fccea441e');
  assert.equal(review.officialDocumentation,expected.find(r=>r.type===type).documentation);
  assert.deepEqual(Object.keys(review.shapes),review.fields);
  assert.deepEqual(Object.keys(review.contracts),review.fields);
  for(const key of ['placement','ownership','network','references','scope','summaries'])assert.match(review[key],/^(Supported|Partial|Missing|Not applicable|Intentionally unsupported|Deferred):/,type+': '+key);
  for(const file of review.implementationEvidence)assert.ok(fs.existsSync(file),file);
  assert.match(review.managedDataContract,/No managed alias/);
  assert.match(gap,/#714/);
  if(Object.values(review).some(v=>typeof v==='string' && v.startsWith('Missing:')))assert.ok(issue>=930&&issue<=955,type);
 }
});

test('application audit preserves qualifiers, API families and nested attribute contracts',()=>{
 const d=read('docs/aws-semantic-decisions.json').types;
 assert.deepEqual(d.aws_lambda_layer_version.review.shapes.version,['string']);
 assert.deepEqual(d.aws_lambda_layer_version_permission.review.shapes.version_number,['number']);
 assert.match(d.aws_lambda_function_scaling_config.review.references,/\$LATEST\.PUBLISHED/);
 assert.match(d.aws_apigatewayv2_routing_rule.review.references,/REST API\/stage/);
 assert.match(d.aws_api_gateway_integration.review.references,/integration_target/);
 assert.deepEqual(d.aws_appsync_source_api_association.review.shapes.source_api_association_config, [['list',['object',{merge_type:'string'}]]]);
 assert.deepEqual(d.aws_appsync_api.review.shapes['event_config.log_config.cloudwatch_logs_role_arn'],['list','list','string']);
 assert.match(d.aws_appsync_graphql_api.review.network,/GLOBAL.*not global/);
 assert.match(d.aws_lambda_function.review.network,/older paths.*known-value/);
 assert.match(d.aws_lambdamicrovms_image.review.references,/INTERNET_EGRESS/);
 assert.match(d.aws_serverlessapplicationrepository_cloudformation_stack.review.references,/no managed SAR application/);
});
test('untrusted schema bytes cannot replace pinned evidence',()=>{
 assert.throws(()=>buildEvidence(Buffer.from('{}')),/SHA-256/);
});
test('queries select exact types or services and validate the full evidence inventory',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const evidence=read('docs/aws-semantic-evidence.json');
 validate(evidence);
 assert.deepEqual(query(evidence,'kafka').map(r=>r.type),inventory.resources.filter(r=>r.service==='kafka').map(r=>r.type));
 const rows=query(evidence,'aws_msk_cluster');
 assert.equal(rows.length,1);
 assert.equal(rows[0].type,'aws_msk_cluster');
 assert.deepEqual(rows[0].candidates,evidence.candidates.aws_msk_cluster);
 assert.deepEqual(rows[0].decision,read('docs/aws-semantic-decisions.json').types.aws_msk_cluster);
 assert.throws(()=>query(evidence,'aws_msk_'),/Unknown managed type/);
 delete evidence.candidates.aws_msk_cluster;
 assert.throws(()=>validate(evidence),/inventory mismatch/);
});

test('CLI checks without output and returns bounded JSON queries instead of Markdown',()=>{
 const run=(...args)=>spawnSync(process.execPath,['scripts/aws-semantic-audit.cjs',...args],{encoding:'utf8'});
 const checked=run('--check');
 assert.equal(checked.status,0,checked.stderr);
 assert.equal(checked.stdout,'');
 const selected=run('aws_msk_cluster');
 assert.equal(selected.status,0,selected.stderr);
 assert.deepEqual(JSON.parse(selected.stdout).map(r=>r.type),['aws_msk_cluster']);
 for(const args of [[],['--check','unexpected'],['--schema'],['aws_missing_type']]) {
  const invalid=run(...args);
  assert.notEqual(invalid.status,0);
  assert.equal(invalid.stdout,'');
 }
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
