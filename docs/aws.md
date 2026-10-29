# AWS architecture relationships

## Batch compute environments and queues (#700)

Managed `aws_batch_compute_environment` retains an independent Spatial card.
`aws_batch_job_queue` and `aws_batch_scheduling_policy` remain independent Logical
resources. Job definitions remain basic Logical configuration; capacity does not
create jobs, instances or ECS clusters.

Contracts are pinned to AWS Provider 6.70.0 commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:

| Resource input | Required managed endpoint output | Navigation |
| --- | --- | --- |
| Environment `compute_resources.subnets` (set inside one list block) | `aws_subnet.id` | Configured subnets |
| Environment `compute_resources.security_group_ids` (set inside one list block) | `aws_security_group.id` | Configured security groups |
| Queue `compute_environment_order.compute_environment` (list of blocks) | `aws_batch_compute_environment.arn` | Compute environments |
| Queue `scheduling_policy_arn` | `aws_batch_scheduling_policy.arn` | Scheduling policy |

Each reference requires complete explicit unique Terraform provenance and matching
known values, correct output attributes, compatible provider binding/account/region,
and current managed endpoints. Literal equality, dynamic instance selection,
unknown values, duplicate endpoints and partial collections do not establish links.
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

State-qualified identities, actions, replacement/import/moved metadata and
current/deposed distinctions remain intact. Filtered endpoints cannot leave
dangling navigation or be repaired from another named state. No resource is
embedded or counted twice. Role/profile/AMI/EKS/launch-template references remain
follow-up #816; job-definition payloads, credentials, policy interpretation and
runtime scheduling/health are excluded.

Examples: [SVG](../examples/batch.svg), [HTML](../examples/batch.html).

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

## Network Firewall (#693)

`aws_networkfirewall_firewall` is an independent spatial card. A VPC firewall is
placed in its VPC only when `vpc_id` and every `subnet_mapping.subnet_id` have
explicit, unique managed references with matching known IDs, and each subnet's
verified VPC agrees. A multi-AZ firewall is not placed in one arbitrary subnet.
Unknown, partial, duplicate or conflicting evidence leaves the card outside the
VPC with a diagnostic. TGW/AZ-mapping variants do not use this containment rule.

Dedicated navigation uses these Provider 6.70.0 contracts, verified at commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:

| Consumer / input | Verified endpoint / output |
| --- | --- |
| Firewall `firewall_policy_arn` | `aws_networkfirewall_firewall_policy.arn` |
| Policy `firewall_policy.stateful_rule_group_reference.resource_arn` | Complete set of `aws_networkfirewall_rule_group.arn` with known `type = STATEFUL` |
| Policy `firewall_policy.stateless_rule_group_reference.resource_arn` | Complete set of rule-group ARNs with known `type = STATELESS` |
| Logging configuration `firewall_arn` | Firewall `arn`; proven configuration is shown in that firewall's Logging section |
| Logging destination `bucketName` with type `S3` | `aws_s3_bucket.bucket` |
| Logging destination `logGroup` with type `CloudWatchLogs` | `aws_cloudwatch_log_group.name` |
| Logging destination `deliveryStream` with type `KinesisDataFirehose` | `aws_kinesis_firehose_delivery_stream.name` |

Log destinations are under `logging_configuration.log_destination_config.log_destination`.
Their references must identify the map key, not merely the whole map: a whole-map
reference cannot distinguish a destination from a `prefix` expression and remains
generic. Explicit block counts must agree with known values. Set order is not
identity; the typed destination must match uniquely. Names or equal literal values
alone never create links. Unknown discriminator/key values and ambiguous matches
remain unverified. All three destination variants may coexist.

Policies and rule groups remain shared logical resources with independent identity;
neither ownership nor subnet placement propagates through policy references.
Logging ownership requires a unique compatible firewall and preserves its own
Terraform provenance/coverage. No logging configuration values, rule bodies,
policy actions, TLS material or computed endpoint IDs are displayed.

