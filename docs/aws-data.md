# AWS Databases and analytics

[共通契約・サービス一覧](aws.md) に戻る。以下はサービス固有の条件です。各節に記載した Provider 版と証拠の範囲を維持し、他サービスの条件を転用しないでください。

- [OpenSearch Serverless (#696)](#opensearch-serverless-696)
- [RDS Global Clusters](#rds-global-clusters)
- [Redshift provisioned clusters](#redshift-provisioned-clusters)
- [Athena Workgroup and Data Catalog](#athena-workgroup-and-data-catalog)
- [Glue Catalog, Crawler and Job](#glue-catalog-crawler-and-job)
- [Aurora cluster instances](#aurora-cluster-instances)
- [RDS Proxy](#rds-proxy)
- [ElastiCache configuration references (#691)](#elasticache-configuration-references-691)
- [ElastiCache Serverless](#elasticache-serverless)
- [DocumentDB (#697)](#documentdb-697)
- [Neptune (#698)](#neptune-698)
- [DMS replication configuration (#699)](#dms-replication-configuration-699)
- [Redshift Serverless (#695)](#redshift-serverless-695)
- [SageMaker inference configuration (#701)](#sagemaker-inference-configuration-701)

## OpenSearch Serverless (#696)

Collections retain independent Spatial cards. VPC endpoints remain Logical,
with dedicated reference navigation and verified common-VPC context. An endpoint
does not own collections, and collection cards never inherit endpoint subnets.

The managed `aws_opensearchserverless_vpc_endpoint` contracts were checked against
AWS Provider 6.70.0 commit `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:

| Input | Required endpoint output | Navigation |
| --- | --- | --- |
| `vpc_id` | `aws_vpc.id` | Endpoint VPC |
| `subnet_ids` (set) | `aws_subnet.id` | Configured subnets |
| `security_group_ids` (optional/computed set) | `aws_security_group.id` | Configured security groups |

Each path requires explicit unique compatible managed Terraform references and
matching known IDs. Collections must be complete, nonempty and duplicate-free.
VPC context additionally requires every selected subnet and SG to have a verified
`vpc_id` reference to the endpoint's configured VPC. A multi-subnet endpoint is
never assigned to one arbitrary subnet. Individually proven references remain
available even when their combined network context disagrees.

Omitted, null, empty or unknown SGs do not identify the default SG and cannot
establish containment. Literal-only IDs, wrong output attributes, ambiguous
instances, foreign providers, incompatible bindings/accounts/regions, deleted or
deposed objects and filtered evidence retain conservative fallbacks. Named states
cannot supply one another's missing evidence. Collection/endpoint scope retains
verified provider account evidence; ordinary JSON does not prove account ownership.
Data sources keep their existing unresolved presentation and receive no managed
reference rules. Lifecycle actions, moves/imports and independent coverage remain.

Access, security and lifecycle policy JSON stays opaque. No collection membership,
encryption summary or reachability is derived from policies or endpoint URLs.
Direct collection-group/key and Identity Center references remain in #799.
The [fixture](../tests/fixtures/opensearch-serverless-plan.json),
[example](../examples/opensearch-serverless.svg) and
[tests](../tests/unit/semantic/opensearch_serverless.rs) cover positive references,
network disagreement, unknown/partial values, identity boundaries and rendering.

## RDS Global Clusters

`aws_rds_global_cluster` is a Global spatial resource with the RDS service
label. It never inherits the execution provider's region. Regional
`aws_rds_cluster` cards retain their own provider/account/region identity and
existing embedded Instance ownership and coverage.

A Cluster's `global_cluster_identifier` becomes **Global Cluster membership**
only with a complete, unique explicit reference to a current managed AWS Global
Cluster's `id` or `global_cluster_identifier` and equal known evaluated values.
Different provider aliases and regions are allowed; conflicting verified account
evidence rejects membership. Literal IDs, unknown/dynamic/ambiguous references,
data, deleted/deposed and wrong-provider/type endpoints keep ordinary references
and unresolved diagnostics. Filtering marks absent endpoints unavailable, and
named states resolve their own identities independently.

Membership describes configuration, not replication health, writer/reader roles
or failover order. Computed `global_cluster_members`, credentials and attribute
values are not emitted. `source_db_cluster_identifier` retains ordinary Terraform
reference behavior without inferring a primary role.

Checked against Terraform 1.9.8 plan output and AWS Provider 5.100.0's schema and
[Global Cluster contract](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/rds_global_cluster.html.markdown).
The synthetic [fixture](../tests/fixtures/rds-global-plan.json),
[SVG](../examples/rds-global.svg) and [offline HTML](../examples/rds-global.html)
show two regional clusters, embedded instances and an unresolved literal ID.
Regenerate HTML with
`docker compose exec -T dev cargo run --locked -- tests/fixtures/rds-global-plan.json --format html -o examples/rds-global.html`.

## Redshift provisioned clusters

`aws_redshift_cluster` and `aws_redshift_subnet_group` are regional spatial Nodes;
`aws_redshift_parameter_group` is a regional Logical Policy. All use Amazon
Redshift service labels. Serverless resources follow the separate #695 contract below.

Cluster subnet/parameter group references require complete unique explicit
references to the corresponding group's `name` or `id`, matching known values
and compatible provider/region evidence. Subnet Group `subnet_ids` and Cluster
`vpc_security_group_ids` require complete explicit ID collections equal to the
known current managed AWS endpoint IDs. Historical, unknown, mismatched,
ambiguous and foreign endpoints remain unverified and diagnosable.

A verified Subnet Group is placed in its Subnets' common VPC with proven VPC
references. A Cluster follows that verified group to the VPC, including when
only one Subnet is selected; it is never duplicated across Subnets or treated as
deployed in every listed Subnet. Security Group connections describe configured
association, not reachability or effective access. Parameter values, passwords
and connection endpoints are never emitted. Filtering and named states retain
each resource's identity and coverage.

See the [synthetic fixture](../tests/fixtures/redshift-plan.json),
[SVG](../examples/redshift.svg) and [offline HTML](../examples/redshift.html).
Regenerate HTML in Docker with `cargo run --locked -- tests/fixtures/redshift-plan.json --format html -o examples/redshift.html`.
The AWS 5.100.0 schema and official [Cluster](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/redshift_cluster),
[Subnet Group](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/redshift_subnet_group)
and [Parameter Group](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/redshift_parameter_group)
contracts define these scalar names and set-valued ID collections.

## Athena Workgroup and Data Catalog

Exact AWS `aws_athena_workgroup` and `aws_athena_data_catalog` are independent
regional spatial Nodes with the Amazon Athena service label. Neither is a
presentation owner of the other. Their own deployment region comes
from the verified provider binding; a catalog's parameters do not relocate it to
the region/account of a backend. Missing or unresolved region overrides remain
unknown. Managed/data resources keep separate identities and coverage entries.

Explicit Workgroup configuration references, including S3 output-location and
KMS encryption-key expressions, remain ordinary Terraform `References` and
dependencies. They do not assert query execution, effective encryption, access or
runtime use. Unknown planned values do not erase structurally explicit Terraform
provenance. Dynamic, missing and ambiguous references use shared diagnostics;
filtered targets become Unavailable. Foreign providers and unsupported exact types
keep generic presentation and ordinary references.

No dedicated association is inferred between Workgroups and Data Catalogs, or
between a Data Catalog and a Glue Database merely present in the same input.
Literal output URIs, matching names and Catalog parameters never create links.
Query bodies/history, Catalog parameter values and arbitrary attributes are not
rendered. Named queries, prepared statements and runtime query graphs are outside
the initial scope. Current configuration does not prove historical use for
deleted/deposed objects; their lifecycle and Terraform provenance remain visible.

AWS Provider 5.100.0 official
[Workgroup](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/athena_workgroup.html.markdown)
and [Data Catalog](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/athena_data_catalog.html.markdown)
definitions, installed schemas and a Terraform 1.9.8 offline plan confirm the
nested configuration and map-expression shapes. The synthetic
[fixture](../tests/fixtures/athena-plan.json), [SVG](../examples/athena.svg) and
[offline HTML](../examples/athena.html) contrast explicit S3/KMS references with
literal-only settings, multiple indexed instances and independent Athena/Glue
catalogs. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/athena-plan.json --format html -o examples/athena.html`.

## Glue Catalog, Crawler and Job

Exact AWS `aws_glue_catalog_database`, `aws_glue_catalog_table`, `aws_glue_crawler`
and `aws_glue_job` are regional spatial Nodes using the AWS Glue service
label. Every resource retains its own identity, lifecycle and coverage entry.
Catalog Database/Table account scope remains unknown: an explicit catalog can
belong to a different account from the execution provider.

Database, Crawler and Job are independent cards. Database is not a network container.
Table `Catalog database` membership retains its reference row and can also provide
presentation ownership for a measured child card in the Database's Tables section.
The child keeps its own identity, action, bounds, ports and coverage entry.
Missing or filtered parents, incompatible or unknown scope, ambiguous references,
deleted/deposed endpoints and tables referenced by other resources use independent
cards. Presentation ownership never supplies account or region evidence.
It requires a complete, unique, explicit reference to a current managed Database's
`name`, matching known planned names and catalog IDs, and equal known effective
regions. Crawler database references remain ordinary Terraform references because
the crawler has no catalog ID field proving the target catalog. Unknown region/catalog evidence,
dynamic instances, wrong providers/types and deleted/deposed endpoints keep
ordinary Terraform references with an unproven-value diagnostic. Filtered links
become Unavailable. Matching literal names alone establish no relationship.

Crawler `role` and Job `role_arn` produce `Execution role reference` rows only for
explicit IAM Role `arn` references with equal known values. Name-only role
references remain ordinary Terraform references. Explicit S3 references in
Crawler targets and Job script locations also retain the shared Terraform
reference presentation; URI strings create no links. These configuration
references prove neither execution success nor effective permissions.
Script bodies, connection credentials, schema contents and argument values are
omitted. Job arguments/scripts are not interpreted as lineage or execution order;
workflows, triggers and execution graphs remain outside the initial scope.

AWS Provider 5.100.0 installed schemas and official
[Database](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/glue_catalog_database.html.markdown),
[Table](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/glue/catalog_table.go),
[Crawler](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/glue_crawler.html.markdown)
and [Job](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/glue_job.html.markdown)
definitions were checked with Terraform 1.9.8 and its
[plan JSON representation](https://developer.hashicorp.com/terraform/internals/json-format).
The synthetic [fixture](../tests/fixtures/glue-plan.json), [SVG](../examples/glue.svg)
and [offline HTML](../examples/glue.html) contrast explicit configuration with
literal-only values. Regenerate HTML inside Docker with
`cargo run --locked -- tests/fixtures/glue-plan.json --format html -o examples/glue.html`.

## Aurora cluster instances

`aws_rds_cluster_instance` is a supported regional resource with the RDS service label.
Managed instances with a complete, explicit, unique `cluster_identifier` reference
to a managed AWS `aws_rds_cluster` appear as individual cards inside that cluster
when the known `cluster_identifier` matches its referenced `id`.
Deleting instances use both identifiers from before-state evidence.
Unknown accounts require the same provider configuration; different configurations
require equal verified accounts.
The instance and cluster must have equal deployment scopes; unknown, incompatible,
dynamic, ambiguous or filtered ownership falls back to an ordinary visible card.
Each member retains its address, action, lifecycle metadata and account/region
scope. Membership adds no semantic VPC/Subnet containment. Original references
remain dependencies, and member cards show navigable cluster and other reference
links. See the [Aurora example](../examples/aurora.svg).

## RDS Proxy

`aws_db_proxy` is an independent regional Connector card. Its default target group
and targets are regional logical Controllers, with their own identities, actions,
search and coverage. They use the Amazon RDS service label.

`Proxy owner` and `Configured proxy` require an explicit reference to a current
managed AWS Proxy's `name`/`id`, matching known values and compatible provider scope.
`Target group` additionally requires the target and group to reference the same
Proxy identity. A literal `default` group name does not establish ownership.
`DB instance target` accepts an explicit DB Instance `identifier` reference;
`DB cluster target` accepts an RDS Cluster `cluster_identifier` reference. They
require validated group ownership and exactly one target kind, including known
absence of the other kind. Ambiguous, unknown, foreign-provider/type, deleted or
deposed resources do not prove these links. Ordinary Terraform references survive
unverified cases; filtering preserves unavailable endpoint identities.

Proxy `vpc_subnet_ids` and `vpc_security_group_ids` require complete explicit ID
references, matching known values and current managed AWS endpoints in the same
provider scope. Subnet evidence places the Proxy in the verified common VPC,
never inside a target DB. Role and single-auth-block Secret references can be
shown without their values. Multiple/unknown auth references remain ordinary
Terraform provenance. Endpoint URLs, authentication values and connection-pool
configuration are not displayed. The diagram does not certify connections,
authentication, permissions or read/write routing.

Terraform 1.9.8 / 1.16.6 and AWS Provider 5.100.0 / 6.0.0 schemas and offline plans
were checked against the official
[Proxy](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/db_proxy),
[default target group](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/db_proxy_default_target_group)
and [target](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/db_proxy_target)
contracts. See the maintained [SVG](../examples/rds-proxy.svg) and
[offline HTML](../examples/rds-proxy.html).

## ElastiCache configuration references (#691)

Contracts were checked against AWS Provider 6.70.0, official commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`, the pinned full schema and the
[official resource documents](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r).

| Managed consumer | Input | Managed endpoint / output |
| --- | --- | --- |
| `aws_elasticache_global_replication_group` | `primary_replication_group_id` | `aws_elasticache_replication_group.id` or `.replication_group_id` |
| `aws_elasticache_replication_group` | `global_replication_group_id` | `aws_elasticache_global_replication_group.id` or `.global_replication_group_id` |
| `aws_elasticache_cluster` | `replication_group_id` | `aws_elasticache_replication_group.id` or `.replication_group_id` |
| `aws_elasticache_cluster`, `aws_elasticache_replication_group` | `parameter_group_name` | `aws_elasticache_parameter_group.name` or `.id` |
| `aws_elasticache_replication_group` | `user_group_ids` (set of strings) | `aws_elasticache_user_group.user_group_id` or `.id` |
| `aws_elasticache_serverless_cache`, `aws_elasticache_user_group_association` | `user_group_id` | `aws_elasticache_user_group.user_group_id` or `.id` |
| `aws_elasticache_user_group` | `user_ids` (set of strings) | `aws_elasticache_user.user_id` or `.id` |
| `aws_elasticache_user_group_association` | `user_id` | `aws_elasticache_user.user_id` or `.id` |

The pinned provider source assigns Terraform IDs from the same API identifiers in
`internal/service/elasticache/user.go` (UserId), `user_group.go` (UserGroupId),
`replication_group.go` (ReplicationGroupId) and `global_replication_group.go`
(GlobalReplicationGroupId); their read functions populate the corresponding
explicit output attributes. Parameter-group `id` is documented as its name.

Other inputs above are strings. Each scalar requires one explicit unique current
managed AWS reference and equal known nonempty input/output values. Sets require
complete known values, one unambiguous output attribute shared by all references,
and a one-to-one match to distinct endpoints. Mixed `.id`/`.user_id` expressions,
partial sets, duplicates, unknown elements and transformed references stay
unproven. Literal values, names, ARN substrings and `depends_on` do not create
links. `user_name` and global ID suffixes are not accepted endpoint identifiers.

Regional links require equal known regions. Global membership preserves the
regional endpoint's own scope; it never assigns the global object's provider
region to its members. Bindings require equal verified accounts or the same
provider binding when both accounts are unknown. Conflicting or one-sided account
evidence fails. Cross-region aliases without verified account evidence remain
unresolved in plain JSON; the existing saved-plan verification supplies account
proof. Data sources, deleted and deposed resources cannot establish current links.
Unproven explicit references retain generic dependencies and diagnostics.

Global groups, user groups, users, parameter groups and association helpers retain
logical presentation and independent identity. Links do not embed shared users,
create network containment, or route new delivery edges. Association helpers keep
actions, import/move provenance and coverage exactly once; named states cannot
borrow endpoints from another state, and filtered endpoints become unavailable.
The configured-primary label describes the explicit input only, never live role,
health, failover sequence or replication success. Existing network placement and
encryption presentation are unchanged. Passwords (including write-only forms),
authentication/access strings and parameter values are never displayed.

Global-group `parameter_group_name` remains deferred: its regional application
requires a separate explicit primary-region/configuration proof, and a global
object's scope cannot supply that proof. Other exclusions are cache log/SNS/KMS
and snapshot references, standalone-cluster indirect placement, computed member
expansion and encryption summaries. The #718 audit records their separate scope;
#691 does not infer them from adjacent service rules.

[Fixture](../tests/fixtures/elasticache-configuration-plan.json),
[semantic tests](../tests/unit/semantic/elasticache.rs),
[maintained SVG](../examples/elasticache-configuration.svg), browser presentation
checks and native/Wasm SVG/HTML parity cover these contracts.

## ElastiCache Serverless

`aws_elasticache_serverless_cache` is an independent regional Node card, using
the Amazon ElastiCache service label. It is distinct from provisioned Cluster
and Replication Group resources: no cache nodes, replication members or subnet
groups are synthesized.

Explicit `subnet_ids` and `security_group_ids` use the current managed endpoint,
known-value and provider-scope checks described for RDS Proxy. Complete Subnet
evidence places the cache in their verified common VPC, including a single-subnet
configuration; it does not assign a cache node to each Subnet. Missing, unknown,
mixed literal/reference, dynamic, ambiguous, wrong-provider/type, deleted/deposed
or filtered evidence never invents placement or SG connections. The original
resource identity, ordinary references, actions and coverage remain available.
Data lookups keep their identity without inferred network attachments.

Endpoint and reader endpoint values, user-group values and connection details
are not displayed; the graph does not claim runtime cache node count or reachability.
Terraform 1.9.8 / 1.16.6 and AWS Provider 5.100.0 / 6.0.0 schemas and offline plans
were checked against the official
[Serverless Cache contract](https://registry.terraform.io/providers/hashicorp/aws/6.0.0/docs/resources/elasticache_serverless_cache).
See the maintained [SVG](../examples/elasticache-serverless.svg) and
[offline HTML](../examples/elasticache-serverless.html).

## DocumentDB (#697)

Verified provider account scope is preserved for the cluster, instance and subnet
group. Filtered placement requires every intermediate subnet-group, subnet and SG
identity to remain visible; keeping only the cluster and its direct neighbors does
not retain inferred VPC placement.

`aws_docdb_cluster` and `aws_docdb_cluster_instance` retain their spatial cards;
`aws_docdb_subnet_group` remains a shared logical selector. A cluster is placed
inside a VPC only when its `db_subnet_group_name` explicitly references one group's
`name`, the group's complete `subnet_ids` match managed Subnet `id` values, and the
cluster's complete `vpc_security_group_ids` match managed Security Group `id` values.
Every subnet and SG must have a verified reference to the same managed VPC.
Missing/default-only SG or subnet configuration leaves placement unproven.

Instance `cluster_identifier` must explicitly reference one cluster's
`cluster_identifier` with the same known value. The deprecated DocumentDB cluster
`id` (documented as an ARN) is not accepted as an identifier alias. Verified
instances appear inside their cluster without adding semantic subnet containment.
Membership and cluster networking are independent: a cluster with incomplete
network evidence can still have proven members. Shared subnet groups retain one
identity and are not embedded in a consuming cluster.

All endpoints must be current managed AWS resources with compatible provider,
account and known region evidence. Unknown/conflicting values, extra literal set
members, dynamic or ambiguous references, wrong output attributes, deleted/deposed
or data endpoints and filtered evidence retain ordinary dependency/fallback display.
Named states do not borrow proofs. Addresses, actions and lifecycle metadata remain
visible exactly once. Passwords, endpoint strings and arbitrary attributes are not
rendered. Elastic/global clusters, snapshots, parameter contents and inherited
instance encryption summaries are outside this contract.

Contracts were checked against the pinned Provider 6.70.0
[cluster](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/docdb_cluster.html.markdown),
[instance](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/docdb_cluster_instance.html.markdown)
and [subnet group](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/docdb_subnet_group.html.markdown)
docs and schema. Network collections are sets; instance subnet-group output is
computed. See [fixture](../tests/fixtures/docdb-plan.json), [SVG](../examples/docdb.svg)
and [HTML](../examples/docdb.html). Regenerate HTML with
`cargo run --locked -- tests/fixtures/docdb-plan.json --format html -o examples/docdb.html`
inside Docker. Native/Wasm parity includes both output formats.

## Neptune (#698)

`aws_neptune_cluster`, `aws_neptune_cluster_instance` and
`aws_neptune_subnet_group` use the same conservative proof requirements as
[DocumentDB](#documentdb-697), with Neptune-specific endpoint types and paths.
Cluster `neptune_subnet_group_name` references the group's `name`; the group's
complete `subnet_ids` and cluster `vpc_security_group_ids` must identify managed
subnets and SGs in one proven VPC. The shared subnet group stays logical.

Instance `cluster_identifier` references cluster `cluster_identifier`, independently
of cluster network placement. If the instance configures `neptune_subnet_group_name`,
its verified group must equal the cluster's verified group; missing, unknown,
literal-only or mismatching configured evidence cannot establish membership.
An absent optional instance group does not prevent otherwise proven membership.
Neither the group nor membership assigns an individual subnet to the instance.

The pinned Provider 6.70.0 [cluster contract](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/neptune_cluster.html.markdown),
[instance contract](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/neptune_cluster_instance.html.markdown)
and [subnet group contract](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/neptune_subnet_group.html.markdown)
were checked against schema. Network collections are sets. RDS/DocumentDB types
and cluster `id` are not accepted as substitutes for these exact contracts.
Analytics, snapshots, global databases, parameters and encryption summaries remain
outside this scope. Credentials, graph data and endpoint strings are not rendered.

See [fixture](../tests/fixtures/neptune-plan.json), [SVG](../examples/neptune.svg)
and [HTML](../examples/neptune.html). Regenerate HTML inside Docker with
`cargo run --locked -- tests/fixtures/neptune-plan.json --format html -o examples/neptune.html`.
Native/Wasm parity covers SVG and HTML; lifecycle, named-state and filtering
boundaries follow the DocumentDB contract.

## DMS replication configuration (#699)

Provisioned `aws_dms_replication_instance` retains its independent spatial card.
Its `replication_subnet_group_id` must explicitly reference a managed
`aws_dms_replication_subnet_group` `id` or `replication_subnet_group_id` with the
same known value. Complete `subnet_ids` in that group and the instance's complete
`vpc_security_group_ids` must match explicit managed Subnet/SG `id` references.
Every network endpoint must have a proven VPC reference to the same VPC. The
instance is placed once in that VPC, even when one subnet is selected. Unknown,
missing/default-only, extra literal or mixed-VPC evidence leaves placement unproven.
The shared logical subnet group is not a network container; computed group VPC and
instance IP outputs cannot substitute for configured proof.

`aws_dms_replication_task` shows separate `Replication instance`, `Source endpoint`
and `Target endpoint` navigation. Inputs must reference the exact instance's
`replication_instance_arn` or an `aws_dms_endpoint`'s `endpoint_arn`, respectively,
with equal known values. Endpoint `endpoint_type` must be known `source` or `target`
for the corresponding task input. A generic `arn`, wrong output, matching literal
or opposite-direction endpoint cannot establish the relationship. Tasks and
endpoints stay independent logical resources; they do not inherit instance VPC
placement or become embedded database children.

All proof requires unique explicit current managed AWS identities, compatible
provider/account/known-region evidence and complete collections. Verified provider
account scope is retained for these four exact types. Deleted/deposed/data/ambiguous
or incompatible references keep conservative fallback and diagnostics. Filtering
invalidates incomplete placement proof; named states never borrow matching endpoints.
Actions, moved addresses and current/deposed coverage remain distinct and exactly once.
Serverless replication, `aws_dms_s3_endpoint`, certificates, KMS, credentials and
auxiliary migration relationships remain separate #863–#866. No database is
reconstructed from hostnames, URLs or connection strings. Passwords, SQL/table
mappings, task settings, IPs and runtime status are not rendered as summaries.

Pinned Provider 6.70.0 contracts:
[instance](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/dms_replication_instance.html.markdown),
[subnet group](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/dms_replication_subnet_group.html.markdown),
[endpoint](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/dms_endpoint.html.markdown),
[task](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/dms_replication_task.html.markdown).
Schema confirms subnet/SG sets; subnet group source assigns Terraform `id` from its
configured identifier. Instance and endpoint ARN output names differ from generic
`arn`. See [fixture](../tests/fixtures/dms-plan.json), [SVG](../examples/dms.svg)
and [HTML](../examples/dms.html), with positive/negative native/Wasm parity tests.
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/dms-plan.json --format html -o examples/dms.html`.

## Redshift Serverless (#695)

Workgroups are independent spatial resources. `namespace_name` must explicitly
reference a managed namespace's `namespace_name` with the same known value to show
`Namespace owner` navigation. Namespace remains a logical service identity; it is
not a VPC container and does not inherit the workgroup's network placement.

Workgroup `subnet_ids` and `security_group_ids` must completely match managed
Subnet and Security Group `id` values whose explicit VPC references agree. Unknown,
extra literal, missing/default-only or mixed-VPC evidence leaves placement unproven.
Endpoint access resources are spatial controllers with their own complete
`subnet_ids`/`vpc_security_group_ids` proof. They may be in a different VPC from the
workgroup. No placement is copied from `workgroup_name` or computed endpoint/ENI data.

Endpoint `workgroup_name` verifies the exact workgroup's `workgroup_name` for
`Workgroup owner` navigation. An explicit `owner_account` additionally requires a
matching verified workgroup account; unknown or foreign ownership cannot borrow a
same-named workgroup. Cross-account ownership is outside this initial contract.
Verified provider account scope is retained for all three supported types.
Namespaces and endpoints are not embedded into network owners by service references.

All references require unique current managed AWS identities, compatible
provider/account/known-region evidence and known matching exact output attributes.
The namespace/workgroup UUID outputs and `id` are not substituted for these named
outputs. Deleted/deposed, data, dynamic, ambiguous and filtered evidence retains
fallback display. Named states stay independent; actions and lifecycle identities
remain visible exactly once. Credentials, endpoint addresses, query/configuration
contents, effective access and runtime connectivity are excluded. Provisioned
Redshift #506 is unchanged; namespace IAM/KMS and snapshot semantics are separate.

Pinned Provider 6.70.0 [namespace](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/redshiftserverless_namespace.html.markdown),
[workgroup](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/redshiftserverless_workgroup.html.markdown)
and [endpoint](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/redshiftserverless_endpoint_access.html.markdown)
contracts and schema were checked: all network collections are sets; endpoint SG
input is `vpc_security_group_ids`, while workgroup uses `security_group_ids`.
See [fixture](../tests/fixtures/redshift-serverless-plan.json),
[SVG](../examples/redshift-serverless.svg) and [HTML](../examples/redshift-serverless.html).
Regenerate HTML inside Docker with
`cargo run --locked -- tests/fixtures/redshift-serverless-plan.json --format html -o examples/redshift-serverless.html`.
Both output formats are covered by native/Wasm parity.

## SageMaker inference configuration (#701)

Endpoints retain Spatial cards; endpoint configurations and models remain
independent Logical resources, including shared models and configurations.
Dedicated references require explicit unique current managed AWS endpoints,
matching known values and compatible provider/account/region/state evidence:

| Consumer input | Managed endpoint output | Contract |
| --- | --- | --- |
| Endpoint `endpoint_config_name` | Endpoint configuration `name` | One matching known name; ARN/ID are not substitutes. |
| Endpoint configuration `production_variants.model_name` | Model `name` | Every variant must have a known model name and the complete referenced model set must match. Multiple variants may reuse one model. |
| Model `execution_role_arn` | IAM Role `arn` | Compatible account/binding; IAM role is global, so regional equality is not required for the role. |
| Model `vpc_config.subnets` | Subnet `id` | One VPC configuration list block with a complete nonempty subnet set. |
| Model `vpc_config.security_group_ids` | Security Group `id` | Complete nonempty SG set in the same configuration block. |

The pinned schema makes production variants a list of blocks and `model_name`
optional for inference components. An omitted model, unknown member or partially
literal collection leaves the production-model reference unverified. Distinct
managed models with the same output name are ambiguous. Shadow variants and
inference-component relationships remain outside this initial contract.

Only the model receives VPC context, after both network collections are verified
and every subnet/SG has explicit matching VPC-ID proof for one current managed VPC.
This is hosting/batch connection configuration; neither an endpoint nor its
configuration inherits that VPC. A subnet/SG is not a service owner, and consuming
a shared model/configuration does not embed it. Missing or conflicting VPC proof
retains regional fallback and individually verified references. Account scope uses
existing verified provider evidence; ordinary JSON does not manufacture an account.

Unknown, ambiguous, data, deleted/deposed, foreign-provider and filtered targets
retain conservative fallback. Actions, identities, named-state boundaries and
exactly-once coverage remain intact. Containers, environment variables, model
artifact URIs, policy contents, Studio/training jobs and runtime inference are not
interpreted or exposed. No network reachability or health claim is made.

Pinned Provider 6.70.0 contracts:
[endpoint](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/sagemaker_endpoint.html.markdown),
[configuration](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/sagemaker_endpoint_configuration.html.markdown),
[model](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/sagemaker_model.html.markdown).
[Tests](../tests/unit/semantic/sagemaker.rs) and the synthetic
[fixture](../tests/fixtures/sagemaker-plan.json) cover positive/negative evidence;
native/Wasm parity covers SVG/HTML and incomplete model/network collections.
Examples: [SVG](../examples/sagemaker.svg), [HTML](../examples/sagemaker.html).
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/sagemaker-plan.json --format html -o examples/sagemaker.html`.
