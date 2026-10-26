# AWS semantic coverage audit

The [gap register](aws-semantic-gaps.md) covers every **1,742 managed type in 249 service packages** from the pinned AWS Provider 6.70.0 inventory. Basic role, display and scope eligibility are implemented for that denominator. This is not 1,742 complete architecture integrations.

## Reading the register

Each exact type has current managed defaults, a semantic status, missing capability or explicit deferral, priority, issue/disposition and schema input candidates. Official contracts are linked at commit `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`. Generic Terraform dependencies remain available in every category.

- **Implemented** describes a named capability only: classification, a verified ownership rule, a particular reference path, or an allowlisted summary. It never means all attributes of a type are supported.
- **Partial** retains bounded existing behavior. The [AWS contracts](aws.md), [routes](routes.md), [components](components.md) and linked completed issues define its evidence and exclusions. An implementation-file mention can be an endpoint or exclusion, not a rule originating from that type.
- **Basic-only** establishes no dedicated semantic behavior in this audit. A spatial card and service label alone do not prove containment, ownership or associations.
- **Intentionally unsupported** covers value/policy interpretation and runtime inference, independently of basic display. **Deferred** is a backlog disposition, not proof that an attribute can never support a relationship.

P1 prioritizes network placement and explicit service relationships with high architecture value. P2 covers bounded workload/storage configuration chains. P3 retains current defaults until a concrete use case and exact evidence contract justify additional semantics. P3 is not a claim of completeness or a prohibition on future implementation. The initial triage selects 15 bounded service issues (#688–#702); other services remain explicitly deferred per type rather than being hidden from the denominator.

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
| `aws_mq_broker` | `subnet_ids`, `security_groups`, `configuration.id` | Basic spatial card; no dedicated placement/configuration references. #689. |
| `aws_apprunner_service` | `network_configuration.egress_configuration.vpc_connector_arn` | Spatial service; connector reference is egress configuration, not service subnet ownership. #694. |
| `aws_apprunner_vpc_connector` | `subnets`, `security_groups` | Logical basic object; verify network context without turning every configuration into a card. #694. |
| `aws_redshiftserverless_workgroup` | `namespace_name`, `subnet_ids`, `security_group_ids` | Basic spatial workgroup, unlike existing provisioned Redshift #506. #695. |
| `aws_opensearchserverless_vpc_endpoint` | `vpc_id`, `subnet_ids`, `security_group_ids` | Basic endpoint; policy JSON is not collection ownership evidence. #696. |
| `aws_docdb_cluster` | `db_subnet_group_name`, `vpc_security_group_ids` | Basic spatial cluster; RDS rules do not automatically apply. #697. |
| `aws_neptune_cluster` | `neptune_subnet_group_name`, `vpc_security_group_ids` | Basic spatial cluster; instance membership and placement need separate proof. #698. |
| `aws_dms_replication_instance` | `replication_subnet_group_id`, `vpc_security_group_ids` | Basic spatial instance; no endpoint/database reconstruction from strings. #699. |
| `aws_batch_compute_environment` | `compute_resources.subnets`, `compute_resources.security_group_ids` | Basic spatial compute environment; scheduling policy stays logical. #700. |
| `aws_sagemaker_endpoint` | `endpoint_config_name` | Basic spatial endpoint; model ownership is not network containment. #701. |
| `aws_fsx_lustre_file_system` | `subnet_ids`, `security_group_ids`, `kms_key_id` | Basic spatial filesystem; network/key references missing. #702. |
| `aws_dynamodb_table` | `replica.region_name`, `server_side_encryption.kms_key_arn` | Spatial table, explicit Backup endpoint and [table/configuration navigation](dynamodb.md) exist. Import/export/restore references are tracked in #744; Lambda stream identity coordinates with #714. Inline replica expansion and at-rest summaries remain explicitly deferred in the #718 review. |
| `aws_elasticache_replication_group` | `subnet_group_name`, `global_replication_group_id`, `user_group_ids` | Common-VPC placement via subnet group exists (#360); global/user/parameter configuration references are implemented by #691. No at-rest summary. |

Implementation cross-checks: [classification](../src/provider/aws/classification.rs), [basic defaults](../src/provider/aws/basic_entries.rs), [placement](../src/provider/aws.rs), [containment](../src/provider/aws/containment/indirect.rs), [service references](../src/provider/aws/service_references.rs), [Backup endpoints](../src/provider/aws/backup.rs), [messaging](../src/provider/aws/messaging.rs) and [encryption allowlist](../src/provider/aws/encryption.rs). In particular, #601's summaries cover S3/SQS/RDS, not every service with encryption attributes. This distinction prevents borrowing adjacent-service support.

## Placement and deferred scope decisions

Retain existing independent MSK/MQ/DocumentDB/Neptune/DMS/FSx/SageMaker endpoint cards while adding evidence in their bounded issues. Retain Batch compute-environment cards; review Network Firewall firewalls for independent cards. App Runner connectors, database subnet groups, namespace/configuration objects and policies need separate ownership/network decisions; do not promote all objects from a service together. Existing embedded EFS access points, KMS aliases and Secrets Manager versions remain examples of proven presentation ownership, not a default for every reference.

MSK replicators (topics are tracked in #754), Network Firewall endpoint associations/transit-gateway/TLS variants, non-Lustre FSx, DocumentDB elastic/global variants, Neptune Analytics, DMS serverless and SageMaker Studio/training remain deferred outside the initial issue scopes. Their exact types and schema candidates remain in the register. Runtime health, effective IAM/firewall policy, packet routing, credentials, item data and arbitrary attribute dumping are intentionally unsupported. Encryption summaries for additional services require a separate allowlist review; do not copy a boolean from a similarly named field.

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