Only current managed AWS endpoints with explicit instances, compatible provider
bindings and known equal regions are accepted. Conflicting or unresolved provider
and account evidence is rejected. Verified account scope is retained for these
four types; JSON-only account evidence stays unknown. Named states remain isolated;
current configuration does not fabricate deleted/deposed links. Filtering retains
unavailable-endpoint explanations rather than dangling navigation.

Endpoint associations, transit-gateway variants/accepters, TLS inspection,
container monitoring and resource-policy/prefix-list references remain tracked in
#792 following the [#716 audit](aws-semantic-audit.md). KMS key/alias references
and summaries await their separate evidence/allowlist contract. No packet inspection,
effective rule enforcement, health or reachability is inferred.

The maintained [example](../examples/networkfirewall.svg),
[fixture](../tests/fixtures/networkfirewall-plan.json) and
[tests](../tests/unit/semantic/networkfirewall.rs) cover these boundaries.

Inventory-wide basic display does not imply the dedicated capabilities described here.
See the [semantic audit and exact-type gap register](aws-semantic-audit.md) for current
limits, intentional exclusions and bounded service follow-ups.

The [DynamoDB contract](dynamodb.md) covers explicit table/replica membership and
table configuration navigation, including conservative V1/V2 distinctions.

## API Gateway integrations and deployment context

REST and v2 reference rules are separate tables in `api_gateway/rest.rs` and
`api_gateway/v2.rs`. Existing API, Resource, Method, Deployment and service roles
remain unchanged. VPC Link references do not place an API inside a subnet.

| Family | Verified navigation | Required evidence |
| --- | --- | --- |
| REST | API, Resource and Method context | Exact IDs and method output; Resource and Method must belong to the same explicitly referenced API |
| REST | Stage deployment | Exact Deployment ID with the same API owner |
| REST | Lambda Integration / Authorizer | Exact Function or verified Alias `invoke_arn`; Integration requires AWS/AWS_PROXY, POST and matching Method context; Authorizer requires TOKEN/REQUEST |
| REST | Private Integration and NLB | HTTP/HTTP_PROXY with VPC_LINK and matching Method context; VPC Link has exactly one matching NLB ARN |
| v2 | API and explicit Stage deployment | Exact IDs and same API owner; deployment navigation requires known `auto_deploy = false` |
| v2 | Lambda Integration / Authorizer | Known HTTP/WebSocket API; AWS_PROXY and INTERNET for Integration, REQUEST for Authorizer; exact invoke ARN, or raw Lambda ARN for HTTP Integration |
| v2 | Private Integration | HTTP API, HTTP_PROXY, VPC_LINK and exact listener ARN; listener explicitly identifies an ALB/NLB |
| v2 | Link subnet / security groups | Entire known list/set matches unique explicitly referenced instances |

These are configuration reference links, not runtime reachability or permission
claims. No URI substring matching, external endpoint reconstruction, API body,
authorizer credentials or environment values are displayed. Missing, unknown,
partial, ambiguous, wrong-provider/type, data, deleted and deposed evidence retains
generic references and unresolved diagnostics. Provider bindings and known Regions
must match. Filtering and named states retain their existing identity boundaries.
An automatic v2 Stage does not acquire a synthetic Deployment.

The contracts were checked against AWS Provider 6.70.0 schema and a synthetic
Terraform 1.9.8 `plan -refresh=false`. See the official
[REST Integration](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/api_gateway_integration.html.markdown),
[REST VPC Link](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/api_gateway_vpc_link.html.markdown),
[v2 Integration](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/apigatewayv2_integration.html.markdown)
and [v2 Stage](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/apigatewayv2_stage.html.markdown) contracts.
Synthetic examples: REST [SVG](../examples/api-gateway-rest.svg) /
[HTML](../examples/api-gateway-rest.html), v2 [SVG](../examples/api-gateway-v2.svg) /
[HTML](../examples/api-gateway-v2.html). Generate HTML in Docker using
`cargo run --locked -- tests/fixtures/api-gateway-rest-plan.json --format html -o examples/api-gateway-rest.html`
(replace `rest` with `v2` for the second example).

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

