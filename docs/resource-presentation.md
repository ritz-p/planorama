# Resource presentation policy

This is the design contract for #540 and the service follow-ups, not a claim
that the proposed placements are implemented. The [generated support matrix](resource-support.md)
records current defaults. Role, placement, deployment scope, network containment,
semantic membership and presentation ownership are independent.

## Audit of the 42 AWS Logical types

The audit uses `provider/aws.rs` at `ac700bb`. Every type below then defaulted
to Logical for managed and data resources. Names in the table omit `aws_`.

| Types | Proposed treatment and reason |
| --- | --- |
| `acmpca_certificate_authority`, `kms_key`, `secretsmanager_secret`, `cloudwatch_log_group` | Independent cards: service authorities and storage endpoints have architectural identity outside VPCs. |
| `glue_job`, `glue_crawler`, `glue_catalog_database` | Independent cards: execution subjects and catalog storage. Database is not network containment. |
| `ecs_cluster` | Independent card: workload membership does not make the cluster a VPC/Subnet container or an automatic display owner. |
| `athena_data_catalog`, `athena_workgroup` | Independent cards: neither implies ownership of the other. |
| `glue_catalog_table` | Conditional child of a proven catalog database; shared or unresolved database identity requires an independent card. |
| `efs_access_point` | Conditional child of its verified file system, never of a mount target or subnet. |
| `kms_alias`, `secretsmanager_secret_version` | Conditional children of the verified key/secret; retain each object's own identity and scope. |
| `acm_certificate`, `acmpca_certificate` | Conditional owner/consumer presentation only with unique service-specific proof; shared certificates remain independent. Issuance and use are different relationships. |
| `acmpca_certificate_authority_certificate`, `acm_certificate_validation` | Keep Logical: installation/validation configuration, not another authority. |
| `s3_bucket_server_side_encryption_configuration` | Keep Logical: bucket configuration, not a separate storage endpoint. |
| `ecs_task_definition`, `ami`, `launch_template` | Keep Logical: definitions/templates rather than deployed execution subjects. |
| `iam_policy`, `iam_role_policy`, `iam_user_policy`, `iam_group_policy`, `autoscaling_policy` | Keep Logical: policy/configuration objects, irrespective of role names. |
| `iam_group_membership`, `iam_user_group_membership`, `iam_user_policy_attachment`, `iam_group_policy_attachment` | Keep existing provenance-backed relationship presentation; do not turn membership/attachment into containment. |
| `sns_topic_subscription`, `lambda_event_source_mapping`, `cloudwatch_event_target` | Keep existing association presentation and provenance; not independent execution resources. |
| `iam_role`, `iam_instance_profile`, `iam_group` | Keep Logical for this work: identity/access configuration with existing navigable references. |
| `iam_user`, `iam_openid_connect_provider`, `iam_saml_provider` | Conditional future review of identity semantics; retain current Logical placement and Policy role here. Neither the type name nor a display change justifies reclassifying these identities as policies or execution nodes. |
| `ssm_parameter` | Conditional future review as a stored object; retain Logical in this series and never expose parameter values. |
| `cloudwatch_metric_alarm` | Keep Logical: monitoring configuration with existing verified monitor relationships. |

These are 10 independent-card candidates, 6 conditional child candidates and
26 retained types (including 4 deferred semantic/display reviews). Route 53 zones
and records are already Spatial and are outside this migration.

## Provider and selection contract

Provider interpretation must supply a unique owner identity and value-free proof
of the service-specific relationship. Exact, complete, explicit references and
required value agreement are prerequisites, not interchangeable with Terraform
dependency edges. A configured reference, semantic membership, network containment
and display ownership must remain distinguishable in the model. Ownership alone
adds no dependency, traffic, authorization or containment edge.

An owner identity includes state context and current/deposed identity. A local
identity is resolved only inside its own state graph. Cross-state links remain
links unless a separate ownership rule proves them. Current configuration never
establishes historical ownership of a deposed object. Neither layout nor renderers
may infer owners from resource types, names, ARN substrings or proximity.

The provider declares a candidate; the view resolves effective presentation after
filtering and existing context closure. Exactly one visible, eligible owner is
required. Missing, external, ambiguous, shared, self or cyclic ownership falls back
to an independent card (or the explicitly documented existing safe placement).
Do not duplicate shared resources under consumers. Nested owners need explicit
measurement support; until then reject such chains rather than losing children.
Do not restore a filtered parent or child merely to complete a display group.
Existing network-ancestor context closure is separate from presentation ownership.
Routes also retain their existing explicit `verified Route Table owner` context
closure; RDS members and new generic children do not acquire that exception.

Parent and child retain their own account, region/global and unknown scope.
Embedding requires compatible proven scope; equal unknown labels are not proof.
Existing RDS rules additionally allow a shared, nonconflicting provider configuration
with a known region when account identity is unavailable. This is configuration
compatibility, not newly inferred account evidence. Scope disagreement or unproven
compatibility requires fallback. Independent cards use only their own proven
Region/AWS Global panels and otherwise remain in unknown scope.

## Shared implementation boundary

Existing Route and RDS instance presentations reuse `view/routes.rs` card
measurements, `view/spatial.rs` projection and `svg/routes.rs` output. Route
ownership retains its specialized next-hop evidence; RDS `member_owner` is
provider-verified display ownership. Missing selected RDS parents already release
members to ordinary cards. `PresentationOwner` records a state-local Terraform
target, verified attribute and provider-supplied labels. `view/presentation.rs`
resolves Independent, Logical or Embedded after selection, rejecting ambiguous
identity, self/cyclic/nested owners and incompatible member scopes. Route-specific
proof remains in `RoutePresentation`; both paths share effective owner lookup.
EFS Access Points now use this model with verified `file_system_id` service
references and independent Spatial fallback. KMS Keys are independent cards;
Aliases reuse the same presentation model with verified `target_key_id` references
and compatible scope. Keys never become children of encryption consumers.
The audit above remains a historical inventory of defaults at its recorded commit;
other service migrations are pending.

Measure headers, service icons, references, import/move badges and wrapped child
addresses before packing. Reserve parent height and actual child bounds/ports;
feed the same effective ownership into routing ancestry, masks, named-state offsets,
SVG metadata and HTML presentation/export indices. Avoid unrelated Route/RDS
geometry changes. A cluster/catalog is not a network Container merely because its
card contains children.

Each child keeps action, managed/data mode, current/deposed identity, DOM identity,
search/selection/history and focus/modules behavior. Count and coverage include
each selected Terraform resource once; grouping creates no resource. Parent-only
selection does not recreate excluded children; child-only selection retains a
visible fallback. Navigation and neighborhood/viewport exports must use actual
displayed ownership while preserving resource identities and relationship endpoints.

Moving a Logical row must preserve verified service-reference links on the new
card, including unavailable endpoints after selection. A link does not become a
drawn edge without a separately defined direction, meaning and provenance.
Never expose additional secret, policy, path or definition values to justify a
display change. Each service change needs focused identity/filter/scope regression
coverage and the rendering checks in [CONTRIBUTING](../CONTRIBUTING.md).
