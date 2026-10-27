# AWS semantic coverage audit

The [gap register](aws-semantic-gaps.md) covers every **1,742 managed type in 249 service packages** from the pinned AWS Provider 6.70.0 inventory. Basic role, display and scope eligibility are implemented for that denominator. This is not 1,742 complete architecture integrations.

## Reading the register

Each exact type has current managed defaults, a semantic status, missing capability or explicit deferral, priority, issue/disposition and schema input candidates. Official contracts are linked at commit `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`. Generic Terraform dependencies remain available in every category.

- **Implemented** describes a named capability only: classification, a verified ownership rule, a particular reference path, or an allowlisted summary. It never means all attributes of a type are supported.
- **Partial** retains bounded existing behavior. The [AWS contracts](aws.md), [routes](routes.md), [components](components.md) and linked completed issues define its evidence and exclusions. An implementation-file mention can be an endpoint or exclusion, not a rule originating from that type.
- **Basic-only** establishes no dedicated semantic behavior in this audit. A spatial card and service label alone do not prove containment, ownership or associations.
- **Intentionally unsupported** covers value/policy interpretation and runtime inference, independently of basic display. **Deferred** is a backlog disposition, not proof that an attribute can never support a relationship.

P1 prioritizes network placement and explicit service relationships with high architecture value. P2 covers bounded workload/storage configuration chains. P3 retains current defaults until a concrete use case and exact evidence contract justify additional semantics. P3 is not a claim of completeness or a prohibition on future implementation. The initial triage selects 15 bounded service issues (#688–#702); other services remain explicitly deferred per type rather than being hidden from the denominator.

## Network connectivity and traffic protection review (#716)

Reviewed planorama commit: `d906716ae153a3d5bd320e64a6205f835850489b`.
All 68 managed types in `directconnect`, `networkmanager`, `networkfirewall`,
`vpclattice`, `networkflowmonitor` and `networkmonitor` have six separate
dimensions, exact fields and collection shapes in [decisions](aws-semantic-decisions.json).
The pinned Provider source archive identifies commit `5e13eed`, and the schema
is checked against the inventory SHA-256 before inspecting fields. Resource docs
are linked per type at the full Provider commit. These are historical conclusions;
later implementations update current gap text without rewriting this baseline.

At this commit all 68 types have basic Logical presentation. Runtime evidence is
[basic defaults](../src/provider/aws/basic_entries.rs),
[classification](../src/provider/aws/classification.rs),
[presentation](../src/provider/aws.rs), [scope](../src/provider/aws/scope.rs),
[unknown scope reasons](../src/provider/aws/basic.rs),
[containment](../src/provider/aws/containment/indirect.rs),
[dedicated references](../src/provider/aws/service_references.rs) and
[summary allowlists](../src/provider/aws/encryption.rs).
The [runtime matrix](resource-support.md) and
[inventory tests](../tests/unit/aws_inventory.rs) verify basic support; there
are no dedicated behavioral tests to borrow from adjacent services. Generic
Terraform dependencies do not establish dedicated service semantics.

Existing #693 owns VPC Firewall placement, policy/rule-group references and logging
ownership/destinations. Other actionable gaps are #786 (Direct Connect), #787
(hosted/cross-account DX), #788 (Network Manager topology), #789 (Cloud WAN),
#790 (Lattice services), #791 (Lattice resources), #792 (Firewall auxiliaries)
and #793 (network monitors). Source/helper types belong to these issues; endpoint
audits #713, #714, #715, #719, #721, #726 and #727 must not duplicate them.
No completed #688–#702 implementation is reopened.

Contract distinctions checked against the pinned documentation and schema:

- DX connection locations, public route-prefix sets and BGP addresses are not
  subnet/resource identities. Connection/LAG/gateway architecture deserves a
  separate card review; configuration helpers do not all become cards.
  Hosted allocator and accepter are separate lifecycle identities even where
  their IDs match. Confirmation removal does not delete the hosted connection.
  Hosted `connection_region` replaces deprecated output `region`; provider-region
  overrides cannot substitute for that target evidence. DX gateway is global.
  Some DX docs mention `bgp_asn_long` where a particular schema still only has
  `bgp_asn`; no ASN summary is added. Unmarked BGP auth strings remain secrets.
- Network Manager management objects remain global. `subnet_arns` and edge-location
  collections do not make the core network a regional VPC. Device `aws_location`
  is separate from physical site coordinates. NO_ENCAP peers can carry subnet
  ARNs; GRE CIDRs cannot identify subnets. Accepters require an attachment-type
  discriminator; Connect attachment exports `attachment_id`, while other
  attachments expose `id`. Routing labels and JSON policy segments are not
  independently managed Terraform objects.
- Firewall VPC/subnet mapping and TGW/AZ mapping are different variants. Policy
  stateful/stateless reference blocks are sets under a single list block; group
  type must agree. Computed firewall endpoints are not Terraform resources.
  Logging uses a destination-type discriminator and a string map with `bucketName`,
  `logGroup` or `deliveryStream`; prefixes and arbitrary map entries stay private.
  Rule-group prefix references have the extra `ip_set_reference` block under
  `rule_group.reference_sets.ip_set_references`. TLS `certificate_authority_arn`
  identifies an imported ACM certificate, not an ACM PCA authority. Container
  monitoring requires ECS/EKS type and same-account/region cluster evidence.
- Lattice listener `listener_id` is distinct from composite Terraform identity.
  Listener rules require service/listener agreement and an action discriminator.
  Target IDs vary between instance ID, Lambda ARN, ALB ARN and IP; IP/DNS values
  cannot reconstruct Terraform endpoints. Resource-configuration docs mislabel
  their `id`/`arn` as resource-gateway outputs; the registered
  `internal/service/vpclattice/resource_configuration.go` resource confirms the
  distinct resource-configuration identity. CHILD/GROUP ownership and gateway
  subnet context need separate proofs. Domain TXT outputs do not create DNS records.
- Flow Monitor selectors have explicit types; account IDs, regions and AZs are
  scope selectors rather than managed resource identities. Network Monitor probe
  `source_arn` is a subnet; destination is an IP and monitor linkage uses
  `monitor_name`, not ARN. Monitoring does not imply endpoint health or reachability.

No reviewed type shares a verified managed alias factory. Registered data sources
remain lookup contracts outside this managed denominator; managed basic defaults
and future ownership rules do not transfer to them. Dedicated links require unique
explicit references, matching known output values, complete collections and compatible
provider/account/region/state evidence. Current configuration must not create
historical/deposed links. Shared resources keep independent coverage.
Per-type summary deferrals require a bounded allowlist before revisiting; secrets,
rule/policy contents, runtime status and arbitrary values remain excluded.

This audit changes no rendering input or runtime geometry. Docker checks cover
the pinned-contract verifier, semantic register/tests and support-matrix consistency;
SVG regeneration and Layout quality do not apply to this audit-only change.

## Method and limits

The runtime-generated [support matrix](resource-support.md) supplies role, display and conditional scope. All pinned managed schemas were traversed for configurable, non-sensitive identifier/name/network fields. The compact [schema evidence](aws-semantic-evidence.json) records **paths only**, with the source schema SHA-256. Candidate names are an audit index, not inferred endpoint types. Nested JSON policy strings, URI contents and dynamically selected references are not interpreted. No candidate field is automatically added to runtime behavior.

The register combines this inventory-wide mechanical review with explicit [review decisions](aws-semantic-decisions.json) for prioritized and existing bounded behavior. Unreviewed semantic details remain deferred. This is a maintained gap/triage inventory, not an assertion that every official field has received manual semantic verification. The conservative scan can miss fields with unconventional names and includes self-names that may not be relationships; an empty candidate list does not establish complete support. Each implementation issue must verify actual output types, collections, endpoint identity and known-value proofs before adding a rule.

Dimensions are assessed separately:

| Dimension | Current baseline and remaining decision |
| --- | --- |
| Architecture placement | Copy actual Spatial/Logical defaults; promote an independent service object only for a justified architecture use case. |
| Ownership / embedded presentation | Require a uniquely verified owner and preserve shared identities; a reference alone is not ownership. Existing EFS/KMS/Secrets/Glue/CA work remains bounded. |
| Network containment | Complete subnet/SG/VPC evidence and compatible scope required; service ownership does not place a resource in a subnet. |
| Dedicated references / associations | Verify exact input/output fields and lifecycle/state identities; keep generic references for unproven endpoints. |
| Scope evidence | Regional is eligibility, not verified account/region; global objects and cross-region target references are separate. Unknown scope is intentional pending a dedicated contract. |
| Safe summaries | Only explicit allowlists with known/unknown/sensitive semantics; schema availability alone never permits value rendering. |

## Representative implementation and official-contract checks

The following paths were checked against the pinned schema and resource documentation, then compared with current provider code. The exact type's register row links its official document. These checks establish gaps; they do not add runtime evidence rules.

| Exact type | Pinned input contract | Actual behavior / decision |
| --- | --- | --- |
| `aws_msk_cluster` | `broker_node_group_info.client_subnets`, `broker_node_group_info.security_groups`, `configuration_info.arn` | Verified common-VPC subnet/SG and configuration ARN/revision rules (#688); other dimensions retain the historical #721 review. |
| `aws_mq_broker` | `subnet_ids`, `security_groups`, `configuration.id` | Verified common-VPC subnet/SG and configuration ID/revision rules (#689); other dimensions retain the historical #721 review. |
| `aws_apprunner_service` | `network_configuration.egress_configuration.vpc_connector_arn` | Verified VPC-egress connector ARN #694; service remains spatial without customer subnet ownership. |
| `aws_apprunner_vpc_connector` | `subnets`, `security_groups` | Verified complete subnet/SG references and common-VPC context #694; shared connector remains logical. |
| `aws_redshiftserverless_workgroup` | `namespace_name`, `subnet_ids`, `security_group_ids` | Verified namespace/workgroup owner navigation and independent workgroup/endpoint VPC proof #695; provisioned Redshift #506 remains separate. |
| `aws_opensearchserverless_vpc_endpoint` | `vpc_id`, `subnet_ids`, `security_group_ids` | Basic endpoint; policy JSON is not collection ownership evidence. #696. |
| `aws_docdb_cluster` | `db_subnet_group_name`, `vpc_security_group_ids` | Verified group/SG common-VPC placement and instance membership #697; other dimensions retain historical #717 review. |
| `aws_neptune_cluster` | `neptune_subnet_group_name`, `vpc_security_group_ids` | Verified common-VPC group/SG placement and instance membership #698; other dimensions retain historical #717 review. |
| `aws_dms_replication_instance` | `replication_subnet_group_id`, `vpc_security_group_ids` | Basic spatial instance; no endpoint/database reconstruction from strings. #699. |
| `aws_batch_compute_environment` | `compute_resources.subnets`, `compute_resources.security_group_ids` | Basic spatial compute environment; scheduling policy stays logical. #700. |
| `aws_sagemaker_endpoint` | `endpoint_config_name` | Basic spatial endpoint; model ownership is not network containment. #701. |
| `aws_fsx_lustre_file_system` | `subnet_ids`, `security_group_ids`, `kms_key_id` | Basic spatial filesystem; network/key references missing. #702. |
| `aws_dynamodb_table` | `replica.region_name`, `server_side_encryption.kms_key_arn` | Spatial table, explicit Backup endpoint and [table/configuration navigation](dynamodb.md) exist. Import/export/restore references are tracked in #744; Lambda stream identity coordinates with #714. Inline replica expansion and at-rest summaries remain explicitly deferred in the #718 review. |
| `aws_elasticache_replication_group` | `subnet_group_name`, `global_replication_group_id`, `user_group_ids` | Common-VPC placement via subnet group exists (#360); global/user/parameter configuration references are implemented by #691. No at-rest summary. |

Implementation cross-checks: [classification](../src/provider/aws/classification.rs), [basic defaults](../src/provider/aws/basic_entries.rs), [placement](../src/provider/aws.rs), [containment](../src/provider/aws/containment/indirect.rs), [service references](../src/provider/aws/service_references.rs), [Backup endpoints](../src/provider/aws/backup.rs), [messaging](../src/provider/aws/messaging.rs) and [encryption allowlist](../src/provider/aws/encryption.rs). In particular, #601's summaries cover S3/SQS/RDS, not every service with encryption attributes. This distinction prevents borrowing adjacent-service support.

## Placement and deferred scope decisions

Retain existing independent MSK/MQ/DocumentDB/Neptune/DMS/FSx/SageMaker endpoint cards while adding evidence in their bounded issues. Retain Batch compute-environment cards; review Network Firewall firewalls for independent cards. App Runner connectors, database subnet groups, namespace/configuration objects and policies need separate ownership/network decisions; do not promote all objects from a service together. Existing embedded EFS access points, KMS aliases and Secrets Manager versions remain examples of proven presentation ownership, not a default for every reference.

MSK replicators (topics are tracked in #754), Network Firewall endpoint associations/transit-gateway/TLS variants (now tracked by #792), non-Lustre FSx, DocumentDB/Neptune global variants, DMS serverless and SageMaker Studio/training remain deferred outside the initial issue scopes. DocumentDB Elastic and Neptune Analytics now have bounded follow-up #766 from the #717 review. Their exact types and schema candidates remain in the register. Runtime health, effective IAM/firewall policy, packet routing, credentials, item data and arbitrary attribute dumping are intentionally unsupported. Encryption summaries for additional services require a separate allowlist review; do not copy a boolean from a similarly named field.

## Containers and application hosting review (#713)

Reviewed planorama commit: `f30289f73269a8a348818dd6e2a8bc44c22f22aa`.
All 57 managed types in `ecs`, `eks`, `ecr`, `ecrpublic`, `apprunner`,
`elasticbeanstalk`, `appmesh` and `servicediscovery` have six separate dimensions
in [decisions](aws-semantic-decisions.json). Each records the pinned official
document, inspected fields and schema shapes. These are historical conclusions;
subsequent implementations update current gap text without rewriting the baseline.

Implementation evidence is [classification](../src/provider/aws/classification.rs),
[basic defaults](../src/provider/aws/basic_entries.rs),
[presentation](../src/provider/aws.rs), [scope](../src/provider/aws/scope.rs),
[references](../src/provider/aws/service_references.rs),
[network proof](../src/provider/aws/network.rs),
[spanning placement](../src/provider/aws/containment/spanning.rs),
[VPC placement](../src/provider/aws/containment/vpc_scope.rs),
[SGs](../src/provider/aws/security_groups.rs), and
[summary allowlists](../src/provider/aws/encryption.rs).
Behavioral evidence is [ECR reference tests](../tests/unit/semantic/service_references.rs),
[ECS tests](../tests/unit/semantic/ecs_clusters.rs),
[EKS tests](../tests/unit/semantic/eks.rs),
[basic inventory tests](../tests/unit/aws_inventory.rs) and the runtime support matrix.
Basic-only types do not inherit adjacent services' tested semantics.

Existing ECR repository-policy/lifecycle references, ECS cluster navigation and
service network placement, and EKS cluster/role references and network placement
remain bounded by [their contracts](aws.md#ecr-repositories). They do not establish
embedded service ownership. EKS node-group/Fargate subnet proof is stricter than
legacy cluster/service placement; this audit does not silently broaden the latter.

App Runner networking is implemented by #694; historical review objects preserve
the earlier baseline. New bounded work is #775 (App Mesh), #776
(App Runner configuration), #777 (ECR Public and registry configuration), #778
(ECS capacity), #779 (ECS workloads), #780 (EKS auxiliary configuration), #781
(Beanstalk) and #782 (Cloud Map). Source/helper relationships belong to those
issues; endpoint audits #709, #710, #712, #715, #719, #726, #727 and #730 must not
duplicate them. Each remaining deferral records a concrete contract/use-case
prerequisite; policy, payload and runtime interpretation remain excluded.

Contract distinctions checked against Provider 6.70.0 at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:

- App Runner egress requires `egress_type = VPC`; the connector ARN includes its
  revision. Connector subnet/SG inputs are schema sets despite the docs calling
  them lists. Connector context never places the service in a customer subnet.
  Ingress `service_arn` is separate from its endpoint/VPC pair. Deployment and
  autoscaling resources stay logical; image URLs are not ECR reference evidence.
- App Mesh names require mesh and owner-account agreement. Weighted targets and
  mutually exclusive virtual-node/router providers require protocol/variant
  discrimination. Mesh ownership is not network containment; DNS, SDS names,
  certificate paths and routing predicates do not identify managed endpoints.
- ECR Public is restricted to the us-east-1 API; current conditional Regional
  eligibility is not global-scope proof. Its `repository_name` differs from private
  policy `repository`. Registry IDs/prefixes, `ROOT` and filters do not identify
  managed repositories. KMS references need the encryption discriminator.
- ECS capacity providers distinguish ASG from managed-instances configuration.
  `FARGATE`/`FARGATE_SPOT` are builtins, not managed endpoints. Daemon provider ARNs
  form a set and its task-definition ARN includes revision; the API can report a
  stale revision during deployment. Express `network_configuration` is a typed
  list-of-object attribute with subnet/SG sets, not a nested schema block; omitted
  default subnets cannot be reconstructed. Its `current_deployment` is deprecated.
- EKS access policy ARNs are not IAM policy ARNs. Access associations require the
  cluster/principal tuple. Pod namespace/service-account selectors are not managed
  Kubernetes resources. Node-group launch-template versions need explicit revision
  proof; computed ASGs and OIDC URLs do not establish configured references.
- Beanstalk versions/templates are application-scoped; shared versions are not
  embedded in environments. Network settings are opaque namespace/name/value
  tuples, not direct subnet inputs. Computed ASG/ELB/instance lists cannot create
  managed objects. Cloud Map direct/DNS namespace alternatives must agree;
  registrations' arbitrary attributes do not identify EC2 instances. Its custom
  health-check configuration is deprecated and not runtime health evidence.

No reviewed type is a verified managed alias. Data schemas remain separate lookup
contracts; managed ownership does not extend to data/deposed identities. References
require explicit unique compatible endpoints, correct output attributes, matching
known nonempty values and complete collections. Provider/account/region, named-state,
deletion and filtering boundaries remain intact. Scope eligibility alone supplies
neither account identity nor member region. Environment/secret maps, container JSON,
policy bodies and catalog payloads are excluded even without sensitive markers.

This audit changes no runtime defaults, rendering input or geometry. Docker checks
cover all pinned contracts, register consistency, audit tests and support-matrix
consistency; SVG regeneration and Layout quality are not applicable.

## NoSQL, caches and time-series review (#718)

Reviewed planorama commit: `bebadbbeac605bae9efd6810957a89130616d947`.
The 39 exact managed types owned by #718 now have a `review` object in
[decisions](aws-semantic-decisions.json), with separate placement, ownership,
network, references, scope and summary dispositions. Each records the official
resource document at Provider commit `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`,
checked input paths and their full collection ancestry from the pinned schema.
The schema hash matches the inventory. This review changes no runtime defaults. Review objects preserve the historical
baseline at the recorded commit; current status/gap text includes subsequent
implementation work such as #691.

Implementation evidence: [basic defaults](../src/provider/aws/basic_entries.rs),
[classification](../src/provider/aws/classification.rs),
[scope](../src/provider/aws/scope.rs),
[DynamoDB verification](../src/provider/aws/dynamodb.rs),
[service references](../src/provider/aws/service_references.rs),
[indirect placement](../src/provider/aws/containment/indirect.rs),
[VPC scope](../src/provider/aws/containment/vpc_scope.rs),
[network verification](../src/provider/aws/network.rs),
[Backup](../src/provider/aws/backup.rs), and
[encryption allowlist](../src/provider/aws/encryption.rs).
Behavioral evidence: [DynamoDB tests](../tests/unit/semantic/dynamodb.rs),
[ElastiCache integration tests](../tests/integration/elasticache.rs),
[indirect placement tests](../tests/unit/semantic/indirect.rs),
[VPC scope tests](../tests/unit/semantic/vpc_scope.rs),
[encryption tests](../tests/unit/semantic/encryption.rs) and the generated support
matrix. Basic-only types have no dedicated behavior to claim from these tests.

Completed #690 table/configuration navigation, #360 subnet-group placement,
#516 serverless network proof and #508 Backup endpoints remain bounded. The #691 implementation adds cache global/user/parameter configuration. New bounded work is #739 (DAX),
#740 (MemoryDB), #741 (Keyspaces), #742 (InfluxDB), #743 (Timestream) and #744
(DynamoDB data movement). Cross-service sources belong to these issues; endpoint
audits #714, #719, #721, #726, #727 and #730 must not duplicate them.

Contract distinctions checked during review:

- DynamoDB legacy V1 `replica` is a set; V2 `global_table_arn` is a string.
  Inline indexes/replicas/witness regions do not identify managed Terraform nodes.
  GSI deprecated hash/range keys and newer key-schema blocks remain data schema,
  not separate architecture resources. Table items contain arbitrary JSON.
- Cache `user_ids`, `user_group_ids`, subnet and SG attributes are sets.
  InfluxDB documentation calls its subnet/SG attributes lists, but both managed
  schemas use sets. Known, complete endpoint evidence is required for every member.
- ElastiCache replication-group `at_rest_encryption_enabled` is a string in the
  pinned schema; the global group's similarly named output is computed. Neither
  is in the existing encryption-summary allowlist. Passwords and access strings
  must not enter summaries even where the schema omits a sensitive marker.
- Global group suffixes are not full IDs. MemoryDB multi-region parameter names
  cannot resolve to ordinary regional parameter groups. InfluxDB's parameter-group
  identifier has no corresponding managed type in this inventory; its initial
  `bucket` is an InfluxDB bucket, not S3.
- Timestream scheduled queries use `execution_role_arn`; destination identity
  requires both database and table. SQL, computed run histories and column mappings
  are not reference evidence. Timestream database KMS IDs require key ARNs, excluding
  alias ARNs. QLDB's `AWS_OWNED_KMS_KEY` is a sentinel, not a key endpoint.

No listed type is a verified alias of another managed type. Data-only entries in
the support matrix do not inherit these managed decisions. Shared users, ACLs,
parameter groups and network selectors remain independent. Scope eligibility is
not account evidence: basic fallback clears account, regional types require known
provider/resource-region evidence, and global objects do not propagate their
provider region into members. Plain ARN substrings never supply ownership proof.
The existing reference contract requires explicit unique current managed endpoints,
correct output attributes, equal known nonempty values, compatible bindings and
state identity; current configuration cannot establish historical/deposed links.

## Streaming, messaging and workflows review (#721)

Reviewed planorama commit: `de2686ced20f383ee48ec65b1354b643e2fe659c`.
All 55 managed types in the fixed #721 scope have separate placement, ownership,
network, reference, scope and summary decisions in
[the decision register](aws-semantic-decisions.json). Each review records the
pinned official resource document and checked schema paths with their complete
collection ancestry. These are historical audit conclusions, not runtime changes.

Implementation and behavioral evidence:

- [Basic defaults](../src/provider/aws/basic_entries.rs),
  [classification](../src/provider/aws/classification.rs),
  [scope](../src/provider/aws/scope.rs), [network proof](../src/provider/aws/network.rs)
  and [basic tests](../tests/unit/aws_inventory.rs) establish the baseline.
- [Messaging](../src/provider/aws/messaging.rs),
  [subscription tests](../tests/unit/semantic/messaging.rs),
  [event-source tests](../tests/unit/semantic/event_sources.rs) and
  [target tests](../tests/unit/semantic/event_targets.rs) retain existing bounded
  SNS/SQS/Lambda/Kinesis and EventBridge behavior. Data endpoints supported by
  existing messaging contracts remain distinct from managed-type audit ownership.
- [Service references](../src/provider/aws/service_references.rs) and
  [their tests](../tests/unit/semantic/service_references.rs) retain Step Functions
  execution-role navigation. [Firehose](../src/provider/aws/firehose.rs) and
  [its tests](../tests/unit/semantic/firehose.rs) verify Kinesis input and
  extended-S3 delivery/backup discrimination. [Encryption](../src/provider/aws/encryption.rs)
  and [its tests](../tests/unit/semantic/encryption.rs) support SQS summaries;
  they do not establish encryption summaries for the other reviewed services.

Existing implementation scopes remain #688 (MSK), #689 (MQ) and #692
(Scheduler/Pipes). New bounded gaps are #748 (EventBridge configuration), #749
(Kinesis consumers/policies), #750 (MSK Connect), #751 (MWAA), #752 (Flink), #753
(SNS/SQS policy owners), #754 (MSK topic/channel/policy), #755 (Schemas) and #756
(Firehose MSK source/roles). These issues own the source/helper relationships;
endpoint audits should coordinate without duplicating them. Other dimensions
record concrete deferral reasons and revisit conditions per exact type.

Contract distinctions checked against Provider 6.70.0:

- MQ subnet/security-group attributes and MSK secret ARN collections are sets,
  despite documentation calling them lists. MQ `configuration` is a list block
  with a numeric revision; a matching ID alone cannot verify that revision.
- MSK Serverless `vpc_config` is a list with set-valued subnet/SG fields. Per-configuration network contexts must
  not be flattened into arbitrary subnet ownership. An MSK VPC connection's
  `vpc_id` belongs to its client, independently of its target cluster scope.
- MSK Connect plugin/worker references include revisions. Bootstrap server
  strings are not cluster identity. Channel topic references need same-cluster
  proof; replicators additionally require indexed source/target tuple evidence.
- Scheduler `target.arn` may be a universal service operation ARN; it is not
  necessarily a resource ARN. Pipes supports self-managed Kafka `smk://` sources.
  Task network blocks cannot establish placement of a schedule or pipe.
- EventBridge rules retain deprecated `is_enabled` alongside `state` contracts.
  Global endpoints require two same-named buses in distinct regions; a secondary
  region string is not a resource. Connection authorization and invocation VPC
  Lattice references are separate from network containment.
- SNS application subscription endpoints identify platform endpoints, not platform
  applications. SQS policy owners use queue URLs, not queue ARNs. Policy bodies,
  redrive JSON, schema content and message payloads are intentionally uninterpreted.
- Step Functions alias routes use published version ARNs, not ordinary state
  machine ARNs. Activity and state-machine encryption enums differ. Legacy SQL
  analytics and Flink application contracts are not aliases. MWAA log destinations
  are computed outputs, not explicit configured endpoint references.

No type in this scope is a verified resource alias. Basic regional eligibility
does not prove account ownership. Exact managed/data, named-state, current/deposed
and filtering distinctions stay unchanged. Network ownership and references need
explicit unique endpoint evidence with matching known values and compatible
scope; neither ARN substrings nor equal literal names establish them. Shared
configuration remains independent. Unknown evidence never becomes a runtime claim.

## Maintenance and verification

Inside Docker, regenerate the support matrix first, then the gap register:

```sh
sh scripts/update-resource-support.sh
node scripts/aws-semantic-audit.cjs
node scripts/aws-semantic-audit.cjs --check
node --test tests/support/aws-semantic-audit.test.cjs
```

To reproduce schema extraction, use the pinned acquisition procedure in [AWS inventory](aws-inventory.md), then:

```sh
node scripts/aws-semantic-audit.cjs --schema /tmp/aws-inventory-schema/schema.json
```

Representative fields can also be checked against a retained pinned source checkout:

```sh
node tests/support/verify-aws-semantic-contracts.cjs /tmp/aws-inventory-schema/schema.json SOURCE
```

`SOURCE` is the extracted official commit from the inventory acquisition procedure.
The verifier checks each representative path is configurable, prints its schema type,
and checks its leaf field is documented; the behavioral interpretation remains the
explicit manual review above. It performs no AWS calls.

The extractor rejects a schema hash or managed-type denominator mismatch. Ordinary CI checks the committed evidence, exact-type coverage, decision references and deterministic output offline. Update the explicit decisions when service work lands; a closed issue does not automatically upgrade semantic status. Review the scope/placement decisions when #615 identifies changed contracts. Runtime changes still require their normal tests, examples and Layout quality; this audit changes documentation/tooling only.

## Relational and graph database review (#717)

Reviewed planorama commit: `5d233e4a467f323b46082a35ede1467eec999898`.
All 59 managed types in `rds`, `docdb`, `docdbelastic`, `neptune`,
`neptunegraph`, `dsql` and `odb` have separate placement, ownership, network,
reference, scope and summary conclusions in [decisions](aws-semantic-decisions.json).
Their `review` objects record this historical baseline, exact reviewed fields,
collection ancestry and resource documentation at Provider commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`. Subsequent implementation updates the
current status/gap, while preserving the historical review.

Runtime evidence is [classification](../src/provider/aws/classification.rs),
[basic defaults](../src/provider/aws/basic_entries.rs), [scope](../src/provider/aws/scope.rs),
[indirect placement](../src/provider/aws/containment/indirect.rs),
[VPC placement](../src/provider/aws/containment/vpc_scope.rs),
[SG attachment](../src/provider/aws/security_groups.rs),
[network verification](../src/provider/aws/network.rs),
[membership](../src/provider/aws/members.rs),
[references](../src/provider/aws/service_references.rs),
[proxy validation](../src/provider/aws/rds_proxy.rs),
[Backup](../src/provider/aws/backup.rs) and [encryption](../src/provider/aws/encryption.rs).
Behavioral evidence is [membership tests](../tests/unit/semantic/members.rs),
[indirect tests](../tests/unit/semantic/indirect.rs),
[VPC tests](../tests/unit/semantic/vpc_scope.rs),
[proxy tests](../tests/unit/semantic/rds_proxy.rs),
[global tests](../tests/unit/semantic/rds_global.rs),
[reference tests](../tests/unit/semantic/service_references.rs),
[encryption tests](../tests/unit/semantic/encryption.rs) and the runtime support matrix.
Basic-only types have no dedicated behavior to claim from adjacent RDS tests.

The review preserves RDS subnet-group placement, instance presentation membership,
Global Cluster references, Proxy #514, Backup #508 and encryption #601. Legacy RDS
indirect placement does not implement the same strict complete-known-value proof
as newer service-specific network validators; its existing bounded behavior is
recorded without extending that claim to DocumentDB or Neptune.

Existing work is #697 (DocumentDB) and #698 (Neptune). Additional bounded gaps are
#762 (database configuration/IAM), #763 (snapshots/export), #764 (notifications),
#765 (proxy/custom endpoints), #766 (Elastic/Analytics), #767 (DSQL), #768 (ODB)
and #769 (RDS auxiliary/zero-ETL). These own the listed source/helper relationships;
endpoint audits #719, #721, #722, #726 and #727 must not duplicate them. Redshift
Serverless #695 owns its namespace/workgroup architecture, not RDS integration.

Contract distinctions checked against pinned docs and schema:

- RDS `aws_db_instance.id` is a resource ID, distinct from `identifier`.
  RDS cluster membership currently verifies the referenced cluster `id`.
  DocumentDB cluster `id` is documented as a deprecated ARN; new membership must
  use the verified `cluster_identifier` contract instead of copying RDS logic.
- Subnet/SG collections and event `source_ids` are sets. RDS notification input is
  `sns_topic`; DocumentDB/Neptune use `sns_topic_arn`. DocumentDB's argument list
  incorrectly says `sns_topic`, while its example and pinned schema use
  `sns_topic_arn`; the verifier checks that explicit example as evidence.
- Snapshot VPC/AZ/KMS fields are computed source provenance, not configured network
  containment. DocumentDB/Neptune snapshot docs also spell the copied-source output
  `source_db_cluster_snapshot_identifier`, while schema uses
  `source_db_cluster_snapshot_arn`; neither establishes a configured reference.
- DocumentDB instance subnet-group/KMS outputs are computed. Neptune instances have
  a configurable subnet-group name that must agree with their cluster. Neither
  instance membership nor a multi-subnet selector chooses an individual subnet.
- Global database creation source fields accept ARNs despite their identifier suffix.
  Computed global members cannot synthesize managed regional clusters. Global
  objects do not propagate their provider region to members.
- DSQL peer clusters are a set inside a list block. `AWS_OWNED_KMS_KEY` is a sentinel;
  Neptune Analytics uses `AWS_OWNED_KEY`. Neither is a KMS endpoint. Witness region
  and graph public connectivity are not Terraform resource or reachability proof.
- Neptune Analytics endpoint SG configuration is not returned by its API. ODB network
  CIDRs and OCI IDs do not identify EC2 subnets. ODB VM cluster infrastructure/network
  ID and ARN forms are paired alternatives; peering documentation contains misleading
  connection-ID examples and cannot justify string-based endpoint inference.
- ODB Secrets Manager integration enables an account/region Oracle service role;
  its role ARN is computed, with no configured secret/database owner. RDS reserved
  instances are financial reservations, and RDS certificates configure regional
  account defaults rather than managed ACM certificates.
- Proxy target group `name` is distinct from its composite ID; database targets
  require proxy/group consistency and exclusive instance/cluster selection. RDS
  integration's deprecated `id` is an ARN, distinct from `integration_identifier`.

No reviewed type is a verified managed alias. Registered data schemas remain
separate lookup contracts; managed defaults and ownership decisions do not extend
to them. New reference contracts require explicit unique compatible endpoints,
matching known output values and complete collections. Named-state, provider,
account/region, current/deposed, deletion and filtering boundaries remain intact.
Shared parameter/option/subnet groups remain independent. Passwords, policy bodies,
SQL, connection endpoints, clone data and runtime health are excluded. Type-specific
remaining deferrals state their contract or use-case prerequisite in the decisions.

This audit changes no runtime, rendering input or geometry; SVG regeneration and
Layout quality are not required. Docker checks include the complete pinned-contract
verifier, semantic register/tests and resource-support consistency.
