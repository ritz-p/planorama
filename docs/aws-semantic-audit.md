# AWS semantic coverage audit

The [gap register](aws-semantic-gaps.md) covers every **1,742 managed type in 249 service packages** from the pinned AWS Provider 6.70.0 inventory. Basic role, display and scope eligibility are implemented for that denominator. This is not 1,742 complete architecture integrations.

監査方法と現行判断の入口です。[完了済み監査の履歴](aws-semantic-history.md) は調査時点の証拠を保存し、現在の対応範囲の保証には使いません。

- [台帳の読み方](#reading-the-register) / [監査方法と限界](#method-and-limits)
- [現行実装と公式契約の照合](#representative-implementation-and-official-contract-checks) / [未対応・再検討条件](#placement-and-deferred-scope-decisions)
- [更新・検証](#maintenance-and-verification) / [過去の監査](aws-semantic-history.md)

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
| `aws_mq_broker` | `subnet_ids`, `security_groups`, `configuration.id` | Verified common-VPC subnet/SG and configuration ID/revision rules (#689); other dimensions retain the historical #721 review. |
| `aws_apprunner_service` | `network_configuration.egress_configuration.vpc_connector_arn` | Verified VPC-egress connector ARN #694; service remains spatial without customer subnet ownership. |
| `aws_apprunner_vpc_connector` | `subnets`, `security_groups` | Verified complete subnet/SG references and common-VPC context #694; shared connector remains logical. |
| `aws_redshiftserverless_workgroup` | `namespace_name`, `subnet_ids`, `security_group_ids` | Verified namespace/workgroup owner navigation and independent workgroup/endpoint VPC proof #695; provisioned Redshift #506 remains separate. |
| `aws_opensearchserverless_vpc_endpoint` | `vpc_id`, `subnet_ids`, `security_group_ids` | Verified complete VPC/subnet/SG ID navigation and common-VPC context #696; policy JSON is not collection ownership evidence. Historical #724 review remains separate. |
| `aws_docdb_cluster` | `db_subnet_group_name`, `vpc_security_group_ids` | Verified group/SG common-VPC placement and instance membership #697; other dimensions retain historical #717 review. |
| `aws_neptune_cluster` | `neptune_subnet_group_name`, `vpc_security_group_ids` | Verified common-VPC group/SG placement and instance membership #698; other dimensions retain historical #717 review. |
| `aws_dms_replication_instance` | `replication_subnet_group_id`, `vpc_security_group_ids` | Verified common-VPC group/SG placement and directional task/endpoint navigation #699; no database reconstruction from strings. Other dimensions retain historical #723 review. |
| `aws_batch_compute_environment` | `compute_resources.subnets`, `compute_resources.security_group_ids` | Verified compute subnet/SG common-VPC context and queue/environment/scheduling-policy references; shared configurations stay logical. See [Batch](aws-compute.md#batch-compute-environments-and-queues-700). |
| `aws_sagemaker_endpoint` | `endpoint_config_name` | Verified configuration/model-name and model role/subnet/SG references with common-VPC context #701; shared configuration/model ownership stays independent. Historical #725 review retained. |
| `aws_fsx_lustre_file_system` | `subnet_ids`, `security_group_ids`, `kms_key_id` | Verified complete subnet/SG common-VPC context and persistent-only KMS ARN references. See [FSx Lustre](aws-storage.md#fsx-lustre-networking-and-encryption-702); other variants remain deferred. |
| `aws_dynamodb_table` | `replica.region_name`, `server_side_encryption.kms_key_arn` | Spatial table, explicit Backup endpoint and [table/configuration navigation](dynamodb.md) exist. Import/export/restore references are tracked in #744; Lambda stream identity coordinates with #714. Inline replica expansion and at-rest summaries remain explicitly deferred in the #718 review. |
| `aws_elasticache_replication_group` | `subnet_group_name`, `global_replication_group_id`, `user_group_ids` | Common-VPC placement via subnet group exists (#360); global/user/parameter configuration references are implemented by #691. No at-rest summary. |

Implementation cross-checks: [classification](../src/provider/aws/classification.rs), [basic defaults](../src/provider/aws/basic_entries.rs), [placement](../src/provider/aws.rs), [containment](../src/provider/aws/containment/indirect.rs), [service references](../src/provider/aws/service_references.rs), [Backup endpoints](../src/provider/aws/backup.rs), [messaging](../src/provider/aws/messaging.rs) and [encryption allowlist](../src/provider/aws/encryption.rs). In particular, #601's summaries cover S3/SQS/RDS, not every service with encryption attributes. This distinction prevents borrowing adjacent-service support.

## Placement and deferred scope decisions

Retain existing independent MSK/MQ/DocumentDB/Neptune/DMS/FSx/SageMaker endpoint cards while adding evidence in their bounded issues. Retain Batch compute-environment cards; Network Firewall cards and verified VPC/policy/logging relationships are implemented by #693. App Runner connectors, database subnet groups, namespace/configuration objects and policies need separate ownership/network decisions; do not promote all objects from a service together. Existing embedded EFS access points, KMS aliases and Secrets Manager versions remain examples of proven presentation ownership, not a default for every reference.

MSK replicators (topics are tracked in #754), Network Firewall endpoint associations/transit-gateway/TLS variants (now tracked by #792), non-Lustre FSx, DocumentDB/Neptune global variants, DMS serverless remain outside the initial issue scopes; SageMaker Studio/training now have bounded follow-ups #877 and #878 from #725. DocumentDB Elastic and Neptune Analytics now have bounded follow-up #766 from the #717 review. Their exact types and schema candidates remain in the register. Runtime health, effective IAM/firewall policy, packet routing, credentials, item data and arbitrary attribute dumping are intentionally unsupported. Encryption summaries for additional services require a separate allowlist review; do not copy a boolean from a similarly named field.

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

## 過去の監査への参照

### Load balancing, edge and DNS review (#715)

[Load balancing, edge and DNS review (#715)](aws-semantic-history.md#load-balancing-edge-and-dns-review-715) を参照してください。

### Lambda, APIs and application integration review (#714)

[Lambda, APIs and application integration review (#714)](aws-semantic-history.md#lambda-apis-and-application-integration-review-714) を参照してください。

### EC2 VPC, subnet, endpoint and network-interface review (#710)

[EC2 VPC, subnet, endpoint and network-interface review (#710)](aws-semantic-history.md#ec2-vpc-subnet-endpoint-and-network-interface-review-710) を参照してください。

### Machine learning and generative AI review (#725)

[Machine learning and generative AI review (#725)](aws-semantic-history.md#machine-learning-and-generative-ai-review-725) を参照してください。

### Compute and scaling review (#712)

[Compute and scaling review (#712)](aws-semantic-history.md#compute-and-scaling-review-712) を参照してください。

### Network connectivity and traffic protection review (#716)

[Network connectivity and traffic protection review (#716)](aws-semantic-history.md#network-connectivity-and-traffic-protection-review-716) を参照してください。

### Search and business intelligence review (#724)

[Search and business intelligence review (#724)](aws-semantic-history.md#search-and-business-intelligence-review-724) を参照してください。

### Containers and application hosting review (#713)

[Containers and application hosting review (#713)](aws-semantic-history.md#containers-and-application-hosting-review-713) を参照してください。

### NoSQL, caches and time-series review (#718)

[NoSQL, caches and time-series review (#718)](aws-semantic-history.md#nosql-caches-and-time-series-review-718) を参照してください。

### Streaming, messaging and workflows review (#721)

[Streaming, messaging and workflows review (#721)](aws-semantic-history.md#streaming-messaging-and-workflows-review-721) を参照してください。

### Relational and graph database review (#717)

[Relational and graph database review (#717)](aws-semantic-history.md#relational-and-graph-database-review-717) を参照してください。

### Filesystems, transfer and backup review (#720)

[Filesystems, transfer and backup review (#720)](aws-semantic-history.md#filesystems-transfer-and-backup-review-720) を参照してください。

### Analytics and data governance review, batch 1 (#722)

[Analytics and data governance review, batch 1 (#722)](aws-semantic-history.md#analytics-and-data-governance-review-batch-1-722) を参照してください。

### Analytics and data governance review, batch 2 (#723)

[Analytics and data governance review, batch 2 (#723)](aws-semantic-history.md#analytics-and-data-governance-review-batch-2-723) を参照してください。
