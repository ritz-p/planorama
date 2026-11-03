const fs = require('node:fs');
const resources = [], configs = [];
const arn = (service, id) => `arn:aws:${service}:us-east-1:123456789012:${id}`;
function add(type, name, after, expressions = {}) {
  const address = `${type}.${name}`;
  resources.push({address, mode: 'managed', type, provider_name: 'registry.terraform.io/hashicorp/aws', change: {actions: ['create'], after}});
  configs.push({address, mode: 'managed', type, provider_config_key: 'aws', expressions});
}
const ref = (...references) => ({references});
add('aws_vpc', 'main', {id: 'vpc-main'});
add('aws_subnet', 'connection', {id: 'subnet-main', arn: arn('ec2', 'subnet/subnet-main'), vpc_id: 'vpc-main'}, {vpc_id: ref('aws_vpc.main.id')});
for (const name of ['one', 'two']) add('aws_security_group', name, {id: `sg-${name}`, arn: arn('ec2', `security-group/sg-${name}`), vpc_id: 'vpc-main'}, {vpc_id: ref('aws_vpc.main.id')});
add('aws_vpc_endpoint', 'sync', {id:'vpce-sync', vpc_id:'vpc-main', vpc_endpoint_type:'Interface'}, {vpc_id:ref('aws_vpc.main.id')});
add('aws_iam_role', 'storage', {arn:'arn:aws:iam::123456789012:role/storage'});
add('aws_efs_file_system', 'shared', {id:'fs-shared', arn:arn('elasticfilesystem','file-system/fs-shared')});
add('aws_efs_access_point', 'shared', {arn:arn('elasticfilesystem','access-point/fsap-shared'), file_system_id:'fs-shared'}, {file_system_id:ref('aws_efs_file_system.shared.id')});
const groups = ['one','two'].map(n=>arn('ec2',`security-group/sg-${n}`));
const groupRefs = ref('aws_security_group.one.arn','aws_security_group.two.arn');
for (const name of ['one','two']) add('aws_datasync_agent', name, {arn:arn('datasync',`agent/agent-${name}`), subnet_arns:[arn('ec2','subnet/subnet-main')],security_group_arns:groups,vpc_endpoint_id:'vpce-sync',private_link_endpoint:'PRIVATE_IP',activation_key:'PRIVATE_ACTIVATION'}, {subnet_arns:ref('aws_subnet.connection.arn'),security_group_arns:groupRefs,vpc_endpoint_id:ref('aws_vpc_endpoint.sync.id')});
add('aws_datasync_location_efs','shared',{arn:arn('datasync','location/loc-efs'),efs_file_system_arn:arn('elasticfilesystem','file-system/fs-shared'),access_point_arn:arn('elasticfilesystem','access-point/fsap-shared'),file_system_access_role_arn:'arn:aws:iam::123456789012:role/storage',in_transit_encryption:'TLS1_2',ec2_config:[{subnet_arn:arn('ec2','subnet/subnet-main'),security_group_arns:groups}]},{efs_file_system_arn:ref('aws_efs_file_system.shared.arn'),access_point_arn:ref('aws_efs_access_point.shared.arn'),file_system_access_role_arn:ref('aws_iam_role.storage.arn'),ec2_config:[{subnet_arn:ref('aws_subnet.connection.arn'),security_group_arns:groupRefs}]});
for (const variant of ['lustre','openzfs','windows']) {
  add(`aws_fsx_${variant}_file_system`,'shared',{arn:arn('fsx',`file-system/fs-${variant}`)});
  add(`aws_datasync_location_fsx_${variant}_file_system`,'shared',{arn:arn('datasync',`location/loc-${variant}`),fsx_filesystem_arn:arn('fsx',`file-system/fs-${variant}`),security_group_arns:groups},{fsx_filesystem_arn:ref(`aws_fsx_${variant}_file_system.shared.arn`),security_group_arns:groupRefs});
}
add('aws_fsx_ontap_storage_virtual_machine','shared',{arn:arn('fsx','storage-virtual-machine/svm-shared'),file_system_id:'fs-ontap'});
add('aws_datasync_location_fsx_ontap_file_system','shared',{arn:arn('datasync','location/loc-ontap'),storage_virtual_machine_arn:arn('fsx','storage-virtual-machine/svm-shared'),fsx_filesystem_arn:arn('fsx','file-system/fs-ontap'),security_group_arns:groups},{storage_virtual_machine_arn:ref('aws_fsx_ontap_storage_virtual_machine.shared.arn'),security_group_arns:groupRefs});
add('aws_s3_bucket','shared',{arn:'arn:aws:s3:::example-storage'});
add('aws_datasync_location_s3','shared',{arn:arn('datasync','location/loc-s3'),s3_bucket_arn:'arn:aws:s3:::example-storage',s3_config:[{bucket_access_role_arn:'arn:aws:iam::123456789012:role/storage'}]},{s3_bucket_arn:ref('aws_s3_bucket.shared.arn'),s3_config:[{bucket_access_role_arn:ref('aws_iam_role.storage.arn')}]});
add('aws_s3_access_point','outpost',{arn:arn('s3-outposts','outpost/op-example/accesspoint/example')});
add('aws_datasync_location_s3','outpost',{arn:arn('datasync','location/loc-outpost'),s3_bucket_arn:arn('s3-outposts','outpost/op-example/accesspoint/example'),agent_arns:[arn('datasync','agent/agent-one')]},{s3_bucket_arn:ref('aws_s3_access_point.outpost.arn'),agent_arns:ref('aws_datasync_agent.one.arn')});
for (const variant of ['azure_blob','hdfs','nfs','object_storage','smb']) {
  const values = {agent_arns:['one','two'].map(n=>arn('datasync',`agent/agent-${n}`))};
  const expressions = {agent_arns:ref('aws_datasync_agent.one.arn','aws_datasync_agent.two.arn')};
  add(`aws_datasync_location_${variant}`,'shared',{arn:arn('datasync',`location/loc-${variant}`),...(variant==='nfs'?{on_prem_config:[values]}:values),server_hostname:'PRIVATE_HOST',subdirectory:'PRIVATE_PATH'},variant==='nfs'?{on_prem_config:[expressions]}:expressions);
}
add('aws_cloudwatch_log_group','sync',{arn:arn('logs','log-group:sync')});
add('aws_datasync_task','sync',{arn:arn('datasync','task/task-sync'),source_location_arn:arn('datasync','location/loc-nfs'),destination_location_arn:'arn:aws:datasync:us-east-1:123456789012:location/loc-s3',cloudwatch_log_group_arn:arn('logs','log-group:sync'),task_report_config:[{s3_destination:[{s3_bucket_arn:'arn:aws:s3:::example-storage',bucket_access_role_arn:'arn:aws:iam::123456789012:role/storage',subdirectory:'PRIVATE_REPORT_PATH'}]}],options:[{bytes_per_second:12345}]},{source_location_arn:ref('aws_datasync_location_nfs.shared.arn'),destination_location_arn:ref('aws_datasync_location_s3.shared.arn'),cloudwatch_log_group_arn:ref('aws_cloudwatch_log_group.sync.arn'),task_report_config:[{s3_destination:[{s3_bucket_arn:ref('aws_s3_bucket.shared.arn'),bucket_access_role_arn:ref('aws_iam_role.storage.arn')}]}]});
fs.writeFileSync('tests/fixtures/datasync-plan.json', JSON.stringify({format_version:'1.2',resource_changes:resources,configuration:{provider_config:{aws:{full_name:'registry.terraform.io/hashicorp/aws',expressions:{region:{constant_value:'us-east-1'},allowed_account_ids:{constant_value:['123456789012']}}}},root_module:{resources:configs}}},null,2)+'\n');
