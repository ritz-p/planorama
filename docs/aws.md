# AWS architecture relationships

AWS rules live in `src/provider/aws`. Dispatch uses provider source identity and
exact resource types; a custom provider's lookalike type does not acquire AWS
semantics. Provider source and configuration identity are described in [models](models.md).

## Deployment scopes

Known static provider regions group explicitly supported regional resources into
visual panels (see [regions.svg](../examples/regions.svg)). Panels are deployment
scopes, not resource containers or Terraform modules. VPC/Subnet containment stays
intact inside each panel, and cross-region relationships remain routed paths.
Spatial IAM resources, CloudFront and supported Route 53 resources share an
`AWS Global` panel when regional panels are present. IAM Role, Policy, Role Policy
and Instance Profile retain global scope metadata but render as logical-resource
rows above the spatial diagram, outside deployment-scope panels. AWS scope
classification lives in `provider/aws/scope.rs`; layout and SVG consume generic
deployment scopes. Unsupported resource types, cross-region peering resources and
unknown regions stay outside panels. No aliases, names, tags, ARNs or IDs supply
region information.

A containment subtree is grouped only when all members have the same known scope.
Conflicting or unknown members leave the whole subtree outside panels, preserving
containment without suggesting a guessed region. Plans without a groupable regional
subtree keep their existing layout. Synthetic component summaries remain separate.
Named-state sections retain independent panels even for equal region names;
`data-deployment-scope`, `data-scope-provider`, `data-region` or `data-global`
identify panel metadata within their enclosing `data-state-id`.

## Resource roles and data

VPC and Subnet are spatial `Container` resources. Other supported types are
classified as nodes, connectors, associations, policies or controllers by
`classification.rs`. Unknown types remain visible as `Unknown`. Roles are separate
from managed/data mode; Security Groups remain Policy cards and target groups
remain Controller cards even when placed inside a VPC.

Placement is independent of role. ECS Cluster, ECS Task Definition, IAM Role,
Policy, Role Policy, Instance Profile, AMI, CloudWatch Log Group, Metric Alarm and Auto Scaling Policy definitions are explicit logical
resources, rendered as rows grouped by provider source, resource type and mode in
the top annotation area. AMI describes
an image definition used by workloads, not a deployed compute instance. These
provider-aware rules apply to managed and data resources; custom providers do
not inherit them. Unknown types retain unresolved placement and stay visible in
the spatial diagram. Missing VPC/subnet references never imply logical placement.
S3, DynamoDB, CloudFront, ECS Service and load balancers remain spatial.

Only `aws_cloudwatch_log_group` and `aws_cloudwatch_metric_alarm` receive this
CloudWatch policy; unsupported CloudWatch types retain unresolved spatial cards.
Logical placement does not suppress architectural significance: individual resource
rows retain actions, identities, operation metadata, scope and focus/filter
participation. A focused observability view can emphasize them; no spatial-view
override is currently implemented.

Configured Log Group references remain linked on their consumers. A tag reference
is labeled References and does not claim configured logging. Alarm monitoring is
separate from a dependency: the initial supported proof requires AWS/RDS, a known
metric name, exactly one DBInstanceIdentifier dimension, complete explicit
provenance to one current AWS DB Instance, matching provider configuration and a
known planned identifier equal to the dimension. Metric queries, unknown values,
ambiguous references and other dimension/namespace combinations stay unresolved
with an UnverifiedMonitoring diagnostic. Values are discarded after comparison.
Verified alarms show Monitors; the target shows Monitored by. These inverse
reference rows create no graph edges. Generic depends_on never proves monitoring,
and no monitoring relationship is inferred across states.

Auto Scaling Policies are logical Policy resources rather than spatial cards. A
complete explicit `autoscaling_group_name` reference to exactly one current AWS
`aws_autoscaling_group` with the same provider configuration is labeled
`Auto Scaling group` on the policy and adds a `Scaling policy` inverse reference
to that ASG. This is reference metadata only: it creates no spatial path,
containment or generic inverse-reference rule. Dynamic, partial, missing,
ambiguous, foreign-provider, wrong-type and cross-provider-configuration targets
do not gain the inverse row; literal name equality, module proximity and policy
names are never used as ownership evidence. Multiple verified policies remain
separate and sort by Terraform identity. Filtering can leave either endpoint
unavailable without emitting a dangling link. See
[autoscaling-policy.svg](../examples/autoscaling-policy.svg).

Infrastructure data sources retain their roles and external identity. Spatial
cards use an external badge and dashed border; logical resources use separate
`data / external` groups and dashed action underlines. AWS metadata data sources
`aws_region`, `aws_partition`,
`aws_caller_identity`, `aws_availability_zones`, and `aws_iam_policy_document`
and their incident edges are omitted. References from retained resources keep
the omitted source identities as provenance, without creating cards or paths.
Cached data resources in prior state are
included when still present in configuration. See [data.svg](../examples/data.svg).

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
| Lambda / EKS | `vpc_config.subnet_ids` | Common VPC |
| OpenSearch | `vpc_options.subnet_ids` | Common VPC |

LB/ECS subnet references outside the chosen parent become Connection edges.
Subnet groups, RDS and the other workloads retain their original dependencies.
Nested fields are resolved separately: flattened block references or sibling SG
fields cannot establish subnet membership. Derived containment retains the reference
chain and receives the [inferred-placement annotation](relationships.md).

Examples: [multiple subnets](../examples/multi-container.svg),
[RDS](../examples/rds-subnet-group.svg), [subnet groups](../examples/subnet-groups.svg),
[VPC workloads](../examples/vpc-workloads.svg).

## Security Group attachments

| Consumer | Attribute |
| --- | --- |
| EC2, RDS Instance / Cluster | `vpc_security_group_ids` |
| Load Balancer | `security_groups` |
| ECS Service | `network_configuration.security_groups` |
| VPC Endpoint | `security_group_ids` |
| Lambda / EKS | `vpc_config.security_group_ids` |
| OpenSearch | `vpc_options.security_group_ids` |

Each explicitly referenced AWS SG becomes a Connection to the managed consumer.
Multiple groups and exact indexed references are supported, including external
SG data sources. Attachment does not change containment or hide SG cards.

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
| `aws_route_table_association` | `subnet_id`, `route_table_id` | Subnet ↔ Route Table |
| `aws_lb_target_group_attachment` | `target_group_arn`, `target_id` | Target Group ↔ EC2 Instance |
| `aws_vpc_endpoint_route_table_association` | `vpc_endpoint_id`, `route_table_id` | VPC Endpoint ↔ Route Table |

These managed helper cards become Association edges only when both endpoints are
statically and uniquely resolved and the helper has exactly those two incident
dependencies. Extra consumers/dependencies, data helpers and unsupported target
types leave the card visible. Every lowered edge retains its source address,
action and lifecycle metadata. Parallel associations remain distinct.

`aws_route` remains a graph node and uses verified [Route Table child-card ownership](routes.md).
Other Association-role resources remain cards. Examples:
[association.svg](../examples/association.svg), [aws-relationships.svg](../examples/aws-relationships.svg).
