# AWS architecture relationships

AWS rules live in `src/provider/aws`. Dispatch uses provider source identity and
exact resource types; a custom provider's lookalike type does not acquire AWS
semantics. Provider source and configuration identity are described in [models](models.md).

## Deployment scopes

`aws_rds_cluster_instance` is a supported regional resource with the RDS icon.
Managed instances with a complete, explicit, unique `cluster_identifier` reference
to a managed AWS `aws_rds_cluster` appear as individual cards inside that cluster
when the known `cluster_identifier` matches its referenced `id`.
Deleting instances use both identifiers from before-state evidence.
The instance and cluster must have equal deployment scopes; unknown, incompatible,
dynamic, ambiguous or filtered ownership falls back to an ordinary visible card.
Each member retains its address, action, lifecycle metadata and account/region
scope. Membership adds no semantic VPC/Subnet containment. Original references
remain dependencies, and member cards show navigable cluster and other reference
links. See the [Aurora example](../examples/aurora.svg).

`aws_sqs_queue` and `aws_sns_topic` are spatial nodes with AWS account and
regional scope from provider evidence. Managed/data mode, actions and ordinary
Terraform references remain visible. The [messaging example](../examples/messaging.svg)
includes both services and a data lookup in a singleton region. Names, ARNs,
policies and proximity do not establish message flow.

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
An explicit resource-level `region` expression currently makes regional scope
unknown, even when literal: overrides are not yet resolved. The provider's region
and binding remain intact. Missing/dynamic bindings, unsupported types and deposed
objects without regional evidence also remain unknown. Global classification is
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

## SNS subscriptions

Managed `aws_sns_topic_subscription` resources remain logical rows, with their own
provider/account/region, action and lifecycle identity. Complete unique explicit
`topic_arn` and `endpoint` references can additionally produce a directed
`Subscription` from SNS Topic to SQS Queue or Lambda Function. The planned endpoint
ARNs must be known and equal to the referenced resource ARNs; only boolean proofs
are retained. The planned `protocol` must agree with the endpoint type (`sqs` or
`lambda`). This conservatively rejects compound expressions with different values.
For deletion actions, verification uses known `before` values (or prior state
when a resource has no change entry) for both the subscription and its endpoints.
Explicit configuration references are still required; planned values cannot prove
a deleted relationship's historical endpoints.

Missing/unknown values, literal-only endpoints, ambiguous/dynamic selectors,
unsupported protocols, foreign providers and deposed resources keep logical rows
with explicit unresolved fields and any independently verified endpoint. Data
queues/topics/functions can be referenced; data subscriptions do not create edges.
Current configuration is never applied to a deposed subscription.

The source row and numbered relationship index share one Terraform identity and
are counted once. Parallel subscriptions retain separate numbers and actions.
Filtering checks availability of every link; removed endpoints produce no path.
Source and endpoint scopes remain independent, including cross-region/account
wiring. State-local references never resolve against an identically named object
in another state. These edges describe configured subscriptions, not successful
delivery, access-policy correctness or runtime reachability.
See [the subscription example](../examples/sns-subscriptions.svg).

## Lambda event source mappings

Managed `aws_lambda_event_source_mapping` resources remain logical rows and add a
directed `Event source` relationship from a verified SQS Queue to a verified Lambda
Function. Both `event_source_arn` and `function_name` require complete, unique,
explicit references. Known planned values must match the queue ARN and the function
name or ARN, respectively. Values are compared transiently; only boolean evidence
is retained. Literal names/ARNs, policies, environment variables and proximity do
not identify endpoints. Unknown or mismatched values remain unresolved.
Deletion actions use known `before` values (or prior state when a resource has no
change entry) for the mapping and both endpoints, still requiring explicit
configuration references, just as subscriptions do.

Each mapping retains its own action, import/move metadata, scope, source identity
and relationship number, including parallel mappings between identical endpoints.
Deposed mappings stay separate logical rows without borrowing current references.
Data queue/function endpoints are supported; data mappings do not establish edges.
Unsupported sources, missing/filtered targets and incomplete or dynamic references
keep the logical row and independently verified partial context. A missing endpoint
never resolves against another state. Cross-account/region endpoints retain their
own scope; the mapping's provider binding does not assign scope to either endpoint.

The relationship describes configured event consumption, not active mapping state,
processing success, access permissions, reachability or delivery guarantees.
Kinesis, DynamoDB Streams, MSK, Kafka, MQ and DocumentDB require separate explicit
type, endpoint and direction rules before they can produce event-source edges.
See [the event-source example](../examples/event-sources.svg), including duplicate
endpoints, a cross-region data queue, an unsupported source and a literal fallback.

## Resource roles and data

VPC and Subnet are spatial `Container` resources. Other supported types are
classified as nodes, connectors, associations, policies or controllers by
`classification.rs`. Unknown types remain visible as `Unknown`. Roles are separate
from managed/data mode; Security Groups remain Policy cards and target groups
remain Controller cards even when placed inside a VPC.

Policy describes access/control rules or policy-like configuration; it does not
necessarily mean IAM or policy enforcement. Controller describes resources that
manage or configure services, workloads or related resources; the role alone does
not establish runtime control, traffic or relationships. Logical type cards show
these explanations once per role, separately from Terraform action and managed/data
mode. Their action subsections change presentation only, not role or placement.

Placement is independent of role. ECS Cluster, ECS Task Definition, IAM Role,
Policy, Role Policy, Instance Profile, AMI, EC2 Launch Template, CloudWatch Log Group,
Metric Alarm, Auto Scaling Policy, SNS Topic Subscription and Lambda Event Source Mapping definitions are explicit logical resources, rendered as rows grouped
by provider source, resource type and mode in
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

Auto Scaling Group Launch Template usage is shown only from complete, explicit
`launch_template.id` / `launch_template.name` references or the equivalent
`mixed_instances_policy.launch_template.launch_template_specification` ID/name path.
Dynamic or ambiguous candidates remain unresolved; names, IDs, versions and module
proximity are never matched heuristically. Launch Templates retain regional deployment
scope metadata when their provider configuration proves it, but remain outside spatial
VPC/Subnet containment.

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
to that ASG only when the known planned policy target name equals the ASG's known
planned name. Only the equality proof is retained; the values are discarded after
comparison. This is reference metadata only: it creates no spatial path,
containment or generic inverse-reference rule. Dynamic, partial, missing,
ambiguous, foreign-provider, wrong-type, cross-provider-configuration and
unproven-value targets remain unresolved and are diagnosable. Literal name
equality without reference provenance, module proximity and policy names are never
used as ownership evidence. Multiple verified policies remain separate and sort
by Terraform identity. Filtering can leave either endpoint unavailable without
emitting a dangling link. See
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
| ElastiCache Replication Group | `subnet_group_name` → ElastiCache Subnet Group | Same verified VPC as the subnet group |
| Lambda / EKS | `vpc_config.subnet_ids` | Common VPC |
| OpenSearch | `vpc_options.subnet_ids` | Common VPC |

LB/ECS subnet references outside the chosen parent become Connection edges.
Subnet groups, RDS and the other workloads retain their original dependencies.
ElastiCache subnet and replication groups are supported visible resources with
AWS account/region scope and the ElastiCache icon. Replication placement requires
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
