# AWS architecture relationships

AWS rules live in `src/provider/aws`. Dispatch uses provider source identity and
exact resource types; a custom provider's lookalike type does not acquire AWS
semantics. Provider source and configuration identity are described in [models](models.md).
The generated [resource support matrix](resource-support.md) lists exact-type
roles, display modes, icons and scope eligibility. This document defines the
conditional relationship and placement proofs behind that catalog.

## CloudFront origins and policies

`aws_cloudfront_distribution` remains a Global spatial connector. Exact AWS
OAC, OAI, cache, origin-request and response-headers policy resources are Global
logical policy rows with their original identity, action and CloudFront icon.
Inline origins and behaviors do not become Terraform resources.

Dedicated configured references require a complete, explicit, unique managed
endpoint, its allowed output attribute and equal known evaluated values:
S3 `bucket_regional_domain_name` / `bucket_domain_name` or application LB
`dns_name` for `origin.domain_name`; OAC `id`; OAI
`cloudfront_access_identity_path`; and each behavior policy's `id`.
The existing ACM certificate reference retains the certificate's regional scope.
These references describe configuration, not connectivity or effective access.

Only singleton nested blocks can currently prove these links. Multiple origins
or ordered behaviors, unknown or dynamic expressions, missing endpoints, data,
deleted/deposed resources and wrong-provider/type endpoints retain ordinary
references and safe unresolved diagnostics. Custom domains and literal IDs are
never matched by name or value alone. Selection marks filtered endpoints
unavailable, and states never resolve through another state's matching address.
Headers, authentication values, domains, URLs and policy bodies are not emitted.

