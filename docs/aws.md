# AWS architecture relationships

AWS の共通契約とサービス別の証拠条件の正本です。型ごとの現行分類は [型ごとの対応確認](resource-support.md)、対応の限界・未対応項目は [監査ガイド](aws-semantic-audit.md) を参照してください。

## 読む範囲

共通契約は [役割と data](#resource-roles-and-data)、[deployment scope](#deployment-scopes)、[包含](#containment)、[SG 接続](#security-group-attachments)、[Association](#association-resources) です。サービス固有の型・値・scope・lifecycle 条件を合わせて確認してください。共通ルールから未記載のサービス対応を推測しないでください。

- [Compute and containers](aws-compute.md)
- [Network and edge](aws-network.md)
- [Databases and analytics](aws-data.md)
- [Storage and backup](aws-storage.md)
- [Identity and security](aws-security.md)
- [APIs and messaging](aws-messaging.md)
- [DynamoDB](dynamodb.md)、[Route](routes.md)、[暗号化表示](encryption.md) は既存の専用契約を参照。

## HTML example generation

For the following examples, use the example basename as `name`; inputs are
`tests/fixtures/<name>-plan.json` and outputs `examples/<name>.html`. Run from the
repository root with the development container running:

```sh
name=lambda
docker compose exec -T dev cargo run --locked -- "tests/fixtures/$name-plan.json" --format html -o "examples/$name.html"
```


For SVG generation and prerequisites, see [CONTRIBUTING](../CONTRIBUTING.md).

## Dedicated reference proof

For the pinned Provider 6.70.0 contracts, dedicated configured links require
unique explicit Terraform provenance, matching known non-sensitive values,
correct endpoint output attributes, compatible provider/account/region and
current endpoints. Collections must be complete and element-correlated.
Literal equality, dynamic selection and partial or ambiguous evidence do not
prove links. Current configuration cannot prove deleted/deposed relationships;
filtering must not restore endpoints or borrow another state's identity.
Service sections specify mode and scope exceptions. Legacy relationship rules
may have narrower proof: do not generalize a new service's guarantees to them.

## Resource roles and data

AWS rules in `src/provider/aws` dispatch by provider source and exact type; custom
provider lookalikes receive no AWS semantics. See [provider identity](models.md).
The six [verified `aws_alb*` aliases](aws-inventory.md#verified-resource-aliases)
reuse canonical `aws_lb*` rules within their mode limits, retaining Terraform identities.
The [managed inventory](aws-inventory.md) covers registered Provider 6.70.0 managed
types with basic defaults only; dedicated relationships and containment require
the service-specific proofs below.

VPC/Subnet are Spatial `Container` resources. Other types use dedicated
classification or the [basic capability table](aws-inventory.md); unknown types
stay visible as `Unknown`. Role is independent of managed/data mode and placement:
SGs remain Policies and target groups Controllers even inside VPCs.

| Role | Meaning and limit |
| --- | --- |
| Identity | Principal, collection or federation configuration; no authentication, assumption, effective-permission or identity-equivalence claim |
| Policy | Access/control rules or policy-like configuration; not necessarily IAM or enforcement |
| Controller | Manages/configures services, workloads or resources; no runtime control, traffic or relationship proof |

Logical type cards explain each role once, separately from action/mode. Action
subsections change presentation only. The [matrix](resource-support.md) records
defaults; [presentation policy](resource-presentation.md) defines ownership/selection.
[SSM Parameters, metric alarms and five IAM Identity types](purpose-placement.md)
stay Logical unless an allowed [`--spatial-type`](configuration.md#explicit-spatial-types)
selects independent cards with reference details retained. Focus alone never changes placement.

Logical rows group by provider source/type/mode in the top annotation area and
retain identity, actions, operation metadata, scope and filter participation.
AMI is an image definition, not a deployed instance. Provider-aware rules cover
managed/data resources; the basic inventory extends managed types only, not custom
providers. Unknown types stay spatial with unresolved placement; missing subnet/VPC
references do not imply Logical placement. Names never merge state, mode or lifecycle identities.

Data resources retain roles/external identities: Spatial cards use dashed borders
and an external badge; Logical rows use `data / external` groups and dashed action
underlines. The matrix's omitted AWS metadata sources lose their cards/incident
edges but remain reference provenance. Cached prior-state data remains when still
configured. See [data.svg](../examples/data.svg).

## Deployment scopes

AWS account scope is resolved independently from region using verified saved-plan
provider evidence; JSON alone leaves accounts unknown. See [the input contract](provider-evidence.md). State IDs and provider
aliases are not accounts. Logical-resource scope labels include a known
`AWS Account: <12-digit ID>`; generated SVG/HTML therefore expose that deployment
identifier. Role ARNs and credential-related provider values are not retained.

Known accounts form `AWS Account: <ID>` backgrounds above Region/AWS Global.
One displayed Spatial resource suffices; equal proven accounts share panels,
while equal regions in different accounts stay separate. Known account/unknown
region stays directly under Account; unknown account/known region gets only Region.
Logical rows stay in the annotation area. Account frames precede child frames;
both headers obstruct routing and their geometry affects state/cross-state sizing.
`data-account-panel` identifies account frames; child `data-account` identifies
the parent within its state. JSON-only [accounts](../examples/accounts.svg) and
[combined examples](../examples/multi-account/README.md) retain unknown accounts.

Static proven regions group eligible Spatial resources after filtering
([example](../examples/regions.svg)); removing the last removes its panel/title/padding.
Logical rows, synthetic summaries and relationship-index entries create no panels.
These backgrounds are deployment scopes, not Terraform resources/semantic containers:
VPC/Subnet containment stays intact and cross-region routes draw above them.
Spatial IAM, CloudFront and supported Route 53 share `AWS Global` when region panels
exist or account is known. IAM Role/Policy/Role Policy/Instance Profile keep global
metadata as Logical rows outside panels. `provider/aws/scope.rs` classifies scopes;
layout/SVG consume them generically. Peering connections retain requester control scope; their verified peer references
may cross account/region boundaries. Unsupported types and
unknown regions stay outside regional panels (inside proven accounts if available).
Aliases, names, tags, ARNs and IDs never establish region.

Each Logical Resources row displays its own `Region: <region>`, `AWS Global`,
or `Region: unknown` label in SVG and HTML. Equal type/mode resources from different
regions stay in one group without a shared region claim. See the maintained
[logical-region example](../examples/logical-regions.svg), generated from
[this fixture](../tests/fixtures/logical-regions-plan.json).
Static resource-level `region` overrides are supported for managed `aws_vpc`,
`aws_subnet` and `aws_cloudwatch_log_group`, verified with Terraform **1.9.8** or
**1.16.6** and HashiCorp AWS **6.0.0**. The configuration must pin
`version_constraint` to `6.0.0`
(optionally prefixed by `= `); a range, missing version or another version does
not prove this schema. AWS v5 fixtures do not support these overrides.
These are exact Core versions, not a minimum-version range. A JSON constraint is
not a selected-version/lock attestation: only the exact AWS 6.0.0 constraint
matches the schema contract checked with real locked plans. The
[compatibility matrix](terraform-compatibility.md) records both Core captures;
other Core versions and Provider releases remain unverified.
The reference-free literal must equal the unique current resource change's known,
non-sensitive `after.region`. Initial accepted literals are `us-east-1`, `us-east-2`,
`us-west-1`, `us-west-2`, `eu-west-1`, `eu-west-2`, `eu-west-3`, `eu-central-1`,
`ap-northeast-1`, `ap-northeast-2`, `ap-southeast-1` and `ap-southeast-2`.
See the [captured evidence](../tests/fixtures/aws-region-overrides/README.md) and
[maintained example](../examples/region-overrides.svg).

A verified override takes precedence over the provider region. An absent override
uses the existing provider fallback; an explicit dynamic, unknown, invalid,
conflicting or unsupported override stays unknown without fallback. Ordinary
attributes such as data `aws_region` or `aws_vpc_endpoint_service.region` do not
establish deployment scopes. Data overrides are not resolved in this initial scope.
Current create/update/no-op/replacement objects use after evidence; deleting
objects with this verified provider version stay region-unknown, including when
the current configuration has no override. Deposed objects never borrow current
configuration. Before values alone are not accepted as override proof.
The provider's original region, source, configuration identity and independent
account evidence remain intact; resolving a region does not establish an account.
Only the accepted scope value is retained, not arbitrary resource attributes.
Missing/dynamic bindings and unsupported types remain unknown. Global classification is
explicit and independent of provider region; policy conditions such as
`aws:RequestedRegion` never determine deployment scope.

Logical Components display an aggregate of established member scopes plus each
member's label. Mixed regions/accounts and incomplete evidence remain explicit;
see [component summaries](components.md). Synthetic entities acquire no scope panel.

A containment subtree is grouped only when all members have the same known scope.
Conflicting or unknown members leave the whole subtree outside panels, preserving
containment without suggesting a guessed account or region. Plans without a groupable account or regional
subtree keep their existing layout. Synthetic component summaries remain separate.
Named-state sections retain independent panels even for equal region names;
`data-deployment-scope`, `data-scope-provider`, `data-region` or `data-global`
identify panel metadata within their enclosing `data-state-id`.
The [regional-state example](../examples/regional-states.svg) shows independent
`ap-northeast-1` bases for application, network and a single-resource state.
Its inputs explicitly bind resources to provider configurations with literal
regions. A provider alias such as `archive` is not the region label; missing or
dynamic provider-region metadata cannot produce a guessed panel.

## Containment

Listener cards prefer nearby positions to their uniquely referenced managed LB.
This uses complete, unambiguous `load_balancer_arn` reference evidence and visible,
current AWS identities; missing, dynamic, foreign-provider or duplicate candidates
receive no preference. It creates no containment. Target Group/Rule relationships
and the independent Logical Component summary are retained. See [placement](routing.md).

Direct containment requires a complete, static reference to one endpoint of the
expected AWS type. Managed children can use data-source parents. Tags, names,
literal IDs, dynamic selection and ambiguous references do not establish a parent;
data-source search conditions do not create containment.

| Child | Attribute | Parent |
| --- | --- | --- |
| Subnet, VPC Endpoint, Route Table, Security Group, Network ACL, LB Target Group | `vpc_id` | VPC |
| EC2 Instance, NAT Gateway | `subnet_id` | Subnet |
| EFS Mount Target | `subnet_id` with known ID proof | Managed Subnet |

Target groups can omit `vpc_id`, including Lambda target groups; omission does not
produce a missing-attribute diagnostic. See [network-scope.svg](../examples/network-scope.svg).

Subnet-based placement uses complete, static AWS subnet references and existing
containment ancestry. Mixed VPCs, missing ancestry, cycles and ambiguous parents
do not invent a common container.

| Resource | Reference path | Placement |
| --- | --- | --- |
| Load Balancer | `subnets`; verified `subnet_mapping.subnet_id` | Single subnet or nearest common ancestor |
| ECS Service | `network_configuration.subnets` | Single subnet or nearest common ancestor |
| RDS Instance / Cluster | `db_subnet_group_name` → group's `subnet_ids` | Single subnet or nearest common ancestor |
| DB / ElastiCache Subnet Group | `subnet_ids` | Common VPC, including a single-subnet group |
| ElastiCache Replication Group | `subnet_group_name` → ElastiCache Subnet Group | Same verified VPC as the subnet group |
| Lambda / EKS | `vpc_config.subnet_ids` | Common VPC |
| EKS Node Group / Fargate Profile | `subnet_ids` | Verified current Subnets and common VPC; independent of cluster membership |
| RDS Proxy | `vpc_subnet_ids` | Verified current Subnets and common VPC; independent of DB targets |
| ElastiCache Serverless | `subnet_ids` | Verified current Subnets and common VPC; no provisioned nodes/members |
| OpenSearch | `vpc_options.subnet_ids` | Common VPC |

LB/ECS subnet references outside the chosen parent become Connection edges.
Subnet groups, RDS and the other workloads retain their original dependencies.
ElastiCache subnet and replication groups are supported visible resources with
AWS account/region scope and the ElastiCache service label. Replication placement requires
a complete, static reference to exactly one AWS ElastiCache subnet group with
the same provider-configuration identity and deployment scope and verified VPC
placement; unresolved, dynamic or incompatible references stay
visible without inferred placement. See the [ElastiCache example](../examples/elasticache.svg).
Nested fields are resolved separately: flattened block references or sibling SG
fields cannot establish subnet membership. Derived containment retains the reference
chain and receives the [inferred-placement annotation](relationships.md).

Examples: [multiple subnets](../examples/multi-container.svg),
[RDS](../examples/rds-subnet-group.svg), [subnet groups](../examples/subnet-groups.svg),
[VPC workloads](../examples/vpc-workloads.svg).

## Security Group attachments

RDS Proxy `vpc_security_group_ids` and ElastiCache Serverless `security_group_ids`
use the strict current-resource and known-value checks in
[RDS Proxy](aws-data.md#rds-proxy) and [ElastiCache Serverless](aws-data.md#elasticache-serverless) before
adding SG connections.

| Consumer | Attribute |
| --- | --- |
| EC2, RDS Instance / Cluster / Proxy | `vpc_security_group_ids` |
| Load Balancer, EFS Mount Target | `security_groups` |
| ECS Service | `network_configuration.security_groups` |
| VPC Endpoint, ElastiCache Serverless | `security_group_ids` |
| Lambda / EKS | `vpc_config.security_group_ids` |
| OpenSearch | `vpc_options.security_group_ids` |

Each explicitly referenced AWS SG becomes a Connection to the managed consumer.
Multiple groups and exact indexed references are supported, including external
SG data sources. Attachment does not change containment or hide SG cards.
EFS Mount Targets, RDS Proxies and ElastiCache Serverless additionally require
current managed SG endpoints and reject deleted consumers/endpoints; other
consumers retain their existing data-SG rules.

The entire planned ID collection must match the referenced resources' IDs using
planned values/unknown slots, with prior state available for cached data sources.
Only the boolean proof survives ingestion. Missing evidence, mixed literal lists,
unknown collection length, partial/dynamic/ambiguous references and wrong endpoint
types/providers remain dependencies. Optional absent attributes are not errors.
EC2's name-based `security_groups` and ECS task definitions are unsupported.
These relationships do not describe firewall rules or reachability.

Examples: [security-groups.svg](../examples/security-groups.svg),
[workload-security-groups.svg](../examples/workload-security-groups.svg).

## ELBv2 certificates and trust stores (#908)

Listeners reference ACM or IAM server certificates by explicit ARN reference and
matching known value; existing ACM validation `certificate_arn` references remain
supported. Listener-certificate helpers additionally prove the listener
ARN; helpers with only those two dependencies become associations with original
actions and provenance. Otherwise they remain Logical resources. Certificates stay
independent, and Global IAM scope never changes the listener's regional scope.

A single complete `mutual_authentication` list element must have known `mode=verify`
and a matching explicit ELBv2 trust-store `arn`/`id` reference. Other modes, incomplete
lists and CloudFront trust stores do not prove this relationship. Existing listener
proximity and forward ownership are unchanged.

Trust stores and revocations can reference an S3 bucket by `bucket`/`id`. CA-bundle
and revocation object provenance additionally requires an explicit managed
`aws_s3_object` or `aws_s3_bucket_object` `key` reference, matching bucket and key,
and, when configured, a `version_id` reference to the same object. An omitted version
means latest and does not identify a pinned version. A literal version or mismatched
object cannot prove object provenance; independent bucket proof remains available.

These relationships require compatible provider/account/region, unique current
managed endpoints and known non-sensitive values. Shared resources remain visible;
unknown, dynamic, data, deleted/deposed and filtered endpoints remain conservative.
No certificate contents, S3 payloads, effective revocation, authentication, traffic,
network containment or arbitrary attribute summaries are inferred. The contracts
follow AWS Provider 6.70.0 schemas and official sources in the semantic decision ledger.

## ELBv2 Cognito authentication references (#909)

Listeners and listener rules, including `aws_alb_*` aliases, support
`default_action.authenticate_cognito` and `action.authenticate_cognito` references
when the corresponding action has known `type=authenticate-cognito`:

| Input | Explicit endpoint output |
| --- | --- |
| `user_pool_arn` | `aws_cognito_user_pool.arn` |
| `user_pool_client_id` | `aws_cognito_user_pool_client.id` |
| `user_pool_domain` | `aws_cognito_user_pool_domain.domain` |

The action lists must match configuration lengths and positions, and each
authentication block must be a singleton list. Every participating element must
resolve uniquely to a current managed endpoint with matching known, non-sensitive
values and compatible provider/account/region in the same named state. An unproven
element keeps that entire field's collection conservative. Pool, client and domain
remain independent resources; these references do not establish ownership or
network containment. Existing forwarding, listener components and TLS rules still
apply independently.

Literal strings, OIDC/JWT/redirect attributes, conditions and dynamic, unknown,
sensitive, deleted/deposed or filtered references do not prove Cognito endpoints.
Session settings, secrets, credentials and effective authentication are not shown.
The contract follows AWS Provider 6.70.0's listener and listener-rule list schemas.

## Association resources

| Resource | Endpoint attributes | Relationship |
| --- | --- | --- |
| `aws_route_table_association` | exactly one `subnet_id` / `gateway_id`, `route_table_id` | Subnet or Internet/VPN Gateway ↔ Route Table |
| `aws_main_route_table_association` | `vpc_id`, `route_table_id` | VPC ↔ Main Route Table |
| `aws_network_interface_attachment` | `network_interface_id`, `instance_id` | ENI ? EC2 Instance |
| `aws_network_interface_sg_attachment` | `network_interface_id`, `security_group_id` | ENI ? Security Group |
| `aws_volume_attachment` | `instance_id`, `volume_id`, with known ID proof | EC2 Instance ↔ EBS Volume |
| `aws_lb_target_group_attachment` | `target_group_arn`, `target_id` | Target Group ↔ EC2 Instance / Lambda Function / Application Load Balancer |
| `aws_lb_listener_certificate` | `listener_arn`, `certificate_arn` | Listener ↔ ACM / IAM Server Certificate |
| `aws_vpc_endpoint_route_table_association` | `vpc_endpoint_id`, `route_table_id` | VPC Endpoint ↔ Route Table |

Target-group attachments (including verified `aws_alb_*` aliases) require unique
explicit managed references and matching known evaluated values. The group's known
`target_type` selects an instance `id`, Lambda function `arn`, or application LB
`arn`; the group reference accepts its `arn` or ARN-valued `id`. Provider binding,
account and region must be compatible. Unknown/sensitive values, deleted/deposed
objects, data sources, dynamic selection and IP targets do not prove an attachment.
An omitted target type is not assumed to be `instance` without its evaluated value.
Extra dependencies or consumers retain the Logical helper and its verified references.
Unproven attachments remain in the Logical Resources section, not workload cards.
Attachments do not establish LB ownership or network containment, and computed
`load_balancer_arns` is not ownership evidence. Port/AZ values do not establish
traffic, health or reachability. The contract follows the pinned AWS Provider
6.70.0 target-group and attachment schemas recorded in the semantic decision ledger.

These managed helper cards become Association edges only when both endpoints are
statically and uniquely resolved and the helper has exactly those two incident
dependencies. Extra consumers/dependencies, data helpers and unsupported target
types leave the card visible. Every lowered edge retains its source address,
action and lifecycle metadata. Parallel associations remain distinct.

Route-table associations additionally require matching known endpoint values,
compatible scope, current managed endpoints and common known VPC context.
Deleted helpers remain cards. See the [typed route contracts](routes.md#typed-route-references-897).

`aws_route` remains a graph node and uses verified [Route Table child-card ownership](routes.md).
Other Association-role resources remain cards. Examples:
[association.svg](../examples/association.svg), [aws-relationships.svg](../examples/aws-relationships.svg).

## ENI and Instance Connect endpoints (#890)

`aws_network_interface` and `aws_ec2_instance_connect_endpoint` are independent
spatial connectors. Required scalar `subnet_id` references must uniquely identify
an `aws_subnet.id` with the same known evaluated value. Attachment never changes
this placement. Inline `security_groups` (ENI) and `security_group_ids` (Instance
Connect) are optional/computed sets: only explicitly configured, complete sets
matching managed `aws_security_group.id` outputs qualify. Each SG must share the
known VPC of the explicitly referenced subnet. A known Instance Connect `vpc_id`
must agree; computed `network_interface_ids` never create managed children.

ENI `attachment` is an optional/computed set. A single complete configured entry
may reference `aws_instance.id` through `attachment.instance`; required
`device_index` must be a known nonnegative integer. Optional/computed
`network_card_index` may be absent, otherwise it must be known and nonnegative.
Multiple entries conservatively retain generic references. The instance and ENI
remain independent. `aws_network_interface_attachment` requires both scalar
`instance_id` and `network_interface_id` references to matching managed `id`
outputs and the same index checks before lowering to an association.
`attachment_id` identifies the attachment, never the ENI.

`aws_network_interface_sg_attachment` requires the complete explicit ENI/SG pair,
compatible provider/account/region and matching known subnet/SG VPC. It does not
merge either resource or reinterpret inline security groups.
`aws_network_interface_permission.network_interface_id` may establish one ENI
owner; `aws_account_id` is the grantee, not source account evidence or an IAM
endpoint. Permission effects remain opaque. All new paths reject unknown or
sensitive values, deleted/deposed or data endpoints, incomplete and ambiguous
references. Named states and filtering retain their independent identities.
No IP, CIDR, DNS, service-created output, SSH reachability or runtime-health
inference is performed. `ip_address_type` does not change these relationships.

These shapes and flags were checked against AWS Provider 6.70.0, commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`, in the EC2 resource schemas and their
[official resource documentation](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r).

## ELBv2 subnet mappings and destinations (#907)

Managed `aws_lb` and verified `aws_alb` aliases support complete configured
`subnet_mapping` sets. Each `subnet_id` must reference one managed `aws_subnet.id`
with a matching known value, compatible scope and the same known VPC as the other
mapped subnets. Configuration elements are correlated by unique subnet IDs rather
than evaluated set order. Mapping relationships use the existing spanning
placement rules; legacy `subnets` and `security_groups` contracts are unchanged.
Both input forms configured together, partial sets, ambiguous elements and
unknown/sensitive endpoint values retain conservative fallback.

For a known internet-facing `network` LB, each configured mapping `allocation_id`
must reference a managed `aws_eip.allocation_id` in the same mapping element and
scope, with known EIP `domain=vpc`. `id`, `association_id`, literal allocation IDs
and computed service-created interfaces do not prove this relationship. EIPs
remain independent. Optional allocations may be absent on other mapped subnets.

An `application` LB's singleton `ipam_pools.ipv4_ipam_pool_id` may reference
`aws_vpc_ipam_pool.id`. Singleton `access_logs`, `connection_logs` and
`health_check_logs` blocks require known `enabled=true` and an explicit bucket
reference matching `aws_s3_bucket.bucket` or `id`. Access logs accept application
or network LBs; connection and health-check logs accept application LBs only.
No omitted default, unknown flag, prefix, policy or ARN-string search establishes
a log destination. Shared pools/buckets keep independent identities and scope.

All new relationships require current managed endpoints, complete unique explicit
references and compatible provider/account/region. Filtering, named-state identity,
deleted/deposed objects, sensitive values and data-mode boundaries remain intact.
No health, traffic, successful log delivery or effective permission is inferred.
Contracts follow AWS Provider 6.70.0 schema and `load_balancer.go` at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`.
