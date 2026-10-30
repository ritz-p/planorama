# AWS Compute and containers

[AWS 共通契約・サービス一覧](aws.md)。以下は各節に記載した Provider 版・証拠範囲の契約です。 [HTML 生成手順](aws.md#html-example-generation)。

## Batch compute environments and queues (#700)

Managed `aws_batch_compute_environment` retains an independent Spatial card.
`aws_batch_job_queue` and `aws_batch_scheduling_policy` remain independent Logical
resources. Job definitions remain basic Logical configuration; capacity does not
create jobs, instances or ECS clusters.

| Resource input | Required managed endpoint output | Navigation |
| --- | --- | --- |
| Environment `compute_resources.subnets` (set inside one list block) | `aws_subnet.id` | Configured subnets |
| Environment `compute_resources.security_group_ids` (set inside one list block) | `aws_security_group.id` | Configured security groups |
| Queue `compute_environment_order.compute_environment` (list of blocks) | `aws_batch_compute_environment.arn` | Compute environments |
| Queue `scheduling_policy_arn` | `aws_batch_scheduling_policy.arn` | Scheduling policy |

Queue collection proof covers every block, including distinct known numeric `order`
values; it does not interpret scheduling or claim job placement. The official
[queue documentation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/batch_job_queue.html.markdown)
calls this a set, while its schema and
[registered implementation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/batch/job_queue.go)
use `ListNestedBlock`.

Environment network proof additionally requires `type = MANAGED` and a known
`compute_resources.type` of `EC2`, `SPOT`, `FARGATE` or `FARGATE_SPOT`.
All subnets and SGs must independently prove the same current VPC through explicit
known `vpc_id` references before the environment receives VPC containment.
Multiple subnets never select one subnet. Missing/empty/unknown SGs (including
an implicit default group), split VPCs and UNMANAGED environments retain the
independent card without inferred containment. Complete configured references
can remain available even when common-VPC placement cannot be established.

No Batch resource is embedded. Role/profile/AMI/EKS/launch-template references remain
follow-up #816; job-definition payloads, credentials, policy interpretation and
runtime scheduling/health are excluded.

Examples: [SVG](../examples/batch.svg), [HTML](../examples/batch.html).

## Lambda aliases, layers and function URLs

`aws_lambda_alias` and `aws_lambda_function_url` remain regional logical Nodes;
`aws_lambda_layer_version` is a regional logical Controller and
`aws_lambda_layer_version_permission` a regional logical Policy. All use AWS Lambda
service labels. Their original identities, actions, history and coverage are retained.

| Source | Verified navigation | Required evidence |
| --- | --- | --- |
| Alias | Function; published version on that function | Exact `function_name` reference to Function name/ARN; `function_version` must reference that same Function's matching version |
| Function | Multiple Layer versions | Complete explicit `layers` references to versioned Layer ARNs; the entire known list must match unique referenced instances |
| Function URL | Function and Alias qualifier | Exact Function name/ARN reference; qualifier must reference the Alias name, and both resources must explicitly identify the same Function |
| Layer permission | Layer version | Both `layer_name` and numeric `version_number` must reference the same Layer instance and match its name/ARN and string version |

These are configured reference links, not routed associations or runtime reachability
claims, and do not establish authorization or execution success. Function versions
never become extra Terraform instances. Traffic-splitting versions are not
reconstructed from Alias routing configuration. No URL, code, environment values,
permission principal or policy body is displayed. Literal-only,
partial, unknown, ambiguous, wrong-provider/type, data, deleted and deposed endpoints
retain generic behavior with unresolved dedicated-reference diagnostics. Current
configuration never proves historical links. The initial rules require compatible
provider bindings and known equal Regions; cross-account sharing is not inferred.
Filtering removes unavailable destinations without reconstructing excluded resources.

Layer `version` is a string while permission `version_number` is numeric.
Examples: [SVG](../examples/lambda.svg), [HTML](../examples/lambda.html).

## Lambda configuration and network proof (#949)

Function configuration links require explicit, unique managed-resource provenance,
matching known values, compatible account/provider/region evidence and current,
non-deleted endpoints. Unknown or sensitive values, literals, ambiguous instances
and incomplete collections retain an unresolved row and ordinary dependencies.

| Function input | Referenced output |
| --- | --- |
| `role` | IAM Role `arn` (global endpoint) |
| `code_signing_config_arn` | Lambda Code Signing Config `arn` |
| `dead_letter_config.target_arn` | SNS Topic or SQS Queue `arn` |
| `file_system_config.arn` | EFS or S3 Files Access Point `arn` |
| `logging_config.log_group` | CloudWatch Log Group `name` |
| `kms_key_arn`, `source_kms_key_arn` | KMS Key `arn` |
| `capacity_provider_config.lambda_managed_instances_capacity_provider_config.capacity_provider_arn` | Lambda Capacity Provider `arn` |

Code Signing Config `allowed_publishers.signing_profile_version_arns` must match
the complete set of Signer Profile `version_arn` outputs; unversioned `arn` is not
accepted. Single-item nested blocks and known collection masks are checked.
Code Signing Config remains an independent Logical Controller. References do not
embed shared resources, change their placement or establish signature validity,
authorization, logging delivery or runtime connectivity. Artifact URIs and extra
configuration summaries remain outside this contract.

Function `vpc_config.subnet_ids` and `vpc_config.security_group_ids` require complete
known ID collections. VPC placement additionally requires each current managed
Subnet to prove its matching `vpc_id` reference to the same current managed VPC.
Unknown-only subnet/SG provenance no longer establishes placement or connections.
State identities, actions, filtering and exactly-once coverage are preserved.

The input shapes and outputs were checked against AWS Provider 6.70.0 commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:
[Function schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/lambda/function.go),
[Code Signing Config schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/lambda/code_signing_config.go),
and the corresponding versioned resource documentation, including Signer Profile
`version_arn`. The synthetic [SVG](../examples/lambda-config.svg) and
[HTML](../examples/lambda-config.html) exercise these reference fields.

## App Runner networking (#694)

The service remains Spatial; VPC connector and VPC ingress connection remain
independent Logical resources. Deployment and autoscaling configuration retain
their Logical defaults. A shared connector is never embedded into a service.

| Managed input | Explicit managed endpoint / output | Proof |
| --- | --- | --- |
| `aws_apprunner_service.network_configuration.egress_configuration.vpc_connector_arn` | `aws_apprunner_vpc_connector.arn` | One network/egress block, known `egress_type = VPC`, matching full connector ARN including revision; shown as Egress connector. |
| `aws_apprunner_vpc_connector.subnets` | `aws_subnet.id` | Complete nonempty set of unique known IDs; shown as Configured subnets. |
| `aws_apprunner_vpc_connector.security_groups` | `aws_security_group.id` | Complete nonempty set of unique known IDs; shown as Configured security groups. |
| `aws_apprunner_vpc_ingress_connection.service_arn` | `aws_apprunner_service.arn` | Unique matching known service ARN; shown as Ingress service. |

Connector VPC context additionally requires every subnet and SG to explicitly
reference the same current managed VPC with matching known `vpc_id`/`id` values.
Neither this context nor ingress service navigation puts the service in a customer
VPC or subnet. Egress/ingress references are validated independently of connector
network proof. Ingress endpoint/VPC-pair navigation is deferred pending a separate
endpoint-type/service and VPC-consistency contract.

Every proof requires explicit unambiguous Terraform references to the exact output,
compatible provider/account/region bindings and current managed identities.
Unknown, partial, duplicate-valued, literal-only, wrong-type/output, deleted or
deposed evidence stays unproven. Missing/unknown egress mode never defaults to VPC.
Filtered endpoints show Unavailable without restoring excluded resources; equally
named resources in another state cannot supply missing proof. Action/import/move
metadata and exactly-once coverage remain intact. Proven provider account scope is
retained for these three managed types; ARN strings do not invent account identity.

Service URLs, image identifiers, environment variables/secrets, credentials and
runtime status are not rendered. Relationships do not imply network reachability.
Configuration/domain references remain #776. See the maintained
[SVG](../examples/apprunner.svg) and [HTML](../examples/apprunner.html).

Verified against AWS Provider 6.70.0 commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:
[service](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/apprunner_service.html.markdown),
[connector](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/apprunner_vpc_connector.html.markdown),
[ingress connection](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/apprunner_vpc_ingress_connection.html.markdown).
The schema uses sets for connector subnet/SG collections. The synthetic fixture
models known planned values; it does not claim an applied AWS deployment.

## ECR repositories

`aws_ecr_repository` is a regional spatial card. `aws_ecr_repository_policy`
and `aws_ecr_lifecycle_policy` remain individually selectable logical resources,
with the Amazon ECR service label. `Configured repository` requires a
complete, unique explicit `repository` reference to a managed AWS repository's
`name` or `id`, matching known values and compatible provider bindings.
Unknown, ambiguous, foreign-provider, deleted and deposed endpoints do not prove
this configuration relationship; ordinary Terraform references remain available.
Filtering an endpoint preserves an unavailable reference and coverage identity.

ECS container definitions retain their ordinary structured Terraform references.
Image URL, tag and digest strings do not establish repository relationships.
Policy/lifecycle bodies, registry URLs, credentials and image metadata are not
displayed. References do not certify image pulls, effective permissions or replication.
See the maintained [SVG](../examples/ecr.svg) and [offline HTML](../examples/ecr.html).

The implementation was checked with Terraform 1.9.8 / 1.16.6 and AWS Provider
5.100.0 / 6.0.0 schemas and offline plans. The official
[repository](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/ecr_repository),
[repository policy](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/ecr_repository_policy)
and [lifecycle policy](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/ecr_lifecycle_policy)
contracts use repository names; fresh computed values remain unverified until known.

## ECS Cluster membership

`aws_ecs_cluster` is an independent regional card for managed and data resources.
It retains its own proven deployment scope, or unknown scope, and never becomes a
VPC/Subnet container or a presentation owner for Services. `aws_ecs_task_definition`
remains Logical. Service placement still uses `network_configuration.subnets`;
Services in different VPCs may reference the same Cluster without moving either
the Cluster or another Service.

The service-reference proof recognizes Service `cluster` references to one current
managed Cluster's `id`, `arn` or `name`. It requires complete explicit unambiguous
provenance, known equal planned values, non-deleted endpoints and identical provider
configuration with a known region and no account conflict. Only the proof is kept;
names, ARNs and task-definition bodies are not output. A verified `Cluster membership`
row uses membership metadata and links to the Cluster. It adds no containment,
ownership or runtime-communication edge. Existing dependency arrows retain their
Terraform direction, Cluster → Service.

Unproven membership remains unresolved while ordinary `Cluster` references retain
their original evidence and navigation. Tags and `depends_on` remain ordinary
references/dependencies. Filtering either endpoint preserves the selected resource;
filtered Cluster links show Unavailable. Data, unknown scope, different provider
bindings and deposed objects do not acquire a current membership proof. State-local
identities never join equally named Clusters across states.
The [ECS example](../examples/ecs-clusters.svg) includes shared membership across
two VPCs, a multi-subnet Service, data lookup and an unknown-scope Cluster.

## ECS capacity providers

Capacity providers and cluster-capacity associations remain logical selectors.
The [capacity example](../examples/ecs-capacity.svg) shows both provider variants,
cluster associations and service strategies. No cluster or capacity-provider VPC
inheritance, managed EC2 instance synthesis or runtime scheduling is inferred.

The contract uses AWS Provider 6.70.0 at
[`5e13eed`](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ecs).
Auto Scaling providers accept only `auto_scaling_group_provider.auto_scaling_group_arn`
to a managed Auto Scaling group's `arn`. Managed Instances providers accept
`cluster` to cluster `name`, infrastructure role and instance-profile ARNs, and
explicit launch-template subnet/security-group ID collections. Exactly one provider
variant must be known and present; `cluster` is required only for Managed Instances.

Cluster-capacity associations accept `cluster_name` to cluster `name`, complete
`capacity_providers` and `default_capacity_provider_strategy.capacity_provider`
collections to capacity-provider `name`. Services accept the corresponding
`capacity_provider_strategy.capacity_provider` collection. Every member must have
explicit unique current managed provenance and an equal known nonempty output.
Unknown, partial, duplicate or mismatched collections do not establish a proof.
`FARGATE` and `FARGATE_SPOT` never become managed endpoints; collections containing
them retain ordinary references. Literal names, dependencies, data resources,
deleted/deposed endpoints and incompatible provider/account/region evidence also
retain ordinary references. IAM endpoints use account compatibility without a
regional-resource requirement. State-local identities and filtering are preserved.

Cluster execute-command KMS/log configuration remains deferred until its enabled
variant contract is implemented. Container payloads and effective permissions are
not interpreted.

## ECS workload variants

The [workload example](../examples/ecs-workloads.svg) uses AWS Provider 6.70.0 at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`. Daemons and Express Gateway Services
have independent spatial cards. Daemons never inherit cluster or capacity-provider
network placement. Express placement uses only a complete verified explicit subnet
collection; omitted/default subnets and security groups are never synthesized.
Task definitions remain logical configuration. A task set becomes a service child
only when its explicit `service` and both resources' `cluster` references prove the
same current managed cluster. Ownership adds no service/VPC containment inference.

Daemon `cluster_arn`, complete `capacity_provider_arns` and
`daemon_task_definition_arn` target exact managed `arn` outputs. Express `cluster`
accepts cluster `name` or `arn`; task sets accept the same cluster fields and service
`name` or `arn`. Standard and daemon task definitions expose execution/task role
references; Express also exposes its infrastructure role. Network references use
complete subnet/security-group `id` collections. Service and task-set
`service_registries.registry_arn` target Cloud Map service `arn`.

Service/task-set task definitions and daemon task definitions require the full
revision-qualified ARN from the matching task-definition family's `arn` output.
Family-only identifiers, formatted family/revision strings, revisionless ARNs and
other revisions retain ordinary references. All new proofs require explicit unique
current managed endpoints, known matching nonempty values and compatible provider,
account and regional evidence. IAM endpoints are account-scoped. Unknown, partial,
literal, deleted, deposed and incompatible evidence retains ordinary navigation.
Existing Service cluster/network behavior is preserved, including state boundaries,
filtering and exactly-once resource coverage.

Container JSON, runtime instances, deployment health, image URLs, secret maps,
load-balancer extensions and task-set capacity scheduling remain deferred. No
managed EC2 tasks or effective permissions are inferred.

## EKS configuration and membership

Cluster and Node Group retain spatial cards. Fargate Profile is a regional spatial
Controller; Add-on and Access Entry are regional logical Controllers. All use the
Amazon EKS service label and retain separate identity, search, selection and coverage.

Node Group, Add-on and Fargate Profile show `Cluster membership` only for complete,
unique explicit `cluster_name` references to a current managed AWS Cluster's
`name`/`id`, with equal known values and compatible provider/region bindings.
Access Entry uses `Access configuration cluster`, which describes configuration,
not authentication or effective permissions. Explicit IAM role/principal ARN
references can link to a managed IAM resource without displaying the ARN.
Unknown, ambiguous, wrong-provider/type, deleted/deposed and filtered endpoints
retain ordinary Terraform provenance or unavailable references.

Fargate Profile and Node Group `subnet_ids` require complete explicit ID references
and matching known values to current managed AWS Subnets in the same provider
scope. Placement uses their verified common VPC; cluster membership never makes
the Cluster a network container. A missing/unknown subnet leaves the resource
visible without inferred containment. Cluster SG references keep their existing
`vpc_config.security_group_ids` rules. Fargate Profile has no SG input in the
verified schemas, so no profile SG attachment is inferred from the cluster.
Selectors do not create Pods, namespaces or workloads. Add-on configuration,
principal values and selectors are not displayed.

Add-on, Fargate Profile and Access Entry have independent membership validation
cases; Node Group uses the same checks. Network placement is tested separately.
Schemas and offline plans were checked with Terraform 1.9.8 / 1.16.6 and AWS
Provider 5.100.0 / 6.0.0 against the official
[Add-on](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/eks_addon),
[Fargate Profile](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/eks_fargate_profile)
and [Access Entry](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/eks_access_entry)
contracts. See the maintained [SVG](../examples/eks.svg) and
[offline HTML](../examples/eks.html).

## Auto Scaling configuration (#810)

Auto Scaling groups retain Spatial cards and existing Launch Template and scaling
policy presentation. A complete explicit `vpc_zone_identifier` subnet set supplies
VPC context only when every subnet proves the same current managed VPC. No subnet
is selected as the group owner; AZ names cannot identify default subnets.

Launch configurations stay Logical. Their `image_id`, `security_groups`,
`iam_instance_profile` and `key_name` reference managed AMI IDs, complete Security
Group IDs, instance-profile names and key-pair names respectively. Group
`launch_configuration` references the configuration name. These relationships
never create instances or transfer network placement to a launch configuration.

Group `load_balancers` and attachment `elb` reference Classic ELB names;
`target_group_arns` and `lb_target_group_arn` reference ELBv2 target-group ARNs.
`traffic_source.identifier` requires a known matching `type`: `elb` for a Classic
ELB name, `elbv2` for an ELBv2 target-group ARN, or `vpc-lattice` for a VPC Lattice
target-group ARN. Every entry must match; omitted or unknown type remains
unverified. Collections mixing Classic names with ARN outputs remain unverified
because the reference model does not retain per-endpoint output-field provenance.
Attachment helpers retain their own identity and reference ASG
`name` or `id` (the group name).

Explicit group placement-group names, service-linked-role ARNs, Lambda-only
termination-policy lists and complete mixed-policy override template IDs/names
also receive configuration references. Built-in termination policies mixed with
Lambda references and overrides without explicit templates remain unverified.
Direct/base Launch Template handling is unchanged.

All new relationships require known matching values, unique explicit references,
current managed AWS endpoints and compatible provider/account/region evidence.
IAM endpoints are global within the compatible account. Data sources, deleted or
deposed endpoints, literals, unknowns and partial collections retain conservative
fallback. Selection and named-state boundaries preserve coverage; private user
data, runtime capacity and health are neither interpreted nor emitted.

Contracts use pinned AWS Provider 6.70.0
[ASG](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/autoscaling_group.html.markdown),
[launch configuration](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/launch_configuration.html.markdown),
[attachment](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/autoscaling_attachment.html.markdown), and
[traffic attachment](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/autoscaling_traffic_source_attachment.html.markdown).
[Tests](../tests/unit/semantic/autoscaling_network.rs),
[fixture](../tests/fixtures/autoscaling-network-plan.json),
[SVG](../examples/autoscaling-network.svg) and
[HTML](../examples/autoscaling-network.html) cover the configuration paths.

## Workload references and monitoring

Auto Scaling Group Launch Template usage is shown only from complete, explicit
`launch_template.id` / `launch_template.name` references or the equivalent
`mixed_instances_policy.launch_template.launch_template_specification` ID/name path.
Dynamic or ambiguous candidates remain unresolved; names, IDs, versions and module
proximity are never matched heuristically. Launch Templates retain regional deployment
scope metadata when their provider configuration proves it, but remain outside spatial
VPC/Subnet containment.

CloudWatch Log Groups are independent regional cards, using only their own proven
deployment scope and keeping unknown scope when it cannot be established. Managed,
data and deposed groups retain separate identities; consumers never become their
network parents or presentation owners. Existing complete, explicit references to
groups remain navigable on spatial and Logical consumers, with unavailable targets
after filtering and unresolved rows for partial or ambiguous references. These are
ordinary `References` or `Dependency` rows for ECS container-definition references.
Lambda's verified `logging_config.log_group` additionally shows a configured log
group link under the contract above; neither proves delivery success.
Tags and `depends_on` receive no logging semantics or dedicated
delivery edge. The [Log Group example](../examples/log-groups.svg) covers shared
consumers, data lookup and unknown scope. Alarm monitoring is
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
