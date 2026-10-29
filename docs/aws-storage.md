# AWS Storage and backup

[AWS 共通契約・サービス一覧](aws.md)。以下は各節に記載した Provider 版・証拠範囲の契約です。

## S3 bucket configuration

`aws_s3_bucket_policy` is a regional Logical Policy resource with the S3 service label,
independent identity, action and coverage. Its `Policy bucket reference` requires
a complete explicit unique current managed AWS Bucket reference to `id` or
`bucket` and matching known evaluated values. Bucket names alone, unknown/dynamic
expressions, data, deleted/deposed and wrong-provider/type endpoints do not prove
ownership. Missing/filtered endpoints remain visible as unresolved/unavailable;
state and module selection keep the original identities.

Policy JSON is not interpreted for permissions or data flow and is never emitted.
Ordinary Terraform reference/dependency navigation remains independent of the
dedicated bucket reference. No policy resource is folded into the bucket.
This is configuration evidence, not proof that a policy is effective.

AWS Provider 5.100.0/6.0.0 installed schemas and the
[policy contract](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/s3_bucket_policy.html.markdown)
define the initial scope. See the synthetic [fixture](../tests/fixtures/s3-policy-plan.json),
[SVG](../examples/s3-policy.svg) and [offline HTML](../examples/s3-policy.html).
Regenerate HTML with
`docker compose exec -T dev cargo run --locked -- tests/fixtures/s3-policy-plan.json --format html -o examples/s3-policy.html`.

Bucket notifications are regional Logical Controllers. The independent
`Notification bucket reference` uses the same Bucket identity proof.
Queue, topic and Lambda function references accept only exact managed AWS
SQS Queue, SNS Topic and Lambda Function `arn` outputs with matching known values.
They are configured references, not runtime event-flow or permission assertions.
`eventbridge` does not create a fabricated endpoint.

AWS Provider 5.100.0/6.0.0 notification blocks are lists. Each configured entry
retains its dotted ordinal path, such as `queue.0.queue_arn`, and list length.
Every enclosing evaluated list and unknown mask must have the expected shape.
Entries are verified independently, so repeated destinations retain distinct
configuration provenance. Reordering only evaluated values, changed list lengths,
unknown/dynamic/ambiguous entries and wrong-provider/type endpoints cannot
borrow another entry's proof. Aggregate ordinary Terraform dependencies remain
independent; no inline setting becomes a separate resource or coverage count.
Filter prefixes/suffixes, IDs, events and payloads are not emitted.

The official [notification contract](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/s3_bucket_notification.html.markdown),
synthetic [fixture](../tests/fixtures/s3-notification-plan.json),
[SVG](../examples/s3-notification.svg) and [offline HTML](../examples/s3-notification.html)
show two queue entries, SNS/Lambda references and a literal fallback. Regenerate HTML with
`docker compose exec -T dev cargo run --locked -- tests/fixtures/s3-notification-plan.json --format html -o examples/s3-notification.html`.

Replication configurations are separate regional Logical Controllers.
`Replication source bucket reference` proves the `bucket` reference to Bucket
`id`/`bucket`; `Replication destination reference` proves each
`rule.<index>.destination.<index>.bucket` reference to Bucket `arn`.
The same explicit/current/managed/type/value checks and nested list-length proofs
apply, including the schema's single destination per rule. Source and destination buckets keep their own scopes, including
cross-region destinations. Every rule keeps its positional provenance while the
configuration resource contributes one identity/action and coverage count.

References describe configuration whether a rule is enabled or disabled; they
do not assert replication activity, permission, versioning or transfer success.
IAM role dependencies remain ordinary Terraform references. Rule IDs, filter
values, payloads and policy bodies are not emitted. Missing/filtered, ambiguous,
dynamic, unknown, wrong-provider/type and deleted/deposed endpoints cannot be
resolved from names or matching ARN strings alone.

