# AWS architecture relationships

AWS rules live in `src/provider/aws`. Dispatch uses provider source identity and
exact resource types; a custom provider's lookalike type does not acquire AWS
semantics. Provider source and configuration identity are described in [models](models.md).

## Resource roles and data

VPC and Subnet are spatial `Container` resources. Other supported types are
classified as nodes, connectors, associations, policies or controllers by
`classification.rs`. Unknown types remain visible as `Unknown`. Roles are separate
from managed/data mode; Security Groups remain Policy cards and target groups
remain Controller cards even when placed inside a VPC.

Infrastructure data sources retain their roles and use an external badge and
dashed border. AWS metadata data sources `aws_region`, `aws_partition`,
`aws_caller_identity`, `aws_availability_zones`, and `aws_iam_policy_document`
and their incident edges are omitted. Cached data resources in prior state are
included when still present in configuration. See [data.svg](../examples/data.svg).

## Containment

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
| `aws_route_table_association` | `subnet_id`, `route_table_id` | Subnet → Route Table |
| `aws_lb_target_group_attachment` | `target_group_arn`, `target_id` | Target Group → EC2 Instance |
| `aws_vpc_endpoint_route_table_association` | `vpc_endpoint_id`, `route_table_id` | VPC Endpoint → Route Table |

These managed helper cards become Association edges only when both endpoints are
statically and uniquely resolved and the helper has exactly those two incident
dependencies. Extra consumers/dependencies, data helpers and unsupported target
types leave the card visible. Every lowered edge retains its source address,
action and lifecycle metadata. Parallel associations remain distinct.

`aws_route` has additional planned-value requirements described in [routes](routes.md).
Other Association-role resources remain cards. Examples:
[association.svg](../examples/association.svg), [aws-relationships.svg](../examples/aws-relationships.svg).