The attribute types were checked against the actual AWS Provider 6.70.0 schema
(the [inventory hash](aws-inventory.md)) and a Terraform 1.9.8 `plan -refresh=false`
with synthetic inputs. The plan confirms both attribute and instance references;
Layer `version` is a string while permission `version_number` is numeric.
Official contracts: [Alias](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/lambda_alias.html.markdown),
[Layer](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/lambda_layer_version.html.markdown),
[Layer permission](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/lambda_layer_version_permission.html.markdown),
[Function URL](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/lambda_function_url.html.markdown).
The synthetic [SVG](../examples/lambda.svg) / [HTML](../examples/lambda.html) includes
two Layers, two Aliases and a literal-only URL fallback. Generate HTML in Docker with
`cargo run --locked -- tests/fixtures/lambda-plan.json --format html -o examples/lambda.html`.

AWS rules live in `src/provider/aws`. Dispatch uses provider source identity and
exact resource types; a custom provider's lookalike type does not acquire AWS
semantics. Provider source and configuration identity are described in [models](models.md).
The six [verified `aws_alb*` resource aliases](aws-inventory.md#verified-resource-aliases)
reuse their canonical `aws_lb*` rules, subject to the documented mode limits,
while preserving their original Terraform identities.
The generated [resource support matrix](resource-support.md) lists exact-type
roles, display modes, service classification and scope eligibility. This document defines the
conditional relationship and placement proofs behind that catalog.

The [AWS managed inventory](aws-inventory.md) supplies basic capabilities for all
registered Provider 6.70.0 managed types. These defaults add no dedicated
relationships or containment. Existing rules below retain their evidence contracts.

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

## CloudFront origins and policies

`aws_cloudfront_distribution` remains a Global spatial connector. Exact AWS
OAC, OAI, cache, origin-request and response-headers policy resources are Global
logical policy rows with their original identity, action and CloudFront service label.
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
service label. Known `scope = CLOUDFRONT` means Global even when the API provider uses
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
offline plan. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/private-ca-plan.json --format html -o examples/private-ca.html`.

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

## Step Functions

`aws_sfn_state_machine` and `aws_sfn_activity` are independent regional spatial
Nodes with the AWS Step Functions service label. A State Machine's `role_arn`
is an Execution role reference only with a complete unique explicit reference to
a current managed AWS IAM Role's `arn` and matching known evaluated values.
Unknown, ambiguous, wrong-provider/type and deleted/deposed endpoints stay
unverified; filtering marks absent endpoints unavailable. Ordinary Terraform
references and state-qualified identities remain intact.

ASL definitions are never parsed into task nodes, execution order or runtime
targets. Explicit Terraform references inside a definition retain their ordinary
dependency meaning. Definition text, payloads and attribute values are not
embedded. Activity resources do not imply workers or execution success.

The [synthetic fixture](../tests/fixtures/step-functions-plan.json) produces
[SVG](../examples/step-functions.svg) and [offline HTML](../examples/step-functions.html).
Regenerate HTML with `cargo run --locked -- tests/fixtures/step-functions-plan.json --format html -o examples/step-functions.html` in Docker.
The AWS Provider 5.100.0 [State Machine](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/sfn_state_machine)
and [Activity](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/sfn_activity)
schema defines regional resources and a scalar `role_arn`; ASL is an opaque string.

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
Regenerate HTML in Docker with `cargo run --locked -- tests/fixtures/cloudtrail-plan.json --format html -o examples/cloudtrail.html`.
The AWS Provider 5.100.0 schema and [official trail contract](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/cloudtrail)
define the home-region requirement and scalar destination references.

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
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/identity-providers-plan.json --format html -o examples/identity-providers.html`.

## Deployment scopes

EFS File Systems, Mount Targets and Access Points are regional spatial Nodes.
All three use the Amazon EFS service label. A File System is never
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
service-name rendering, lifecycle badges and state-local HTML export context are
preserved through the [common presentation contract](resource-presentation.md).
See the synthetic [SVG](../examples/efs.svg) / [HTML](../examples/efs.html).
The AWS Provider 5.100.0 [File System](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_file_system.html.markdown),
[Mount Target](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_mount_target.html.markdown)
and [Access Point](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/efs_access_point.html.markdown)
documentation, installed schema and Terraform 1.9.8 offline plan confirm these
fields and reference shapes. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/efs-plan.json --format html -o examples/efs.html`.

EBS Volumes are regional spatial Nodes with the Amazon EBS service label. A volume belongs
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
Systems Manager service label. Managed parameters and data lookups keep separate identities,
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
lookups, with the AWS Secrets Manager service label. `Version secret` requires
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
configuration with the AWS Certificate Manager service label. `Certificate reference`
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
Amazon Route 53 service label. The [synthetic example](../examples/route53.svg) also has
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
nodes with the Amazon EventBridge service label. They preserve Terraform identities,
managed/data mode, actions and lifecycle metadata. Explicit references remain
ordinary dependencies; event patterns, names and policy text do not establish
message flow or VPC/Subnet containment. Literal-only and implicit default buses
do not create resources or ownership. See the [EventBridge example](../examples/eventbridge.svg).

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

## Amazon MQ network and configuration (#689)

`aws_mq_broker` remains spatial; `aws_mq_configuration` remains an independent
logical resource, including when several brokers share it. Configured subnet and
security-group rows require complete, nonempty sets of explicit references to
current managed AWS `aws_subnet.id` and `aws_security_group.id` endpoints. Every
known value must match, without duplicate, missing or unknown members. Each member
must prove its own `vpc_id` reference to the same current managed AWS VPC. Only then
is the broker placed in that VPC, never an arbitrary subnet or availability zone.
Missing/default networking, incompatible VPCs or scopes leave placement unresolved
with a diagnostic. Removing proof endpoints through filtering removes placement.

The `MQ configuration` row verifies `configuration.id` against exactly one
`aws_mq_configuration.id`, plus an explicit sibling `configuration.revision`
reference to that same resource's `latest_revision`. Both revisions must be known,
equal positive integers. Both engine types must be known and agree on `ActiveMQ`
or `RabbitMQ`. The configuration block must be a single list element. Literal
IDs/revisions, older revisions, unknown values, ambiguous references, wrong output
fields, foreign providers, data resources and deleted/deposed endpoints cannot
establish this dedicated link. Generic Terraform dependencies retain their meaning.

All endpoints require compatible provider account/region evidence, using the same
provider configuration when neither account is known. Named states retain separate
identities; current configuration does not establish historical links. Actions,
import/move provenance and coverage remain attached to each original resource.
Users/passwords, LDAP settings, XML/Cuttlefish configuration bodies and broker
endpoints are not rendered. Replication, default networking, historical revisions,
runtime connectivity, access and health remain outside this implementation.

Contracts were checked against the pinned AWS Provider 6.70.0 schema and official
[broker documentation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/mq_broker.html.markdown)
and [configuration documentation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/mq_configuration.html.markdown):
`subnet_ids` and `security_groups` are string sets; `configuration` is a list with
string `id` and numeric `revision`; configuration `latest_revision` is computed.
The [fixture](../tests/fixtures/mq-plan.json), [SVG](../examples/mq.svg) and
[HTML](../examples/mq.html) show two brokers sharing one configuration.

## MSK network and configuration (#688)

Managed `aws_msk_cluster` and `aws_msk_serverless_cluster` retain independent
spatial cards. Their complete subnet and security-group references can establish
a common VPC, never an arbitrary subnet/AZ. Provisioned clusters use
`broker_node_group_info.client_subnets` / `security_groups`; Serverless uses
`vpc_config.subnet_ids` / `security_group_ids`. All are sets of IDs inside list
blocks. Initial Serverless placement requires exactly one VPC configuration;
multiple configurations remain unresolved rather than being flattened. Omitted
or unknown security groups do not invent the AWS default group.

`aws_msk_vpc_connection` is a spatial client-side connection. `vpc_id`,
`client_subnets` and `security_groups` must identify the same client VPC.
Its separately labeled `target_cluster_arn` points to a provisioned MSK cluster;
that cluster's network never supplies client placement. Initial links require
compatible account/provider evidence and equal known regions; cross-account
cluster references remain unproven until an explicit cross-account contract is
added. Network placement remains independent of target-cluster availability.

Every network member needs a unique current managed AWS endpoint, an explicit
`id` reference and equal known nonempty values. Complete sets reject duplicate,
partial and unknown members. Each subnet and SG must itself have a verified
`vpc_id` reference to the same current VPC. Missing or incompatible evidence
retains the independent card and reports an unproven placement diagnostic.
Filtering any required proof removes inferred containment.

Provisioned `configuration_info.arn` references `aws_msk_configuration.arn` only
when its numeric positive `revision` is also explicitly referenced from the same
configuration's known `latest_revision` and matches it. Older, unknown, literal
or mismatched revisions retain ordinary references. Shared configurations remain
independent logical resources; `server_properties` is never exposed.

`aws_msk_scram_secret_association` verifies `cluster_arn` and the complete
`secret_arn_list` set. `aws_msk_single_scram_secret_association` verifies
`cluster_arn` and scalar `secret_arn`. Cluster endpoints are provisioned
`aws_msk_cluster.arn`; secrets are `aws_secretsmanager_secret.arn`. Serverless
clusters do not inherit provisioned SCRAM semantics. Each association retains
its own lifecycle/provenance and coverage record; shared secrets are not embedded.
These configuration references add no authentication or connectivity claims.

Proofs require compatible bindings, explicit unique current managed endpoints,
valid known-value masks and exact output fields. Data, deleted, deposed, foreign
provider/type, missing, dynamic and filtered endpoints remain conservative.
Named states never supply one another's missing endpoints. Broker URLs, secrets,
topic contents, replication, CA/log/encryption extensions and runtime health are
outside this scope. No new arbitrary value summaries are emitted.

The pinned Provider 6.70.0 contracts and collection shapes are recorded in the
[#721 audit](aws-semantic-audit.md#streaming-messaging-and-workflows-review-721).
The [fixture](../tests/fixtures/msk-plan.json) and [SVG](../examples/msk.svg)
show separate client/cluster VPCs, a multi-subnet cluster, Serverless, shared
configuration and both SCRAM association forms. Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/msk-plan.json --format html -o examples/msk.html`.

## Scheduler and Pipes configuration (#692)

Managed `aws_scheduler_schedule` and `aws_scheduler_schedule_group` retain
independent logical identities. A verified `group_name` reference to a group's
`name` or `id` is labeled **Schedule group membership**; it does not embed the
schedule. Omitted/default group names create no group. `target.arn` and
`target.role_arn` are separately labeled target and execution-role references.

Managed `aws_pipes_pipe` is an independent spatial resource. Its `source`,
`enrichment`, `target` and `role_arn` references have distinct labels. These are
configuration links, not routed delivery edges or ownership of endpoints.

| Field | Initial managed endpoint allowlist / output |
| --- | --- |
| Schedule `target.arn`, pipe `target` | SQS queue, SNS topic, Lambda function, Kinesis stream, Step Functions state machine, EventBridge bus: `arn` |
| Pipe `source` | SQS queue or Kinesis stream: `arn`; competing or unknown source-family parameter blocks reject the proof |
| Pipe `enrichment` | Lambda function: `arn` |
| Schedule `target.role_arn`, pipe `role_arn` | IAM role: `arn` |

Every dedicated link requires an explicit unique current managed Terraform
reference, the allowed output field, equal known nonempty values, and compatible
provider/account evidence. Regional endpoints require equal known deployment
regions; an IAM role is global and does not borrow a region. Unknown accounts
require the same provider configuration. Missing, ambiguous, foreign-provider,
wrong-type, data, deleted and deposed endpoints remain unproven. Deleted schedules
and pipes do not borrow current configuration. Invalid nested unknown-mask shapes
also reject the proof. Named states and filtering preserve their existing identity,
availability, provenance and once-only coverage rules. Unsupported references stay
ordinary references alongside an unresolved dedicated field and diagnostic.

Universal-target operation ARNs, DynamoDB streams, MSK/MQ and self-managed Kafka
sources, API destinations, Step Functions enrichment and ECS/Batch targets are
outside the initial allowlist. Task/source network parameters do not contain the
schedule or pipe. Payloads, credentials, input templates, filters and arbitrary
parameters are not displayed or parsed. KMS/DLQ/logging references and schedule
summaries remain deferred. No runtime delivery or reachability is asserted.

Contracts were checked against AWS Provider 6.70.0 at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`:
[schedule](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/scheduler_schedule.html.markdown),
[group](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/scheduler_schedule_group.html.markdown)
and [pipe](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/pipes_pipe.html.markdown).
The [fixture](../tests/fixtures/eventbridge-configuration-plan.json) and
[SVG](../examples/eventbridge-configuration.svg) include verified links and a
literal universal-target fallback. Regenerate its HTML in Docker with
`cargo run --locked -- tests/fixtures/eventbridge-configuration-plan.json --format html -o examples/eventbridge-configuration.html`.

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
Amazon Kinesis Data Streams service label. Static provider regions are used
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
Nodes with the Amazon Data Firehose service label. They retain their own
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
the dedicated classification rules or the [basic capability table](aws-inventory.md).
Unknown types remain visible as `Unknown`. Roles are separate
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
records current behavior; the [presentation policy](resource-presentation.md)
defines ownership, selection and conditional child presentation.
The [purpose-dependent review](purpose-placement.md) retains Logical defaults for
SSM Parameters, metric alarms and the five IAM Identity types.
The [explicit generation-time option](configuration.md#explicit-spatial-types)
displays these types as independent Spatial cards while retaining reference details.
The matrix
lists the explicit logical types. They render as rows grouped by provider source,
resource type and mode in the top annotation area. AMI describes
an image definition used by workloads, not a deployed compute instance. These
existing provider-aware rules apply to managed and data resources; the basic
inventory extends managed resources only. Custom providers do
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
| EKS Node Group / Fargate Profile | `subnet_ids` | Verified current Subnets and common VPC; independent of cluster membership |
| RDS Proxy | `vpc_subnet_ids` | Verified current Subnets and common VPC; independent of DB targets |
| ElastiCache Serverless | `subnet_ids` | Verified current Subnets and common VPC; no provisioned nodes/members |
| OpenSearch | `vpc_options.subnet_ids` | Common VPC |

LB/ECS subnet references outside the chosen parent become Connection edges.
Subnet groups, RDS and the other workloads retain their original dependencies.
ElastiCache subnet and replication groups are supported visible resources with
AWS account/region scope and the ElastiCache service label. Replication placement requires
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

RDS Proxy `vpc_security_group_ids` and ElastiCache Serverless `security_group_ids`
use the strict current-resource and known-value checks described above before
adding SG connections.

| Consumer | Attribute |
| --- | --- |
| EC2, RDS Instance / Cluster / Proxy | `vpc_security_group_ids` |
| Load Balancer, EFS Mount Target | `security_groups` |
| ECS Service | `network_configuration.security_groups` |
| VPC Endpoint, ElastiCache Serverless | `security_group_ids` |
| Lambda / EKS | `vpc_config.security_group_ids` |
| OpenSearch | `vpc_options.security_group_ids` |

Each explicitly referenced AWS SG becomes a Connection to the managed consumer.
Multiple groups and exact indexed references are supported, including external
SG data sources. Attachment does not change containment or hide SG cards.
EFS Mount Targets, RDS Proxies and ElastiCache Serverless additionally require
current managed SG endpoints and reject deleted consumers/endpoints; other
consumers retain their existing data-SG rules.

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
Regenerate HTML in Docker with
`cargo run --locked -- tests/fixtures/autoscaling-network-plan.json --format html -o examples/autoscaling-network.html`.
