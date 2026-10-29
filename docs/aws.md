# AWS architecture relationships

AWS の共通契約とサービス別の証拠条件の正本です。型ごとの現行分類は [生成一覧](resource-support.md)、対応の限界・未対応項目は [監査ガイド](aws-semantic-audit.md) を参照してください。

## 読む範囲

共通契約は [役割と data](#resource-roles-and-data)、[deployment scope](#deployment-scopes)、[包含](#containment)、[SG 接続](#security-group-attachments)、[Association](#association-resources) です。サービス固有の型・値・scope・lifecycle 条件を合わせて確認してください。共通ルールから未記載のサービス対応を推測しないでください。

- [Compute and containers](aws-compute.md): Batch compute environments and queues / Lambda aliases, layers and function URLs / App Runner networking / ECR repositories / ECS Cluster membership / EKS configuration and membership / Auto Scaling configuration / Workload references and monitoring
- [Network and edge](aws-network.md): Network Firewall / CloudFront origins and policies / WAFv2 / Route 53 / PrivateLink endpoints and services / VPC gateways and Elastic IPs / Security group rule relationships
- [Databases and analytics](aws-data.md): OpenSearch Serverless / RDS Global Clusters / Redshift provisioned clusters / Athena Workgroup and Data Catalog / Glue Catalog, Crawler and Job / Aurora cluster instances / RDS Proxy / ElastiCache configuration references / ElastiCache Serverless / DocumentDB / Neptune / DMS replication configuration / Redshift Serverless / SageMaker inference configuration
- [Storage and backup](aws-storage.md): S3 bucket configuration / AWS Backup / EFS / EBS / FSx Lustre networking and encryption
- [Identity and security](aws-security.md): AWS Private CA / Cognito / CloudTrail / IAM users and groups / IAM identity providers / SSM Parameter Store / Secrets Manager / KMS / ACM / Organizations hierarchy / Organizations policies and direct attachments / Organizations member Account deployment navigation
- [APIs and messaging](aws-messaging.md): API Gateway integrations and deployment context / Step Functions / EventBridge buses and rules / SQS and SNS / Amazon MQ network and configuration / MSK network and configuration / Scheduler and Pipes configuration / SNS subscriptions / Lambda event source mappings / Kinesis Data Streams / Amazon Data Firehose / EventBridge targets
- [DynamoDB](dynamodb.md)、[Route](routes.md)、[暗号化表示](encryption.md) は既存の専用契約を参照。

## Resource roles and data

VPC and Subnet are spatial `Container` resources. Other supported types are
classified as nodes, connectors, associations, identities, policies or controllers by
the dedicated classification rules or the [basic capability table](aws-inventory.md).
Unknown types remain visible as `Unknown`. Roles are separate
from managed/data mode; Security Groups remain Policy cards and target groups
remain Controller cards even when placed inside a VPC.

Identity describes principals, identity collections or federation configuration;
it proves neither authentication, role assumption nor effective permissions.
It does not establish equivalence between users or accounts: state, managed/data
and current/deposed identities remain separate even when display names match.
Policy describes access/control rules or policy-like configuration; it does not
necessarily mean IAM or policy enforcement. Controller describes resources that
manage or configure services, workloads or related resources; the role alone does
not establish runtime control, traffic or relationships. Logical type cards show
these explanations once per role, separately from Terraform action and managed/data
mode. Their action subsections change presentation only, not role or placement.

Placement is independent of role. The [generated matrix](resource-support.md)
records current behavior; the [presentation policy](resource-presentation.md)
defines ownership, selection and conditional child presentation.
The [purpose-dependent review](purpose-placement.md) retains Logical defaults for
SSM Parameters, metric alarms and the five IAM Identity types.
The [explicit generation-time option](configuration.md#explicit-spatial-types)
displays these types as independent Spatial cards while retaining reference details.
The matrix
lists the explicit logical types. They render as rows grouped by provider source,
resource type and mode in the top annotation area. AMI describes
an image definition used by workloads, not a deployed compute instance. These
existing provider-aware rules apply to managed and data resources; the basic
inventory extends managed resources only. Custom providers do
not inherit them. Unknown types retain unresolved placement and stay visible in
the spatial diagram. Missing VPC/subnet references never imply logical placement.
Logical placement does not suppress architectural significance: individual resource
rows retain actions, identities, operation metadata, scope and focus/filter
participation. Focus selects resources without changing their placement;
independent Spatial cards require an explicit allowed `--spatial-type`.

Infrastructure data sources retain their roles and external identity. Spatial
cards use an external badge and dashed border; logical resources use separate
`data / external` groups and dashed action underlines. The matrix marks omitted
AWS metadata data sources; their incident edges are also omitted.
References from retained resources keep
the omitted source identities as provenance, without creating cards or paths.
Cached data resources in prior state are
included when still present in configuration. See [data.svg](../examples/data.svg).

## Deployment scopes

AWS account scope is resolved independently from region using verified saved-plan
provider evidence; JSON alone leaves accounts unknown. See [the input contract](provider-evidence.md). State IDs and provider
aliases are not accounts. Logical-resource scope labels include a known
`AWS Account: <12-digit ID>`; generated SVG/HTML therefore expose that deployment
identifier. Role ARNs and credential-related provider values are not retained.

Known accounts form `AWS Account: <ID>` backgrounds above Region and AWS Global,
when evidence is available. The JSON-only [accounts.svg](../examples/accounts.svg)
demonstrates unknown accounts retaining known regions. One displayed spatial
resource is enough for either level. Multiple bindings with equal proven accounts
share a panel, while the same region under different accounts stays separate.
Known account/unknown region resources remain directly inside Account; unknown
account/known region resources retain their Region panel. Unknown accounts never
receive guessed wrappers. Logical rows remain in the annotation area.
Account backgrounds are drawn before their child backgrounds, and both headers
are routing obstacles. Their final geometry participates in state sizing and
cross-state routing. `data-account-panel` identifies the account frame;
`data-account` on child frames records its parent identity within the state.
The [combined large example](../examples/multi-account/README.md) exercises
JSON-only region/global scopes with VPC/Subnet containment, logical annotations
and cross-state routing; its accounts remain unknown.

Known static provider regions automatically group explicitly supported regional
resources into background panels (see [regions.svg](../examples/regions.svg)).
One displayed spatial resource is sufficient, including when the state has only
one known region. Eligibility is evaluated per state and region after filtering;
removing the last spatial resource removes the panel and its title/padding.
Logical-resource rows, synthetic summaries and relationship-index entries do not
create panels. Panels are deployment scopes, not Terraform resources or semantic
containers. VPC/Subnet containment stays intact inside each panel, and
cross-region relationships remain routed paths above the background.
Spatial IAM resources, CloudFront and supported Route 53 resources share an
`AWS Global` panel when regional panels are present or their account is known. IAM Role, Policy, Role Policy
and Instance Profile retain global scope metadata but render as logical-resource
rows above the spatial diagram, outside deployment-scope panels. AWS scope
classification lives in `provider/aws/scope.rs`; layout and SVG consume generic
deployment scopes. Unsupported resource types, cross-region peering resources and
unknown regions stay outside regional panels (but inside a proven account). No aliases, names, tags, ARNs or IDs supply
region information.

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
| Load Balancer | `subnets` | Single subnet or nearest common ancestor |
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

## Association resources

| Resource | Endpoint attributes | Relationship |
| --- | --- | --- |
| `aws_route_table_association` | exactly one `subnet_id` / `gateway_id`, `route_table_id` | Subnet or Internet/VPN Gateway ↔ Route Table |
| `aws_main_route_table_association` | `vpc_id`, `route_table_id` | VPC ↔ Main Route Table |
| `aws_volume_attachment` | `instance_id`, `volume_id`, with known ID proof | EC2 Instance ↔ EBS Volume |
| `aws_lb_target_group_attachment` | `target_group_arn`, `target_id` | Target Group ↔ EC2 Instance |
| `aws_vpc_endpoint_route_table_association` | `vpc_endpoint_id`, `route_table_id` | VPC Endpoint ↔ Route Table |

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

## 旧見出しからの参照

既存の外部リンク用の案内です。仕様の本文はリンク先だけで管理します。

### Batch compute environments and queues (#700)

[Batch compute environments and queues (#700)](aws-compute.md#batch-compute-environments-and-queues-700) を参照してください。

### OpenSearch Serverless (#696)

[OpenSearch Serverless (#696)](aws-data.md#opensearch-serverless-696) を参照してください。

### Network Firewall (#693)

[Network Firewall (#693)](aws-network.md#network-firewall-693) を参照してください。

### API Gateway integrations and deployment context

[API Gateway integrations and deployment context](aws-messaging.md#api-gateway-integrations-and-deployment-context) を参照してください。

### Lambda aliases, layers and function URLs

[Lambda aliases, layers and function URLs](aws-compute.md#lambda-aliases-layers-and-function-urls) を参照してください。

### RDS Global Clusters

[RDS Global Clusters](aws-data.md#rds-global-clusters) を参照してください。

### CloudFront origins and policies

[CloudFront origins and policies](aws-network.md#cloudfront-origins-and-policies) を参照してください。

### WAFv2

[WAFv2](aws-network.md#wafv2) を参照してください。

### S3 bucket configuration

[S3 bucket configuration](aws-storage.md#s3-bucket-configuration) を参照してください。

### AWS Private CA

[AWS Private CA](aws-security.md#aws-private-ca) を参照してください。

### Cognito

[Cognito](aws-security.md#cognito) を参照してください。

### Step Functions

[Step Functions](aws-messaging.md#step-functions) を参照してください。

### Redshift provisioned clusters

[Redshift provisioned clusters](aws-data.md#redshift-provisioned-clusters) を参照してください。

### CloudTrail

[CloudTrail](aws-security.md#cloudtrail) を参照してください。

### AWS Backup

[AWS Backup](aws-storage.md#aws-backup) を参照してください。

### Athena Workgroup and Data Catalog

[Athena Workgroup and Data Catalog](aws-data.md#athena-workgroup-and-data-catalog) を参照してください。

### Glue Catalog, Crawler and Job

[Glue Catalog, Crawler and Job](aws-data.md#glue-catalog-crawler-and-job) を参照してください。

### IAM users and groups

[IAM users and groups](aws-security.md#iam-users-and-groups) を参照してください。

### IAM identity providers

[IAM identity providers](aws-security.md#iam-identity-providers) を参照してください。

### EFS

[EFS](aws-storage.md#efs) を参照してください。

### EBS

[EBS](aws-storage.md#ebs) を参照してください。

### SSM Parameter Store

[SSM Parameter Store](aws-security.md#ssm-parameter-store) を参照してください。

### Secrets Manager

[Secrets Manager](aws-security.md#secrets-manager) を参照してください。

### KMS

[KMS](aws-security.md#kms) を参照してください。

### ACM

[ACM](aws-security.md#acm) を参照してください。

### Route 53

[Route 53](aws-network.md#route-53) を参照してください。

### EventBridge buses and rules

[EventBridge buses and rules](aws-messaging.md#eventbridge-buses-and-rules) を参照してください。

### Aurora cluster instances

[Aurora cluster instances](aws-data.md#aurora-cluster-instances) を参照してください。

### SQS and SNS

[SQS and SNS](aws-messaging.md#sqs-and-sns) を参照してください。

### Amazon MQ network and configuration (#689)

[Amazon MQ network and configuration (#689)](aws-messaging.md#amazon-mq-network-and-configuration-689) を参照してください。

### MSK network and configuration (#688)

[MSK network and configuration (#688)](aws-messaging.md#msk-network-and-configuration-688) を参照してください。

### Scheduler and Pipes configuration (#692)

[Scheduler and Pipes configuration (#692)](aws-messaging.md#scheduler-and-pipes-configuration-692) を参照してください。

### SNS subscriptions

[SNS subscriptions](aws-messaging.md#sns-subscriptions) を参照してください。

### Lambda event source mappings

[Lambda event source mappings](aws-messaging.md#lambda-event-source-mappings) を参照してください。

### Kinesis Data Streams

[Kinesis Data Streams](aws-messaging.md#kinesis-data-streams) を参照してください。

### Amazon Data Firehose

[Amazon Data Firehose](aws-messaging.md#amazon-data-firehose) を参照してください。

### EventBridge targets

[EventBridge targets](aws-messaging.md#eventbridge-targets) を参照してください。

### App Runner networking (#694)

[App Runner networking (#694)](aws-compute.md#app-runner-networking-694) を参照してください。

### ECR repositories

[ECR repositories](aws-compute.md#ecr-repositories) を参照してください。

### ECS Cluster membership

[ECS Cluster membership](aws-compute.md#ecs-cluster-membership) を参照してください。

### EKS configuration and membership

[EKS configuration and membership](aws-compute.md#eks-configuration-and-membership) を参照してください。

### RDS Proxy

[RDS Proxy](aws-data.md#rds-proxy) を参照してください。

### ElastiCache configuration references (#691)

[ElastiCache configuration references (#691)](aws-data.md#elasticache-configuration-references-691) を参照してください。

### ElastiCache Serverless

[ElastiCache Serverless](aws-data.md#elasticache-serverless) を参照してください。

### Organizations hierarchy

[Organizations hierarchy](aws-security.md#organizations-hierarchy) を参照してください。

### Organizations policies and direct attachments

[Organizations policies and direct attachments](aws-security.md#organizations-policies-and-direct-attachments) を参照してください。

### Organizations member Account deployment navigation

[Organizations member Account deployment navigation](aws-security.md#organizations-member-account-deployment-navigation) を参照してください。

### DocumentDB (#697)

[DocumentDB (#697)](aws-data.md#documentdb-697) を参照してください。

### Neptune (#698)

[Neptune (#698)](aws-data.md#neptune-698) を参照してください。

### DMS replication configuration (#699)

[DMS replication configuration (#699)](aws-data.md#dms-replication-configuration-699) を参照してください。

### Redshift Serverless (#695)

[Redshift Serverless (#695)](aws-data.md#redshift-serverless-695) を参照してください。

### SageMaker inference configuration (#701)

[SageMaker inference configuration (#701)](aws-data.md#sagemaker-inference-configuration-701) を参照してください。

### FSx Lustre networking and encryption (#702)

[FSx Lustre networking and encryption (#702)](aws-storage.md#fsx-lustre-networking-and-encryption-702) を参照してください。

### PrivateLink endpoints and services (#902)

[PrivateLink endpoints and services (#902)](aws-network.md#privatelink-endpoints-and-services-902) を参照してください。

### VPC gateways and Elastic IPs (#895)

[VPC gateways and Elastic IPs (#895)](aws-network.md#vpc-gateways-and-elastic-ips-895) を参照してください。

### Auto Scaling configuration (#810)

[Auto Scaling configuration (#810)](aws-compute.md#auto-scaling-configuration-810) を参照してください。

### Security group rule relationships (#899)

[Security group rule relationships (#899)](aws-network.md#security-group-rule-relationships-899) を参照してください。

### Workload references and monitoring

[Workload references and monitoring](aws-compute.md#workload-references-and-monitoring) を参照してください。