Verified against AWS Provider 5.100.0/6.0.0 list schemas, Terraform 1.9.8 plan
expressions and the official
[replication contract](https://github.com/hashicorp/terraform-provider-aws/blob/v6.0.0/website/docs/r/s3_bucket_replication_configuration.html.markdown).
The synthetic [fixture](../tests/fixtures/s3-replication-plan.json),
[SVG](../examples/s3-replication.svg) and [offline HTML](../examples/s3-replication.html)
show two rules, regional separation and a literal fallback. Regenerate HTML with
`docker compose exec -T dev cargo run --locked -- tests/fixtures/s3-replication-plan.json --format html -o examples/s3-replication.html`.

## AWS Backup

`aws_backup_vault` is a regional spatial Node; `aws_backup_plan` and
`aws_backup_selection` are regional spatial Controllers. All have independent
identity, coverage and AWS Backup service labels.

Plan `rule.target_vault_name` references require a unique explicit Vault
`name`/`id` reference and matching known value. Since rules are a set, only a
single rule is verified initially; multiple rules remain unavailable without
assuming positional correspondence. Selection `plan_id` requires a Plan `id`
reference. Both require compatible provider/region evidence. Selection
`iam_role_arn` and Vault `kms_key_arn` accept explicit Role/Key ARN references.
All dedicated references require current managed AWS endpoints and known values.

Selection `resources` is verified only when complete explicit ARN references
equal the entire known ARN collection. Initial endpoint types are EC2 Instance,
EBS Volume, EFS File System, RDS Instance/Cluster, DynamoDB Table and S3 Bucket.
Every endpoint must have compatible provider configuration and the same known
regional scope as the Selection; different or unknown regions remain unverified.
Wildcards, extra literal values, unknowns, wrong types/providers and historical
endpoints remain unavailable. Selections with tags, conditions or exclusions
also remain unverified; these are never expanded into a resource list. Ordinary
Terraform references remain available separately. Filtering and named states
preserve each resource's identity.

Configured references do not prove execution, recoverability or retention.
Rule contents, tag/condition values and recovery point contents are not emitted.
The [synthetic fixture](../tests/fixtures/backup-plan.json),
[SVG](../examples/backup.svg) and [offline HTML](../examples/backup.html) contrast
explicit and tag-based selections. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/backup-plan.json --format html -o examples/backup.html`.
See the AWS Provider 5.100.0 [Vault](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/backup_vault),
[Plan](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/backup_plan)
and [Selection](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/backup_selection)
schema and reference contracts.

## EFS

EFS File Systems, Mount Targets and Access Points are regional Spatial Nodes
with the Amazon EFS label. File Systems never enter Subnets, including One Zone;
no AZ layer is inferred. Mount Targets reuse Subnet containment and complete SG
collection proof. `File system` / `Mount target subnet` links require unique,
complete explicit managed AWS `id` references of the exact endpoint type with
matching known planned values. Unknowns keep ordinary references plus an
unproven-value diagnostic; deleted/deposed endpoints cannot prove current links.

An Access Point's verified `file_system_id` permits a measured child card under
that File System, never a Mount Target/Subnet. Independent scope must match by
known account/region, or known region plus the same nonconflicting provider config;
parent scope is not copied. Missing/external/ambiguous/filtered/incompatible owners,
and data/deleted/deposed Access Points, leave independent cards with their own scope.
The [presentation contract](resource-presentation.md) preserves each child's identity,
action, coverage, references, label, lifecycle badge and state-local HTML context;
multiple Access Points stay distinct and counted once. Focus/modules never restore
excluded parents or children. No EC2/ECS mount, NFS
reachability, file content or arbitrary POSIX/path settings are inferred/output.

See the synthetic [SVG](../examples/efs.svg) / [HTML](../examples/efs.html).
The AWS Provider 5.100.0 [File System](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_file_system.html.markdown),
[Mount Target](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_mount_target.html.markdown)
and [Access Point](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_access_point.html.markdown)
documentation, installed schema and Terraform 1.9.8 offline plan confirm these
fields and reference shapes. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/efs-plan.json --format html -o examples/efs.html`.

## EBS

EBS Volumes are regional Spatial Nodes with the Amazon EBS label. Their AWS AZ
does not create a view layer, infer region/co-location or place the volume in an
instance's Subnet. An `aws_volume_attachment` Association requires unique explicit
`instance_id` / `volume_id` references to current managed AWS Instance/Volume
`id` outputs with matching known planned values; only the proof is retained.
Deleted/deposed, unknown, dynamic, wrong-type/provider or ambiguous endpoints,
and extra dependencies/consumers, retain a navigable attachment card and diagnostics.
Verified edges preserve attachment identity/action/import/move/coverage once;
filtering removes unavailable edges or disables fallback-card links.
Inline `aws_instance.ebs_block_device` creates no resource and never merges with
volumes. Device paths, snapshot payloads and arbitrary block attributes are omitted;
attachments prove neither encryption, mount success nor runtime access.

See the synthetic [SVG](../examples/ebs.svg) / [HTML](../examples/ebs.html).
The AWS Provider 5.100.0 [Volume](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/ebs_volume.html.markdown)
and [Attachment](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/volume_attachment.html.markdown)
documentation, installed schema and Terraform 1.9.8 offline plan confirm these
fields and reference shapes. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/ebs-plan.json --format html -o examples/ebs.html`.

## FSx Lustre networking and encryption (#702)

`aws_fsx_lustre_file_system` retains its independent Spatial card. Dedicated
references require current managed AWS endpoints, unique explicit Terraform
provenance, compatible provider/account/region evidence and matching known values:

- `subnet_ids` references Subnet `id`. The pinned contract supports one subnet;
  the entire nonempty list must match, without literals or duplicate values.
- `security_group_ids` references Security Group `id`. The entire nonempty set
  must match; order is irrelevant and partial/unknown collections are unverified.
- `kms_key_id` references KMS Key `arn`, only with known `deployment_type` of
  `PERSISTENT_1` or `PERSISTENT_2`. Key IDs, aliases, absent/unknown deployment
  mode and implicit AWS-managed keys do not establish this reference.

When both network references are verified and every referenced subnet/SG has
matching explicit VPC ID proof for the same current managed VPC, the filesystem
is placed in that VPC. It is not embedded into a subnet, SG or key. Omitted/default
SGs, missing VPC proof or conflicting VPCs retain regional fallback; individually
proven references remain available. Computed filesystem `vpc_id`/ENI outputs do
not establish configured placement. Account scope is retained only from existing
verified provider evidence; JSON values do not manufacture account evidence.

Unknown, ambiguous, incompatible, deleted/deposed, data-source and filtered
endpoints retain conservative fallback and ordinary Terraform references.
Identity, actions, named-state boundaries and exactly-once coverage are preserved.
Import/export S3 paths, mount names/DNS, credentials, content and root-squash
settings are not emitted or interpreted. Windows/OpenZFS/ONTAP/cache contracts
remain #826; Lustre source-backup/logging references remain #825.

The pinned [Provider 6.70.0 contract](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/fsx_lustre_file_system.html.markdown)
and schema distinguish the subnet list, SG set and persistent-only KMS ARN.
[Tests](../tests/unit/semantic/fsx.rs) and the synthetic
[fixture](../tests/fixtures/fsx-lustre-plan.json) cover verified, literal and
unknown evidence, with [SVG](../examples/fsx-lustre.svg) and
[HTML](../examples/fsx-lustre.html) examples. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/fsx-lustre-plan.json --format html -o examples/fsx-lustre.html`.
Native/Wasm parity covers the same fixture and conservative fallbacks.
