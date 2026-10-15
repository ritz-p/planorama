# Architecture icons

## AWS Architecture Icons

Planorama embeds official AWS Architecture Icons from the **07312026 (July 31, 2026)** package downloaded from the [AWS Architecture Icons page](https://aws.amazon.com/architecture/icons/) on 2026-10-05.

The selected original SVG files are stored unchanged under `assets/icons/aws/`. `manifest.json` records the official download URL, ZIP SHA-256, original entry paths, and individual file hashes. Only 19 required service icons are bundled; the full ZIP is not committed. Git preserves asset bytes through `.gitattributes`.

`aws_sqs_queue` and `aws_sns_topic` use the official Amazon Simple Queue Service
and Amazon Simple Notification Service icons from the same verified release,
for both managed and data resources. See the [messaging example](../examples/messaging.svg).

### Rights and scope

AWS owns these assets. They are **not MIT-licensed**. The official page permits customers and partners to use the assets in architecture diagrams; the [AWS Site Terms](https://aws.amazon.com/terms/) also apply. The downloaded package contains SVG/PNG assets and no separate license document. This is not a grant of unrestricted redistribution or sublicensing rights. The selected assets support this AWS architecture diagram renderer, not a standalone icon distribution service. For other uses, consult AWS's terms or obtain permission. Planorama does not imply AWS endorsement.

`assets/icons/aws/NOTICE.txt` records ownership and source, and accompanies SVGs that use AWS icons.

### Mapping

AWS selection requires the HashiCorp AWS provider identity
`registry.terraform.io/hashicorp/aws`, or the existing `InferredAws` fallback when
provider information is entirely absent. Explicit foreign provider information
and unresolved bindings block that fallback; an `aws_` prefix alone does not
override them. See [provider identity](models.md).

Selection matches exact resource types, independently of managed/data mode,
ResourceRole and action. Managed and data entities of the same type share a service
icon. Related types intentionally use service-level icons: subnet, gateway,
routing, peering and network security resources use Amazon VPC; listeners and
target groups use Elastic Load Balancing. No public/private subnet status is
inferred from the resource type. Icon coverage does not add classification,
placement, containment or connection rules. Spatial data cards retain their
external badge and dashed border; logical data rows retain external metadata and
a dashed action underline.

### Data-source coverage

The fixture audit for this expansion found six visible AWS data-source types:
`aws_ami`, `aws_iam_role`, `aws_lb`, `aws_security_group`, `aws_subnet` and `aws_vpc`.
Each already had a service icon. The provider audit then checked the
[service-package registrations at `cc1ce533`](https://github.com/hashicorp/terraform-provider-aws/tree/cc1ce533996e43a79e87e0abf3f65fd8aaeb3b64/internal/service)
for the bundled service families: `ec2`, `elb`, `elbv2`, `lambda`, `ecs`,
`eks`, `rds`, `dynamodb`, `elasticache`, `opensearch`, `elasticsearch`,
`s3`, `s3control`, `cloudfront`, `apigateway`, `apigatewayv2`, `iam`,
`cloudwatch`, `logs` and `autoscaling`.
The 134 additional exact mappings below cover the reviewed direct infrastructure
objects and collection lookups. They reuse the 17 bundled official service icons;
no icon asset is added or modified.

| Service icon | Additional resource types |
| --- | --- |
| Amazon VPC | `aws_customer_gateway`, `aws_ec2_managed_prefix_list`, `aws_ec2_managed_prefix_lists`, `aws_ec2_network_insights_path`, `aws_ec2_transit_gateway_attachment`, `aws_ec2_transit_gateway_attachments`, `aws_ec2_transit_gateway_connect`, `aws_ec2_transit_gateway_connect_peer`, `aws_ec2_transit_gateway_dx_gateway_attachment`, `aws_ec2_transit_gateway_multicast_domain`, `aws_ec2_transit_gateway_peering_attachments`, `aws_ec2_transit_gateway_route_table_associations`, `aws_ec2_transit_gateway_route_table_propagations`, `aws_ec2_transit_gateway_route_table_routes`, `aws_ec2_transit_gateway_route_tables`, `aws_ec2_transit_gateway_vpc_attachments`, `aws_ec2_transit_gateway_vpn_attachment`, `aws_nat_gateways`, `aws_network_acls`, `aws_network_interface`, `aws_network_interfaces`, `aws_prefix_list`, `aws_route_tables`, `aws_security_groups`, `aws_subnets`, `aws_vpc_dhcp_options`, `aws_vpc_ipam`, `aws_vpc_ipam_pool`, `aws_vpc_ipam_pool_cidrs`, `aws_vpc_ipam_pools`, `aws_vpc_ipams`, `aws_vpc_peering_connections`, `aws_vpc_security_group_rule`, `aws_vpc_security_group_rules`, `aws_vpcs`, `aws_vpn_connection`, `aws_vpn_gateway` |
| Amazon EC2 | `aws_ami_ids`, `aws_ec2_capacity_block_reservation`, `aws_ec2_host`, `aws_ec2_hosts`, `aws_ec2_public_ipv4_pool`, `aws_ec2_public_ipv4_pools`, `aws_eips`, `aws_instances`, `aws_key_pair`, `aws_launch_template`, `aws_spot_datafeed_subscription` |
| AWS PrivateLink | `aws_vpc_endpoint_associations`, `aws_vpc_endpoint_service` |
| Elastic Load Balancing | `aws_alb`, `aws_alb_listener`, `aws_alb_target_group`, `aws_elb`, `aws_lbs`, `aws_lb_trust_store` |
| AWS Lambda | `aws_lambda_alias`, `aws_lambda_code_signing_config`, `aws_lambda_function_url`, `aws_lambda_functions`, `aws_lambda_layer_version` |
| Amazon ECS | `aws_ecs_clusters`, `aws_ecs_container_definition` |
| Amazon EKS | `aws_eks_access_entry`, `aws_eks_addon`, `aws_eks_clusters`, `aws_eks_node_groups` |
| Amazon RDS | `aws_db_cluster_snapshot`, `aws_db_instances`, `aws_db_parameter_group`, `aws_db_proxy`, `aws_db_snapshot`, `aws_rds_cluster_parameter_group`, `aws_rds_clusters`, `aws_rds_global_cluster`, `aws_rds_snapshots` |
| Amazon DynamoDB | `aws_dynamodb_backups`, `aws_dynamodb_table_item`, `aws_dynamodb_tables` |
| Amazon ElastiCache | `aws_elasticache_replication_group`, `aws_elasticache_serverless_cache`, `aws_elasticache_subnet_group`, `aws_elasticache_user` |
| Amazon OpenSearch Service | `aws_elasticsearch_domain` |
| Amazon S3 | `aws_s3_access_point`, `aws_s3_account_public_access_block`, `aws_s3_bucket_notification`, `aws_s3_bucket_object`, `aws_s3_bucket_object_lock_configuration`, `aws_s3_bucket_objects`, `aws_s3_bucket_policy`, `aws_s3_bucket_replication_configuration`, `aws_s3_buckets`, `aws_s3_directory_buckets`, `aws_s3_object`, `aws_s3_objects`, `aws_s3control_access_points`, `aws_s3control_multi_region_access_point`, `aws_s3control_multi_region_access_points` |
| Amazon CloudFront | `aws_cloudfront_cache_policy`, `aws_cloudfront_connection_group`, `aws_cloudfront_distribution_tenant`, `aws_cloudfront_function`, `aws_cloudfront_origin_access_control`, `aws_cloudfront_origin_access_identities`, `aws_cloudfront_origin_access_identity`, `aws_cloudfront_origin_request_policy`, `aws_cloudfront_realtime_log_config`, `aws_cloudfront_response_headers_policy` |
| Amazon API Gateway | `aws_api_gateway_api_key`, `aws_api_gateway_api_keys`, `aws_api_gateway_authorizer`, `aws_api_gateway_authorizers`, `aws_api_gateway_domain_name`, `aws_api_gateway_resource`, `aws_api_gateway_vpc_link`, `aws_apigatewayv2_apis`, `aws_apigatewayv2_vpc_link` |
| AWS Identity and Access Management | `aws_iam_access_keys`, `aws_iam_group`, `aws_iam_instance_profiles`, `aws_iam_openid_connect_provider`, `aws_iam_role_policies`, `aws_iam_role_policy_attachments`, `aws_iam_roles`, `aws_iam_saml_provider`, `aws_iam_server_certificate`, `aws_iam_user`, `aws_iam_user_ssh_key`, `aws_iam_users` |
| Amazon CloudWatch | `aws_cloudwatch_log_groups` |
| Amazon EC2 Auto Scaling | `aws_autoscaling_groups`, `aws_autoscaling_policy`, `aws_launch_configuration` |

The legacy `aws_elasticsearch_domain` lookup shares the OpenSearch Service icon;
[Amazon OpenSearch Service supports these Elasticsearch domains](https://docs.aws.amazon.com/opensearch-service/latest/developerguide/what-is.html).

Unsupported exact types still render without an icon; service prefixes do not
select a fallback symbol. Coverage follows the audited service-family registrations
at the pinned revision. Other service families, including EBS, Outposts and Client
VPN, and later provider additions require their own reviewed mappings.
Catalog, instance-type, available-version and pricing queries remain excluded,
as do token/session, execution, generated-document and general metadata helpers.
Examples are `aws_eks_cluster_auth`, `aws_iam_session_context`,
`aws_lambda_invocation`, `aws_eks_addon_version` and `aws_rds_engine_version`.
The existing visibility rules still omit `aws_region`, `aws_partition`,
`aws_caller_identity`, `aws_availability_zones` and `aws_iam_policy_document`
data sources and their incident edges.
See [AWS data visibility](aws.md#resource-roles-and-data).

The [data icon fixture](../tests/fixtures/data-icons-plan.json) and its
[CI-generated example](../examples/data-icons.svg) exercise visible data lookups
alongside managed resources and icon-free foreign-provider/unsupported types.
CI checks exact-type/provider boundaries and preserves data-specific presentation
independently of service artwork.

## Planorama icon for Terraform built-in resources

Planorama uses an original neutral state-file symbol for its Terraform built-in
provider category. Version **1.0.0**, created on 2026-10-12, is stored at
`assets/icons/terraform/terraform-builtin-24.svg` with a 24px view box. The adjacent
`manifest.json` records the Planorama source, version, date, file hash and license
scope. This independent artwork is not the HashiCorp Terraform product logo and
does not reproduce its stylized T or brand design.

The Flight Icons logo candidate was removed because permission for bundling it
had not been established. The
[HashiCorp Trademark Policy](https://www.hashicorp.com/en/trademark-policy)
distinguishes graphic-logo permission from the software license; this replacement
does not claim permission to redistribute HashiCorp logos or imply endorsement.

### License and scope

The original Planorama icon is licensed under **MIT**. The complete license and
`Copyright (c) 2026 Planorama contributors` notice are in
`assets/icons/terraform/LICENSE.txt`. The license applies to
`terraform-builtin-24.svg` and its embedded copies, not to the repository as a
whole or to AWS assets. Its terms permit copying, modification and redistribution
with the copyright and permission notices retained.

`NOTICE.txt` records the artwork's source and independent identity. SVGs using
this icon embed that notice together with the complete MIT license, so the
permission notice accompanies distributed diagrams. Runtime rendering makes no
network request.

### Mapping

Selection requires the canonical built-in source `terraform.io/builtin/terraform`,
or the narrowly defined `InferredTerraform` fallback when provider information is
entirely absent. Explicit provider information takes precedence; unresolved
bindings, `hashicorp/terraform`, `builtin/terraform`, and foreign hosts do not
select the built-in provider icon. See [provider identity](models.md).

| Resource type | Example address | Shared icon |
| --- | --- | --- |
| `terraform_data` | `terraform_data.example` | `ResourceIcon::Terraform` |
| `terraform_remote_state` | `data.terraform_remote_state.example` | `ResourceIcon::Terraform` |

Both types use the symbol ID `planorama-icon-terraform`. Module paths and instance
keys do not alter selection. The icon is independent of managed/data mode,
ResourceRole and action; spatial data cards keep their external marker and dashed
border, while logical data rows retain external metadata and a dashed underline.
Unsupported resource types keep their existing icon-free rendering. This mapping
adds no AWS classification, containment or connection rules.

## Shared rendering

The build script parses each bundled SVG through the same allowlisted parser and
emits reusable inline symbols. Original geometry, groups, view boxes, gradients,
clip paths and masks are preserved; internal IDs and local references are
deterministically namespaced per symbol. The allowlist rejects scripts, event
handlers, CSS, images, external references, duplicate IDs and unresolved local
references at build time. Rust's `include_str!` embeds the validated symbols, so
no XML parser or asset files are needed at runtime.

Repeated resources share their icon symbol. Spatial cards and container headers
display icons at 24px beside the selected full address, preserving the existing
icon gap and heading measurements. Logical-resource groups place the same 24px
symbol in the upper-right corner of each resource row and reserve address-heading
space for it.
Notices are selected by the artwork actually used;
Terraform-only output does not receive an AWS ownership notice, and mixed output
retains both. Named-state panels namespace IDs and local fragment references.
Action colors, external markers, metadata and resource identity remain separate
from the artwork.

## Updating sources

For AWS, download a release directly from AWS and select the original service
SVGs. Preserve their original colors and paths, and update the manifest, notice
and source dates together. Review changed license, trademark and brand conditions.

For Planorama's built-in provider icon, update the original SVG and its version,
hash and source date in the manifest together. Retain the asset's MIT license and
notice, and keep the artwork independent of HashiCorp's product logos. Introducing
third-party artwork requires a separate source and rights review before bundling.

Verify source hashes and inspect SVGs for external dependencies or executable
content.
Regenerate examples locally with `scripts/update-examples.sh`, inspect the diff,
and commit the affected outputs. The [manual SVG workflow](ci.md) can also recover
snapshots; ordinary CI runs formatting, tests and Clippy. Original icon assets
are not generated outputs.