Checked against AWS Provider 5.100.0 and 6.0.0 schemas and the official
[distribution contract](https://github.com/hashicorp/terraform-provider-aws/blob/v6.0.0/website/docs/r/cloudfront_distribution.html.markdown).
The synthetic [fixture](../tests/fixtures/cloudfront-plan.json),
[SVG](../examples/cloudfront.svg) and [offline HTML](../examples/cloudfront.html)
show S3/ALB origins, OAC/OAI, policies and an unresolved custom origin.
Regenerate HTML with
`docker compose exec -T dev cargo run --locked -- tests/fixtures/cloudfront-plan.json --format html -o examples/cloudfront.html`.

## WAFv2

Exact AWS Web ACLs and Rule Groups are spatial Policy cards with the AWS WAF
icon. Known `scope = CLOUDFRONT` means Global even when the API provider uses
`us-east-1`; `REGIONAL` uses verified resource/provider region evidence. Unknown
scope stays unknown with a diagnostic. Deleted/deposed scope can use its own
before-value, but current configuration cannot establish historical links.

`aws_wafv2_web_acl_association` retains its identity/action as a logical
association fallback. It becomes an undirected Association edge only when both
`web_acl_arn` and `resource_arn` prove unique current managed AWS endpoints with
matching known ARN outputs, compatible account/provider bindings and the same
known region. Protected endpoints are application LBs and explicit
`aws_api_gateway_stage.arn` (REST stages, not HTTP/WebSocket APIs or a REST API
root ARN). Extra relationships keep the association resource visible.
The edge preserves import/move provenance and coverage exactly once.

CloudFront `web_acl_id` is a separate configured reference to an explicit Web ACL
ARN with known CLOUDFRONT scope; it never creates an association resource.
Singleton `rule.statement.rule_group_reference_statement.arn` links require
known matching scope and compatible bindings. Multiple/ambiguous rules keep
ordinary references. Inline and AWS-managed rule groups never create resources.
Literal, unknown, dynamic, data, deleted/deposed and wrong-provider/type endpoints
remain unresolved. Filtering cannot invent missing endpoints; each state resolves
independently. Rule bodies, match criteria, IP sets and arbitrary values are not
emitted, and links do not assert successful protection or runtime inspection.

Verified against AWS Provider 5.100.0/6.0.0 schemas and the official
[Web ACL](https://github.com/hashicorp/terraform-provider-aws/blob/v6.0.0/website/docs/r/wafv2_web_acl.html.markdown)
and [association](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/wafv2_web_acl_association.html.markdown)
contracts. The synthetic [fixture](../tests/fixtures/waf-plan.json),
[SVG](../examples/waf.svg) and [offline HTML](../examples/waf.html) cover both scopes,
ALB/stage associations and unresolved configuration. Regenerate HTML with
`docker compose exec -T dev cargo run --locked -- tests/fixtures/waf-plan.json --format html -o examples/waf.html`.

## S3 bucket configuration

`aws_s3_bucket_policy` is a regional Logical Policy resource with the S3 icon,
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

## AWS Private CA

Exact AWS `aws_acmpca_certificate_authority` and `aws_acmpca_certificate` are
independent spatial Nodes; `aws_acmpca_certificate_authority_certificate` is a logical
Controller that preserves the installation resource's own identity. All three
use the AWS Private CA icon/service name and their proven regional provider
scope. Managed authorities and certificate installations retain the verified
provider account. Issued certificates and authority data sources keep account
scope unknown because they can use a CA shared by another account.
[AWS cross-account permissions](https://docs.aws.amazon.com/privateca/latest/userguide/pca-rbp.html)
include issuance and authority lookup, but not certificate import.

ACM certificates default to independent cards. A current managed certificate can
appear in its signing CA's measured `Certificates` section only with the verified
`Signing CA reference` below, compatible scope and no incoming attribute references
from other resources. Even one possible consumer keeps the certificate independent;
this includes validation intermediaries and unverified references. Shared certificates
are never duplicated under consumers. Private CA issued certificates always remain
independent because their account ownership is unknown, even with a matching provider
configuration. No certificate is embedded under a consumer or installation resource.
Missing, unknown or incompatible scope and missing/filtered issuers keep independent
cards using the certificate's own scope. Equal known region and a shared nonconflicting
provider configuration permit ACM grouping when both account scopes are unknown;
this does not establish account identity. Authorities never become network containers.
Child identity, lifecycle, references, coverage and selection/export use the shared
[presentation contract](resource-presentation.md).

`Signing CA reference` on Private CA and ACM certificates and `Target CA reference`
on installations require a complete, explicit, unique reference to a managed
CA's `arn`, with equal known planned values. `Installed certificate reference`
requires the same proof for an installation's `certificate` reference to a
managed Private CA certificate's `certificate` attribute. These are configuration
links alongside ordinary Terraform dependencies, not a derived CA hierarchy.
CSR and chain references retain ordinary Terraform provenance; CA type, names,
literal ARNs, certificate contents and co-occurrence never establish a parent CA.
Unresolved, external, ambiguous, unknown, wrong-provider/type, deleted/deposed
and filtered endpoints cannot become verified links. Unproven configured
references show unresolved with safe diagnostics, or Unavailable when filtered;
absent configuration does
not fabricate a missing parent resource. Data resources remain visible without
managed-only verification. No link claims issuance, installation success,
validity, revocation status or trust. CSR, certificate/chain contents, private
keys and subject values are omitted from generated output and retained models.

The [synthetic fixture](../tests/fixtures/private-ca-plan.json),
[SVG](../examples/private-ca.svg) and [offline HTML](../examples/private-ca.html)
show subordinate issuance, installation, ACM use and an external CA reference.
The smaller [certificate-card fixture](../tests/fixtures/certificate-cards-plan.json)
and [SVG](../examples/certificate-cards.svg) contrast an unused certificate grouped
under its issuer with one shared by an LB listener and CloudFront.
The AWS Provider 5.100.0
[authority](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acmpca_certificate_authority.html.markdown),
[certificate](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acmpca_certificate.html.markdown)
and [installation](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acmpca_certificate_authority_certificate.html.markdown)
contracts were checked against the installed schema and a Terraform 1.9.8
offline plan. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/private-ca-plan.json --format html -o examples/private-ca.html`.

## Athena Workgroup and Data Catalog

Exact AWS `aws_athena_workgroup` and `aws_athena_data_catalog` are independent
regional spatial Nodes with the Amazon Athena icon/service name. Neither is a
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
and `aws_glue_job` are regional spatial Nodes using the AWS Glue icon/service
name. Every resource retains its own identity, lifecycle and coverage entry.
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

## IAM users and groups

Exact AWS `aws_iam_role`, `aws_iam_user` and `aws_iam_group` use logical Identity
rows. Roles and users represent principals; groups represent collections of
identities, not assumable principals. `aws_iam_policy` and inline role/user/group
policies remain Policy, as does the existing instance-profile auxiliary
configuration. Classification does not promote any of these resources to Spatial.
`aws_iam_group_membership`, `aws_iam_user_group_membership`,
`aws_iam_user_policy_attachment` and `aws_iam_group_policy_attachment` are logical
Associations. All use the IAM icon/service name and Global scope, independently
of the provider region. Each stays a separate resource with its own action,
provenance, selection/search identity and coverage entry.

`User`, `Group`, `Member users`, `Member groups` and `Direct policy` reference rows
require complete, explicit references to current managed AWS resources of the
exact endpoint type. User/group references use `name`, and direct policy references
use `arn`, with equal known planned values. Name endpoints require equal
verified accounts, or the same non-conflicting provider configuration when both
accounts are unknown. Missing/conflicting bindings and cross-account name matches
remain unproven. Account compatibility is checked after optional saved-plan
evidence is applied, so different aliases can qualify when their proven accounts
agree. Membership collections must match the
entire uniquely named referenced endpoint set; literal names alone, mixed literal
members, unknown values and ambiguous instance selections cannot establish these
links. Deleted/deposed resources cannot establish current membership or attachment.
Ordinary Terraform references remain available when dedicated proof fails, with
an unproven-value diagnostic; filtered targets become Unavailable. Dependency
arrows remain Terraform dependencies, not effective-permission or trust edges.
Inline policy bodies, credentials, access keys, login profiles and passwords are
not displayed. Group-inherited permissions and policy contents are not evaluated.
The aggregate `aws_iam_policy_attachment` and exclusive management resources are
outside this initial scope.

The checked AWS Provider 5.100.0 schemas for
[group membership](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/iam/group_membership.go),
[user group membership](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/iam/user_group_membership.go),
[direct attachment](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/iam/user_policy_attachment.go)
and [inline policy](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/iam_user_policy.html.markdown)
define the name sets and scalar endpoints. Installed schema and an offline
Terraform 1.9.8 plan confirm the name-set and reference shapes. The synthetic
[fixture](../tests/fixtures/iam-plan.json), [SVG](../examples/iam.svg) and
[offline HTML](../examples/iam.html) include both membership directions, inline
policies, independent attachments and an unresolved literal-only membership.
Regenerate SVGs with the standard script and HTML in Docker with
`cargo run --locked -- tests/fixtures/iam-plan.json --format html -o examples/iam.html`.

## IAM identity providers

Exact AWS `aws_iam_openid_connect_provider` and `aws_iam_saml_provider` use logical
Identity rows with the IAM icon/service name and Global scope. These represent
federation configuration, not IAM users or principals. Provider region
does not make them regional. Managed/data instances, independent actions, moves,
imports, deposed resources and state-qualified identities use the shared resource
presentation and coverage rules.

Structured Terraform expression references remain ordinary `References` links and
dependency relationships, including references inside `assume_role_policy`
expressions. They describe configuration provenance, not established trust,
federation, effective permissions or successful execution. Unknown planned values
do not prevent a structurally explicit Terraform dependency. Dynamic/missing or
ambiguous references keep the shared unresolved diagnostics; filtered targets show
Unavailable. Wrong-provider or unrecognized types keep generic fallback and
ordinary Terraform references, without AWS classification. No dedicated historical
trust relationship is inferred for deleted/deposed resources.

Literal trust-policy JSON and matching issuer URLs/ARNs create no relationships.
An EKS issuer URL match does not prove provider identity. SAML metadata, thumbprints,
client ID lists, issuer values and trust-policy contents are not included in output.

The AWS Provider 5.100.0
[OIDC schema](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/iam/openid_connect_provider.go)
and [SAML documentation](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/iam_saml_provider.html.markdown),
installed schemas and Terraform 1.9.8 offline plan were checked for the initial
scope. The synthetic [fixture](../tests/fixtures/identity-providers-plan.json),
[SVG](../examples/identity-providers.svg) and [offline HTML](../examples/identity-providers.html)
contrast explicit OIDC/SAML references with literal policy and EKS issuer matches.
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/identity-providers-plan.json --format html -o examples/identity-providers.html`.

## Deployment scopes

EFS File Systems, Mount Targets and Access Points are regional spatial Nodes.
All three use the Amazon EFS icon. A File System is never
placed inside a Subnet, including One Zone file systems; no AZ layer is inferred.
Mount Targets reuse Subnet containment and SG connections. `File system` and
`Mount target subnet` require a complete, explicit, unique managed-resource `id`
reference with matching known planned values and the exact AWS endpoint type.
Unknown values retain ordinary Terraform references and an unproven-value diagnostic.
Deleted/deposed endpoints do not establish these current-configuration links.
SG lists use the existing complete collection proof. Access Points keep their own
identity, action and coverage. No EC2/ECS mount, NFS reachability, file content or
arbitrary POSIX/path setting is inferred or output.
Access Points with a verified `file_system_id` reference render as measured child
cards inside that File System. This is presentation ownership, not network
containment, and never places them under a Mount Target or Subnet. Scope must
match independently: known account and region, or a known region and the same
nonconflicting provider configuration. Parent scope is never copied to the child.
Missing, external, ambiguous, filtered or scope-incompatible owners leave an
independent Access Point card with its own scope. Data, deleted and deposed Access
Points also remain independent because current managed service-reference proof
does not apply. Child-only focus/modules selections do not restore the File System;
parent-only selection does not restore excluded children. Shared File Systems can
contain multiple distinct Access Points, each counted once. Verified references,
service-name/icon modes, lifecycle badges and state-local HTML export context are
preserved through the [common presentation contract](resource-presentation.md).
See the synthetic [SVG](../examples/efs.svg) / [HTML](../examples/efs.html).
The AWS Provider 5.100.0 [File System](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_file_system.html.markdown),
[Mount Target](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_mount_target.html.markdown)
and [Access Point](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_access_point.html.markdown)
documentation, installed schema and Terraform 1.9.8 offline plan confirm these
fields and reference shapes. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/efs-plan.json --format html -o examples/efs.html`.

EBS Volumes are regional spatial Nodes with the Amazon EBS icon. A volume belongs
to an Availability Zone in AWS; this view groups it by its proven region without
adding an AZ layer or placing it inside the attached instance's Subnet. An AZ
string does not infer a region, attachment, co-location or attachment success.
Independent `aws_volume_attachment` resources are Associations. Both `instance_id`
and `volume_id` must explicitly and uniquely reference a current managed AWS
Instance/Volume's `id`, with known matching planned values. Only the proof survives
ingestion. Deleted/deposed, unknown, dynamic, wrong-type/provider and ambiguous
endpoints retain a visible attachment card and safe diagnostics. Additional
dependencies/consumers also retain that card so its references remain navigable.
Verified edges keep the attachment's identity, action, import/move provenance and
coverage once, independently of endpoint identities. Filtering removes unavailable
edges or marks unavailable fallback-card links without inventing endpoints.
Inline `aws_instance.ebs_block_device` blocks do not create extra Terraform
resources and are never merged with standalone volumes. Device paths, snapshot
payloads and arbitrary block attributes are not output. The association makes no
claim about encryption, mount success or runtime access.
See the synthetic [SVG](../examples/ebs.svg) / [HTML](../examples/ebs.html).
The AWS Provider 5.100.0 [Volume](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/ebs_volume.html.markdown)
and [Attachment](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/volume_attachment.html.markdown)
documentation, installed schema and Terraform 1.9.8 offline plan confirm these
fields and reference shapes. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/ebs-plan.json --format html -o examples/ebs.html`.

SSM Parameter Store `aws_ssm_parameter` is a regional logical Node with the AWS
Systems Manager icon. Managed parameters and data lookups keep separate identities,
even with equal names. Parameter paths create no inferred hierarchy, environment,
containment or deployment scope. Values (including ordinary String/StringList),
SecureString payloads, descriptions and arbitrary type strings are omitted.
ECS and other consumers retain explicit ordinary Terraform references; the ECS
JSON-expression limitation described below also applies to parameters. Neither
literal JSON/ARNs nor equal path names add links. Run Command, Session Manager and
Automation remain outside this scope. See [SVG](../examples/ssm.svg) /
[HTML](../examples/ssm.html) and the checked AWS Provider 5.100.0
[Parameter schema](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/ssm_parameter.html.markdown).
The installed schema and Terraform 1.9.8 fixture plan confirmed the value fields
and ECS reference shape. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/ssm-plan.json --format html -o examples/ssm.html`.

Secrets Manager Secrets are independent regional cards, including separate data
lookups, with the AWS Secrets Manager icon. `Version secret` requires
an explicit `secret_id` reference from a managed or data Version to a managed
Secret's `id` or `arn`, matching known planned values. Name-based references remain
ordinary Terraform references because names resolve in the consumer's account and
region. Versions retain their own identities and actions.
Managed and data Versions become measured children only of one verified managed
Secret with compatible proven scope. Unknown, external, ambiguous, filtered,
data or deposed Secrets leave Versions as independent cards; deposed/deleted
Versions also remain independent. A shared nonconflicting provider configuration
and known region can prove compatibility without inferring an account.
Direct Version focus or `--modules` selection keeps the selected Version without
restoring its Secret; Secret-only selection does not restore excluded Versions.
Multiple Versions remain distinct children with their own bounds, links and
coverage counts. Shared consumers never own the Secret. Cards retain their own
proven region or unknown scope; display ownership adds no containment or consumer
association.
Equal names never merge managed/data resources. Payloads, secret strings/binary,
rotation configuration and container environment values are not retained or output.
See [SVG](../examples/secrets-manager.svg) / [HTML](../examples/secrets-manager.html).

Terraform 1.9.8 with AWS Provider 5.100.0 serializes ECS `container_definitions` as
a JSON string and flattens `jsonencode` references to the whole attribute. This
does not prove which reference supplies `secrets.valueFrom`. Such ECS references
remain navigable ordinary Terraform references; no dedicated secret-consumption
relationship is asserted. Literal JSON and ARN suffixes never establish endpoints.
No arbitrary JSON/definition parsing or expression evaluation is added. The
[Secret](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/secretsmanager_secret.html.markdown),
[Version](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/secretsmanager_secret_version.html.markdown)
and [ECS](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/ecs_task_definition.html.markdown)
contracts were checked against the installed schema and a real fixture plan.
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/secrets-manager-plan.json --format html -o examples/secrets-manager.html`.

KMS Keys are independent regional cards with the AWS Key
Management Service icon. `Alias key` verifies `target_key_id` against an explicitly
referenced Key's `id`, `key_id` or `arn`. Encryption-key links use an exact allowlist:
SQS Queue `kms_master_key_id`, RDS Instance/Cluster `kms_key_id`, and S3 Bucket
Server-Side Encryption Configuration
`rule.apply_server_side_encryption_by_default.kms_master_key_id`. The S3
configuration remains a regional logical Controller with a separately verified
`bucket` link. Each key field requires matching known values and an explicit
managed `aws_kms_key` attribute reference. Same-named fields on other types do not
qualify. Literal aliases/ARNs, AWS managed keys and multi-region replicas are never
merged or guessed. Key policies, key material and encryption contexts are omitted;
configuration references establish neither runtime encryption nor permissions.
Aliases use measured child cards only when this unique verified Key also has
compatible proven scope. A shared nonconflicting provider configuration with a
known region may establish compatibility when account identity is unavailable;
it does not infer an account. Unknown, external, ambiguous, filtered, data or
deposed Keys leave Aliases as independent cards. Data/deposed Aliases also remain
independent. Focus/modules selection never restores excluded parents or children.
Keys remain independent even with several encryption consumers; each Key and Alias
retains one identity, action and coverage count. Child presentation adds no network
containment or communication edge, and existing service-reference links remain
navigable. Cards use their own proven region, otherwise unknown scope.
See [SVG](../examples/kms.svg) / [HTML](../examples/kms.html). The allowlist follows
AWS Provider 5.100.0 [Key](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/kms_key.html.markdown),
[Alias](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/kms_alias.html.markdown)
and [S3 encryption](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/s3_bucket_server_side_encryption_configuration.html.markdown)
schemas, checked with Terraform 1.9.8. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/kms-plan.json --format html -o examples/kms.html`.

ACM certificates are regional cards, optionally grouped under a verified signing CA
as described above; certificate-validation resources remain separate Logical
configuration with the AWS Certificate Manager icon. `Certificate reference`
links on validation resources, LB listeners and CloudFront viewer certificates
require a complete explicit reference to `aws_acm_certificate.arn` / `.id` or,
for consumers, `aws_acm_certificate_validation.certificate_arn`, and matching known
planned values. They do not claim issuance, validation success or validity.
CloudFront keeps Global scope; each certificate keeps its own regional scope.
Certificate bodies, private keys, domains and validation DNS values are omitted.
Route 53 is not required. The same conservative service-link rules below apply.
See [SVG](../examples/acm.svg) / [offline HTML](../examples/acm.html) and the
AWS Provider 5.100.0 [certificate](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acm_certificate.html.markdown)
and [validation](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acm_certificate_validation.html.markdown)
contracts, checked against the installed schema and a Terraform 1.9.8 plan.
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/acm-plan.json --format html -o examples/acm.html`.

Route 53 Hosted Zones and Records are spatial nodes with Global scope and the
Amazon Route 53 icon. The [synthetic example](../examples/route53.svg) also has
[offline HTML](../examples/route53.html). `Hosted zone` links require a complete,
unique explicit `zone_id` reference to a managed Zone's `zone_id` or `id`, with
equal known planned values. `Alias target` requires both `alias.name` and
`alias.zone_id` to reference the same managed ALB/NLB (`dns_name`, `zone_id`) or
CloudFront distribution (`domain_name`, `hosted_zone_id`) with matching values.
The LB type must be known as `application` or `network`. A single private-zone
`vpc.vpc_id` reference can show `Associated VPC`; this is not network containment.

These service links supplement ordinary Terraform references. They preserve
source identity, state, actions, provenance, coverage and selection. Incomplete,
dynamic, ambiguous, literal-only, wrong-type/provider and unknown-value proofs
remain unresolved with `UnprovenEndpointValue`; filtered targets are unavailable.
Deleting/deposed objects do not borrow current configuration to prove historical
service links. Local/module aliases without a direct attribute traversal and
multi-VPC blocks remain ordinary references. DNS values are compared transiently
and never displayed. Records, DNSSEC, routing policies and DNS resolution are not
evaluated. Links do not guarantee reachability or successful DNS configuration.
The field contract follows the AWS Provider 5.100.0
[zone](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/route53_zone.html.markdown)
and [record](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/route53_record.html.markdown)
schemas and Terraform 1.9.8 nested expression paths.

Regenerate the Route 53 SVG with `sh scripts/update-examples.sh` and HTML with
`cargo run --locked -- tests/fixtures/route53-plan.json --format html -o examples/route53.html`
inside the development container.

`aws_cloudwatch_event_bus` and `aws_cloudwatch_event_rule` are regional spatial
nodes with the Amazon EventBridge icon. They preserve Terraform identities,
managed/data mode, actions and lifecycle metadata. Explicit references remain
ordinary dependencies; event patterns, names and policy text do not establish
message flow or VPC/Subnet containment. Literal-only and implicit default buses
do not create resources or ownership. See the [EventBridge example](../examples/eventbridge.svg).

`aws_rds_cluster_instance` is a supported regional resource with the RDS icon.
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
Static resource-level `region` overrides are supported for managed `aws_vpc`,
`aws_subnet` and `aws_cloudwatch_log_group`, verified with Terraform **1.9.8** or
**1.16.6** and HashiCorp AWS **6.0.0**. The configuration must pin
`version_constraint` to `6.0.0`
(optionally prefixed by `= `); a range, missing version or another version does
not prove this schema. AWS v5 fixtures do not support these overrides.
These are exact Core versions, not a minimum-version range. A JSON constraint is
not a selected-version/lock attestation: only the exact AWS 6.0.0 constraint
matches the schema contract checked with real locked plans. The
[compatibility matrix](terraform-compatibility.md) records both Core captures;
other Core versions and Provider releases remain unverified.
The reference-free literal must equal the unique current resource change's known,
non-sensitive `after.region`. Initial accepted literals are `us-east-1`, `us-east-2`,
`us-west-1`, `us-west-2`, `eu-west-1`, `eu-west-2`, `eu-west-3`, `eu-central-1`,
`ap-northeast-1`, `ap-northeast-2`, `ap-southeast-1` and `ap-southeast-2`.
See the [captured evidence](../tests/fixtures/aws-region-overrides/README.md) and
[maintained example](../examples/region-overrides.svg).

A verified override takes precedence over the provider region. An absent override
uses the existing provider fallback; an explicit dynamic, unknown, invalid,
conflicting or unsupported override stays unknown without fallback. Ordinary
attributes such as data `aws_region` or `aws_vpc_endpoint_service.region` do not
establish deployment scopes. Data overrides are not resolved in this initial scope.
Current create/update/no-op/replacement objects use after evidence; deleting
objects with this verified provider version stay region-unknown, including when
the current configuration has no override. Deposed objects never borrow current
configuration. Before values alone are not accepted as override proof.
The provider's original region, source, configuration identity and independent
account evidence remain intact; resolving a region does not establish an account.
Only the accepted scope value is retained, not arbitrary resource attributes.
Missing/dynamic bindings and unsupported types remain unknown. Global classification is
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
directed `Event source` relationship from a verified SQS Queue or Kinesis Data Stream to a verified Lambda
Function. Both `event_source_arn` and `function_name` require complete, unique,
explicit references. Known planned values must match the queue/stream ARN and the function
name or ARN, respectively. Values are compared transiently; only boolean evidence
is retained. Literal names/ARNs, policies, environment variables and proximity do
not identify endpoints. Unknown or mismatched values remain unresolved.
Unverified mapping endpoints report `unproven_endpoint_value` without exposing values.
Deletion actions use known `before` values (or prior state when a resource has no
change entry) for the mapping and both endpoints, still requiring explicit
configuration references, just as subscriptions do.

Each mapping retains its own action, import/move metadata, scope, source identity
and relationship number, including parallel mappings between identical endpoints.
Deposed mappings stay separate logical rows without borrowing current references.
Data queue/stream/function endpoints are supported; data mappings do not establish edges.
Unsupported sources, missing/filtered targets and incomplete or dynamic references
keep the logical row and independently verified partial context. A missing endpoint
never resolves against another state. Cross-account/region endpoints retain their
own scope; the mapping's provider binding does not assign scope to either endpoint.

The relationship describes configured event consumption, not active mapping state,
processing success, access permissions, reachability or delivery guarantees.
DynamoDB Streams, Kinesis consumers, MSK, Kafka, MQ and DocumentDB require separate explicit
type, endpoint and direction rules before they can produce event-source edges.
See [the event-source example](../examples/event-sources.svg), including duplicate
endpoints, a cross-region data queue, an unsupported source and a literal fallback.

## Kinesis Data Streams

Exact AWS `aws_kinesis_stream` resources are regional spatial nodes with the
Amazon Kinesis Data Streams icon/service name. Static provider regions are used
when no resource override is present. Stream region overrides are not yet in the
verified override allowlist and conservatively leave deployment region unknown.
The Lambda mapping rules above support stream `.arn` references, including
independent parallel mappings. Consumer resources and shard/runtime state are
outside this scope.

The synthetic [Kinesis fixture](../tests/fixtures/kinesis-plan.json) and
[SVG](../examples/kinesis.svg) / [offline HTML](../examples/kinesis.html) cover managed/data streams, parallel mappings,
unsupported sources and literal fallback. Generate the offline viewer with:

```sh
docker compose exec -T dev cargo run --locked -- tests/fixtures/kinesis-plan.json --format html -o examples/kinesis.html
docker compose exec -T dev node tests/browser/streaming.cjs
```

Evidence shapes were checked against the pinned AWS Provider 5.100.0
[stream schema](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/kinesis/stream.go),
the official [mapping documentation](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_event_source_mapping)
and Terraform's [JSON plan format](https://developer.hashicorp.com/terraform/internals/json-format).
Known synthetic values demonstrate verified links; a new stream with an unknown
ARN retains ordinary references without an event-source edge.

## Amazon Data Firehose

Exact AWS `aws_kinesis_firehose_delivery_stream` resources are regional spatial
Nodes with the Amazon Data Firehose icon/service name. They retain their own
action, identity, lifecycle metadata and coverage record. Static provider regions
apply when no resource override is present; explicit overrides remain unknown
until supported by the verified override allowlist.

Dedicated links reuse the service-reference rows and preserve ordinary Terraform
dependency arrows. They identify configured roles, not runtime delivery paths:

| Label | Delivery-stream attribute | Evidence |
| --- | --- | --- |
| Kinesis source | `kinesis_source_configuration.kinesis_stream_arn` | One Kinesis source block, no competing/unknown source block, explicit managed `aws_kinesis_stream.arn` reference and matching known value. |
| S3 delivery destination | `extended_s3_configuration.bucket_arn` | Known `destination = extended_s3`, one matching block, no competing/unknown destination block, explicit managed `aws_s3_bucket.arn` reference and matching known value. |
| S3 backup destination | `extended_s3_configuration.s3_backup_configuration.bucket_arn` | The same destination proof, known `s3_backup_mode = Enabled`, one backup block and a separately verified bucket ARN reference. |

Each reference must be complete, unique and instance-explicit. Source and
destination proofs are independent; an unsupported destination does not invalidate
an independently proven Kinesis source. DirectPut has no inferred source link.
Disabled/unknown backup modes do not acquire a backup role. Other destinations,
MSK sources and transformation processors retain ordinary references only.
No dedicated delivery/backup graph edges are added, so a Terraform dependency
arrow is never presented as an assertion of delivery or backup behavior.

Literal ARN equality, dynamic/ambiguous selections and missing values do not
establish dedicated links. Unverified allowlisted fields retain ordinary references
and `unproven_endpoint_value` diagnostics. Deleted/deposed streams or endpoints
never borrow current configuration to establish past dedicated links. Data-mode
streams/endpoints retain visible fallback resources. Filtered links show Unavailable;
missing endpoints never resolve against a different state. Endpoint URLs, credentials,
processor code, record payloads and raw attributes are neither retained nor output.
Links imply no runtime delivery, transformation, permission or reachability guarantee.

The implementation was checked against AWS Provider 5.100.0's installed schema,
official [delivery-stream schema](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/firehose/delivery_stream.go)
and [documentation](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/kinesis_firehose_delivery_stream.html.markdown).
The synthetic [fixture](../tests/fixtures/firehose-plan.json),
[SVG](../examples/firehose.svg) and [offline HTML](../examples/firehose.html)
show distinct delivery/backup buckets and literal fallback. Regenerate and verify:

```sh
docker compose exec -T dev sh scripts/update-examples.sh
docker compose exec -T dev cargo run --locked -- tests/fixtures/firehose-plan.json --format html -o examples/firehose.html
docker compose exec -T dev node tests/browser/streaming.cjs
```

## EventBridge targets

Managed `aws_cloudwatch_event_target` resources remain regional logical rows.
A complete, unique explicit `rule` reference must identify an AWS EventBridge
Rule whose known name matches the target's rule value. A complete, unique `arn`
reference must identify an AWS SQS Queue or Lambda Function with a matching known
ARN. These proofs produce a directed `Event target` relationship from Rule to
endpoint, carrying the target resource's identity, action and lifecycle metadata.

When a bus is configured, both target and rule must explicitly reference the same
recognized Bus and their known bus values must match its name or ARN. If neither
configures a bus, both plan values must independently identify `default`; this
does not identify or synthesize a Bus resource. Target, Rule and any explicitly
identified Bus require equal known regional scopes. Unknown accounts additionally
require the same non-conflicting provider configuration. Endpoint scope remains
independent; the relationship does not prove delivery, permissions or reachability.
Deleted targets compare before-state evidence for all referenced resources.

Missing, literal-only, ambiguous, dynamic, mismatched and foreign-provider
evidence leaves partial logical context without an invented relationship. Data
and deposed targets do not create edges. Unsupported endpoint types stay visible.
Filtering never borrows identity from another state; unavailable links become
plain context. Parallel targets retain separate provenance and numbers, while
each target is counted once across logical rows and the relationship index.
See the [EventBridge target example](../examples/eventbridge-targets.svg).

## Resource roles and data

VPC and Subnet are spatial `Container` resources. Other supported types are
classified as nodes, connectors, associations, identities, policies or controllers by
`classification.rs`. Unknown types remain visible as `Unknown`. Roles are separate
from managed/data mode; Security Groups remain Policy cards and target groups
remain Controller cards even when placed inside a VPC.

Identity describes principals, identity collections or federation configuration;
it proves neither authentication, role assumption nor effective permissions.
It does not establish equivalence between users or accounts: state, managed/data
and current/deposed identities remain separate even when display names match.
Policy describes access/control rules or policy-like configuration; it does not
necessarily mean IAM or policy enforcement. Controller describes resources that
manage or configure services, workloads or related resources; the role alone does
not establish runtime control, traffic or relationships. Logical type cards show
these explanations once per role, separately from Terraform action and managed/data
mode. Their action subsections change presentation only, not role or placement.

Placement is independent of role. The [generated matrix](resource-support.md)
records current behavior; the [42-type placement audit and migration policy](resource-presentation.md)
separately describe proposed service cards and conditional child presentations.
The [purpose-dependent review](purpose-placement.md) retains Logical defaults for
SSM Parameters, metric alarms and the five IAM Identity types.
The [explicit generation-time option](configuration.md#explicit-spatial-types)
displays these types as independent Spatial cards while retaining reference details.
The matrix
lists the explicit logical types. They render as rows grouped by provider source,
resource type and mode in the top annotation area. AMI describes
an image definition used by workloads, not a deployed compute instance. These
provider-aware rules apply to managed and data resources; custom providers do
not inherit them. Unknown types retain unresolved placement and stay visible in
the spatial diagram. Missing VPC/subnet references never imply logical placement.
Logical placement does not suppress architectural significance: individual resource
rows retain actions, identities, operation metadata, scope and focus/filter
participation. Focus selects resources without changing their placement;
independent Spatial cards require an explicit allowed `--spatial-type`.

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
ordinary `References` or `Dependency` rows, including Lambda logging configuration
and ECS container-definition references; they do not prove configured logging or
delivery success. Tags and `depends_on` receive no logging semantics or dedicated
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

Infrastructure data sources retain their roles and external identity. Spatial
cards use an external badge and dashed border; logical resources use separate
`data / external` groups and dashed action underlines. The matrix marks omitted
AWS metadata data sources; their incident edges are also omitted.
References from retained resources keep
the omitted source identities as provenance, without creating cards or paths.
Cached data resources in prior state are
included when still present in configuration. See [data.svg](../examples/data.svg).

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
| EFS Mount Target | `subnet_id` with known ID proof | Managed Subnet |

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
| Load Balancer, EFS Mount Target | `security_groups` |
| ECS Service | `network_configuration.security_groups` |
| VPC Endpoint | `security_group_ids` |
| Lambda / EKS | `vpc_config.security_group_ids` |
| OpenSearch | `vpc_options.security_group_ids` |

Each explicitly referenced AWS SG becomes a Connection to the managed consumer.
Multiple groups and exact indexed references are supported, including external
SG data sources. Attachment does not change containment or hide SG cards.
EFS Mount Targets additionally require current managed SG endpoints and reject
deleted consumers/endpoints; other consumers retain their existing data-SG rules.

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
| `aws_volume_attachment` | `instance_id`, `volume_id`, with known ID proof | EC2 Instance ↔ EBS Volume |
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
