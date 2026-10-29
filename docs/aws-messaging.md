# AWS APIs and messaging

[共通契約・サービス一覧](aws.md) に戻る。以下はサービス固有の条件です。各節に記載した Provider 版と証拠の範囲を維持し、他サービスの条件を転用しないでください。

- [API Gateway integrations and deployment context](#api-gateway-integrations-and-deployment-context)
- [Step Functions](#step-functions)
- [EventBridge buses and rules](#eventbridge-buses-and-rules)
- [SQS and SNS](#sqs-and-sns)
- [Amazon MQ network and configuration (#689)](#amazon-mq-network-and-configuration-689)
- [MSK network and configuration (#688)](#msk-network-and-configuration-688)
- [Scheduler and Pipes configuration (#692)](#scheduler-and-pipes-configuration-692)
- [SNS subscriptions](#sns-subscriptions)
- [Lambda event source mappings](#lambda-event-source-mappings)
- [Kinesis Data Streams](#kinesis-data-streams)
- [Amazon Data Firehose](#amazon-data-firehose)
- [EventBridge targets](#eventbridge-targets)

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

## EventBridge buses and rules

`aws_cloudwatch_event_bus` and `aws_cloudwatch_event_rule` are regional spatial
nodes with the Amazon EventBridge service label. They preserve Terraform identities,
managed/data mode, actions and lifecycle metadata. Explicit references remain
ordinary dependencies; event patterns, names and policy text do not establish
message flow or VPC/Subnet containment. Literal-only and implicit default buses
do not create resources or ownership. See the [EventBridge example](../examples/eventbridge.svg).

## SQS and SNS

`aws_sqs_queue` and `aws_sns_topic` are spatial nodes with AWS account and
regional scope from provider evidence. Managed/data mode, actions and ordinary
Terraform references remain visible. The [messaging example](../examples/messaging.svg)
includes both services and a data lookup in a singleton region. Names, ARNs,
policies and proximity do not establish message flow.

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
[#721 audit](aws-semantic-history.md#streaming-messaging-and-workflows-review-721).
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
