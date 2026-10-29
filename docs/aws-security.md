# AWS Identity and security

[AWS 共通契約・サービス一覧](aws.md)。以下は各節に記載した Provider 版・証拠範囲の契約です。 [HTML 生成手順](aws.md#html-example-generation)。

## AWS Private CA

Exact AWS `aws_acmpca_certificate_authority` and `aws_acmpca_certificate` are
independent spatial Nodes; `aws_acmpca_certificate_authority_certificate` is a logical
Controller that preserves the installation resource's own identity. All three
use the AWS Private CA service label and their proven regional provider
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
offline plan.

## Cognito

`aws_cognito_user_pool`, `aws_cognito_user_pool_client` and
`aws_cognito_identity_pool` are independent regional spatial Nodes with the
Amazon Cognito service label. Their identity, lifecycle and coverage
remain independent of any verified reference.

Client `user_pool_id` references require an explicit unique User Pool `id`
reference and matching known values. Identity Pool
`cognito_identity_providers.client_id` and `.provider_name` references require
respectively a Client `id` or User Pool `endpoint` reference with matching known
values. The provider block is a set: only a single element is currently verified;
multiple elements, unknown values and ambiguous endpoints remain unavailable.
Managed, current, non-deleted AWS endpoints are required. Ordinary Terraform
references remain distinct from these configured associations. Literal ID/URL
matches alone never create associations; filtered endpoints become unavailable.
External IdP and role configuration retain ordinary Terraform references only.

No client secret, user schema, callback URL, credentials or provider attribute
values are included in the diagram. References do not prove authentication flow,
user counts or effective permissions. The [synthetic fixture](../tests/fixtures/cognito-plan.json)
produces [SVG](../examples/cognito.svg) and [offline HTML](../examples/cognito.html).
Terraform 1.9.8 / AWS Provider 5.100.0 schema and a refresh-disabled create plan
were checked against the provider's [User Pool](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/cognito_user_pool),
[Client](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/cognito_user_pool_client)
and [Identity Pool](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/cognito_identity_pool)
documentation. Create-time unknown IDs remain unverified.

## CloudTrail

`aws_cloudtrail` is one regional spatial Node with the AWS CloudTrail service
label. Provider region describes the trail's home region. Multi-region and
organization settings do not clone the trail into other regions/accounts or
make it Global; they do not prove collection coverage or successful delivery.

Dedicated configured references support `s3_bucket_name` to S3 `id`/`bucket`,
`cloud_watch_logs_group_arn` to Log Group `arn`, `cloud_watch_logs_role_arn` to IAM
Role `arn`, and `kms_key_id` to KMS Key `id`/`key_id`/`arn`. Each requires one
complete explicit reference to a current managed AWS endpoint and matching known
values. The Logs destination additionally accepts exactly the endpoint ARN plus
`:*`, as required by the provider contract. Other suffixes, unknown or ambiguous
values, foreign types/providers and historical endpoints remain unverified.
Literal strings alone never resolve endpoints. Filtering marks removed endpoints
unavailable and named states preserve independent identity.

Event selector text, audit events, log contents and raw attribute values are not
emitted. References do not claim logging permissions or organization membership.
Unsupported configurations retain ordinary Terraform references and fallback.
The [synthetic fixture](../tests/fixtures/cloudtrail-plan.json) produces
[SVG](../examples/cloudtrail.svg) and [offline HTML](../examples/cloudtrail.html).

The AWS Provider 5.100.0 schema and [official trail contract](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/cloudtrail)
define the home-region requirement and scalar destination references.

## IAM users and groups

Exact AWS `aws_iam_role`, `aws_iam_user` and `aws_iam_group` use logical Identity
rows. Roles and users represent principals; groups represent collections of
identities, not assumable principals. `aws_iam_policy` and inline role/user/group
policies remain Policy, as does the existing instance-profile auxiliary
configuration. Classification does not promote any of these resources to Spatial.
`aws_iam_group_membership`, `aws_iam_user_group_membership`,
`aws_iam_user_policy_attachment` and `aws_iam_group_policy_attachment` are logical
Associations. All use the IAM service label and Global scope, independently
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
Identity rows with the IAM service label and Global scope. These represent
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

## SSM Parameter Store

SSM Parameter Store `aws_ssm_parameter` is a regional logical Node with the AWS
Systems Manager service label. Managed parameters and data lookups keep separate identities,
even with equal names. Parameter paths create no inferred hierarchy, environment,
containment or deployment scope. Values (including ordinary String/StringList),
SecureString payloads, descriptions and arbitrary type strings are omitted.
ECS and other consumers retain explicit ordinary Terraform references; the
[ECS JSON-expression limitation](#secrets-manager) also applies to parameters. Neither
literal JSON/ARNs nor equal path names add links. Run Command, Session Manager and
Automation remain outside this scope. See [SVG](../examples/ssm.svg) /
[HTML](../examples/ssm.html) and the checked AWS Provider 5.100.0
[Parameter schema](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/ssm_parameter.html.markdown).
The installed schema and Terraform 1.9.8 fixture plan confirmed the value fields
and ECS reference shape.

## Secrets Manager

Secrets are independent regional cards with the AWS Secrets Manager label;
managed/data identities never merge by name. `Version secret` requires an explicit
managed/data Version `secret_id` reference to managed Secret `id`/`arn` and equal
known planned values. Name references stay ordinary because they resolve in the
consumer's account/region.

Managed/data Versions become measured children of one verified managed Secret
with compatible proven scope (known region plus shared nonconflicting provider
configuration can suffice without inferring account). Unknown/external/ambiguous/
filtered/data/deposed Secrets, and deleted/deposed Versions, leave independent
cards. Each card keeps its own proven region or unknown scope. The
[presentation contract](resource-presentation.md) preserves distinct identities,
actions, bounds, links and coverage; focus/modules do not restore excluded parents
or children. Consumers never own Secrets; child display adds no network containment
or consumer association. Payloads, secret strings/binary, rotation configuration
and container environment values are not retained/output.
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

## KMS

KMS Keys are independent regional cards with the AWS Key
Management Service service label. `Alias key` verifies `target_key_id` against an explicitly
referenced Key's `id`, `key_id` or `arn`. Encryption-key links use an exact allowlist:
SQS Queue `kms_master_key_id`, RDS Instance/Cluster `kms_key_id`, and S3 Bucket
Server-Side Encryption Configuration
`rule.apply_server_side_encryption_by_default.kms_master_key_id`. The S3
configuration is a regional Controller, summarized inside its uniquely verified
bucket when scope is compatible; otherwise it remains independently visible.
See [at-rest encryption](encryption.md) for plan-setting summaries and fallback rules.
Each key field requires matching known values and an explicit
managed `aws_kms_key` attribute reference. Same-named fields on other types do not
qualify. Literal aliases/ARNs, AWS managed keys and multi-region replicas are never
merged or guessed. Key policies, key material and encryption contexts are omitted;
configuration references establish neither runtime encryption nor permissions.
Aliases become measured children of the uniquely verified Key only with compatible
proven scope; known region and shared nonconflicting provider configuration may
suffice without account inference. Unknown/external/ambiguous/filtered/data/deposed
Keys, and data/deposed Aliases, leave independent cards. Under the
[presentation contract](resource-presentation.md), each Key/Alias keeps its identity,
action, coverage and own proven region (otherwise unknown); filtering restores
neither endpoint. Keys stay independent of encryption consumers. Child display
adds no network containment/communication edge and preserves service references.

See [SVG](../examples/kms.svg) / [HTML](../examples/kms.html). The allowlist follows
AWS Provider 5.100.0 [Key](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/kms_key.html.markdown),
[Alias](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/kms_alias.html.markdown)
and [S3 encryption](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/s3_bucket_server_side_encryption_configuration.html.markdown)
schemas, checked with Terraform 1.9.8.

## ACM

ACM certificates are regional cards, optionally grouped under a verified signing CA
under the [Private CA ownership contract](#aws-private-ca); certificate-validation resources remain separate Logical
configuration with the AWS Certificate Manager service label. `Certificate reference`
links on validation resources, LB listeners and CloudFront viewer certificates
require a complete explicit reference to `aws_acm_certificate.arn` / `.id` or,
for consumers, `aws_acm_certificate_validation.certificate_arn`, and matching known
planned values. They do not claim issuance, validation success or validity.
CloudFront keeps Global scope; each certificate keeps its own regional scope.
Certificate bodies, private keys, domains and validation DNS values are omitted.
The [ELBv2 TLS contract](aws.md#elbv2-certificates-and-trust-stores-908) adds IAM
server certificates, listener-certificate helpers and verify-mode trust stores;
it preserves the ACM validation reference path and requires compatible scope.
Route 53 is not required. The conservative [service-link proof rules](aws-network.md#route-53) also apply.
See [SVG](../examples/acm.svg) / [offline HTML](../examples/acm.html) and the
AWS Provider 5.100.0 [certificate](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acm_certificate.html.markdown)
and [validation](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/acm_certificate_validation.html.markdown)
contracts, checked against the installed schema and a Terraform 1.9.8 plan.

## Organizations hierarchy

`aws_organizations_organization`, `aws_organizations_organizational_unit` and
`aws_organizations_account` retain individual spatial cards. Organization and OU
also support their real data-source schemas; no Account data source is assumed.
These cards use a separate `AWS Organizations Global` scope without inheriting
the executing provider's management-account identity. An Organizations Account
represents a member account, not a deployment scope or a VPC/Subnet container.

Known `roots[index].id` values produce synthetic Root cards whose provenance is
the source Organization and list index. The source row links back to the
Organization; Root cards have no Terraform action and add no resources to counts.
Explicit `parent_id` references link OU/Account cards to their derived Root or OU.
Verification requires a unique exact AWS endpoint, a complete static traversal
and matching known planned values. The intermediate traversals emitted by
Terraform for `roots[0].id` are accepted. Literal-only IDs, missing parents,
dynamic selection, wrong providers/types, ambiguity, cycles and unknown values
remain unresolved. Current configuration never establishes deleted/deposed
hierarchy. Filtering can make a parent unavailable without restoring it.

Names, email addresses, account/root IDs, role values and other plan attributes
are not retained for this display. Cards use Terraform addresses; Root labels
add only the structural `roots[index]` path. Organization grouping is state-local;
it does not infer cross-state ownership or account deployment evidence.

The implementation was checked against the AWS Provider 5.100.0 schema and a
Terraform 1.9.8 offline plan. Upstream schemas:
[Organization](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/organizations/organization.go),
[OU](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/organizations/organizational_unit.go),
[Account](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/organizations/account.go).
The synthetic [input](../tests/fixtures/organizations-plan.json) and
[SVG](../examples/organizations.svg) / [HTML](../examples/organizations.html) show a Root, nested OUs and two member accounts.
Generate offline HTML with:

```sh
planorama tests/fixtures/organizations-plan.json --format html -o examples/organizations.html
```

## Organizations policies and direct attachments

`aws_organizations_policy` and `aws_organizations_policy_attachment` retain
individual logical rows, Terraform actions and resource counts. Policies display
an allowlisted policy type; `SERVICE_CONTROL_POLICY` is identified as SCP, while
TAG/BACKUP/other recognized types keep their own names. Unknown types remain
explicitly unsupported. Policy content, description, conditions and account emails
are never retained for rendering. Policy data sources use the real provider schema;
there is no policy-attachment data source. Deleted/deposed policies use their
own before-value for the type label; current configuration never creates past
attachments. Data policies may be referenced through `id` or `policy_id`; managed
policies use `id`. The referenced field must match the known attachment value.

Each attachment independently verifies `policy_id` and `target_id` against a
complete, explicit, unique, current AWS reference and known matching plan values.
Targets are derived Organization Roots, OUs or member Accounts. Its two labeled
reference rows describe the configured direct attachment and preserve partial
information when one endpoint is unresolved. Parallel attachments remain separate
resources. Filtering never restores a hidden policy, Organization or account.
These references do not calculate inherited/effective policy, IAM authorization
or runtime application success. Dependency arrows remain Terraform dependencies.
Policies/attachments keep the executing provider's verified management-account
scope, separately from the member-account identity at the target.

The [synthetic input](../tests/fixtures/organization-policies-plan.json),
[SVG](../examples/organization-policies.svg) and
[offline HTML](../examples/organization-policies.html) cover Root, OU, Account
and parallel attachments. Schema verification used AWS Provider 5.100.0:
[policy](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/organizations/policy.go)
and [attachment](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/service/organizations/policy_attachment.go).

```sh
planorama tests/fixtures/organization-policies-plan.json --format html -o examples/organization-policies.html
```

## Organizations member Account deployment navigation

A current managed Organizations Account can link to an existing verified AWS
Account deployment frame when its known, valid 12-digit account identity matches
the provider-evidence account. The provider's management account remains separate
from the member identity. JSON-only provider scopes stay unknown; member resources
never supply provider evidence. Names, emails, provider aliases and module/state
names are not matching evidence.

Matching runs after selection but checks member-identity ambiguity against the
complete input. Duplicate member representations remain explicitly ambiguous even
if filtering hides one. Missing, unknown, mismatched, deleted/deposed or excluded
endpoints do not produce links. One unique member can navigate to separately
state-qualified deployment frames in multiple input states. These are navigation
correspondences, not Terraform dependencies or cross-state ownership. Account
frames link back to the member; resources, actions and coverage counts are unchanged.
Current spatial managed resources and data sources can witness a frame. Candidates
use the same projected, scope-consistent containment groups as layout; mixed-scope
trees without an Account panel cannot produce deployment links. Ordinary Terraform
attributes named `deployment_scope` retain their own reference rows and metadata.

The [maintained SVG](../examples/organization-deployments.svg) and
[offline HTML](../examples/organization-deployments.html) use the synthetic
[evidence fixture](../tests/fixtures/organization-deployments-plan.json). The
snapshot test passes a synthetic saved-plan/configuration archive through the
existing evidence validator, then renders the normal selection pipeline. This
is a test-only example, not a CLI account-evidence bypass. Rendering that JSON
alone deliberately shows no verified deployment link. Regenerate both example
formats through the registered SVG generation script. Neither role ARNs nor
credentials are copied into the output.
