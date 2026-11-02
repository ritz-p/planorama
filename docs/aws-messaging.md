# AWS APIs and messaging

## AppSync GraphQL APIs (#947)

GraphQL APIs have independent Spatial cards. Explicit current managed Lambda,
Cognito and IAM role references require known equal output values and compatible
provider/account/region evidence. Authentication and MERGED execution-role links
are gated by their exact discriminator. Additional Cognito authentication uses a
complete indexed list with element-correlated pool IDs and known region qualifiers.
Unknown/sensitive, mismatched, dynamic and historical endpoints remain unproven.

Source API associations require distinct GraphQL and MERGED endpoints. Only one
configured ARN/ID alternative per endpoint is accepted; unconfigured computed
counterparts may corroborate identity but cannot create a reference. ARN-based
cross-account associations require explicit provider accounts and a matching
endpoint ARN account/region. IDs alone cannot establish cross-account identity.
Shared APIs, roles and authentication backends retain independent scope; no schema,
token, policy, authentication result or runtime reachability interpretation occurs.

[AWS 共通契約・サービス一覧](aws.md)。以下は各節に記載した Provider 版・証拠範囲の契約です。 [HTML 生成手順](aws.md#html-example-generation)。

## API Gateway integrations and deployment context

REST and v2 reference rules are separate tables in `api_gateway/rest.rs` and
`api_gateway/v2.rs`. Existing API, Resource, Method, Deployment and service roles
remain unchanged. VPC Link references do not place an API inside a subnet.

| Family | Verified navigation | Required evidence |
| --- | --- | --- |
| REST | API, Resource and Method context | Exact IDs and method output; Resource and Method must belong to the same explicitly referenced API |
| REST | Stage deployment | Exact Deployment ID with the same API owner |
| REST | Stage certificate, documentation and canary | Exact certificate ID; documentation version and canary Deployment ID require the same explicitly referenced REST API |
| REST | Stage access logs | One known block with an exact Log Group or Firehose ARN; Firehose name starts with `amazon-apigateway-` |
| REST | Account logging role | Explicit matching IAM Role ARN and compatible regional provider binding; the account configuration does not own the role |
| REST | Lambda Integration / Authorizer | Exact Function or verified Alias `invoke_arn`; Integration requires AWS/AWS_PROXY, POST and matching Method context; Authorizer requires TOKEN/REQUEST |
| REST | Private Integration and NLB | HTTP/HTTP_PROXY with VPC_LINK and matching Method context; VPC Link has exactly one matching NLB ARN |
| v2 | API and explicit Stage deployment | Exact IDs and same API owner; deployment navigation requires known `auto_deploy = false` |
| v2 | Stage client certificate | Explicit matching API Gateway client certificate ID and known WebSocket API |
| v2 | Stage access logs | Known HTTP/WebSocket API and one complete block with an explicitly referenced matching CloudWatch Log Group ARN; independent of automatic deployment |
| v2 | Lambda Integration / Authorizer | Known HTTP/WebSocket API; AWS_PROXY and INTERNET for Integration, REQUEST for Authorizer; exact invoke ARN, or raw Lambda ARN for HTTP Integration |
| v2 | Private Integration | HTTP API, HTTP_PROXY, VPC_LINK and exact listener or Cloud Map service ARN; listener explicitly identifies an ALB/NLB |
| v2 | Quick-create Lambda | HTTP API with an explicit Function or verified Alias reference matching its raw ARN; generated integrations, routes and stages are not Terraform nodes |
| v2 | Integration role | Explicit IAM Role ARN; quick-create requires a verified Lambda target, integrations require HTTP AWS_PROXY or WebSocket AWS/AWS_PROXY |
| v2 | Link subnet / security groups | Entire known list/set matches unique explicitly referenced instances |
| v2 | Authorizer role | Known HTTP/WebSocket API and REQUEST authorizer with an explicit matching IAM Role ARN |
| v2 | Route integration / authorizer | Same explicitly referenced API; target exactly equals `integrations/` plus the referenced Integration ID; CUSTOM requires REQUEST, JWT requires HTTP and a JWT Authorizer |
| v2 | Route request models | WebSocket API; the entire known map matches unique explicitly referenced Model names with the same API parent |

These are configuration reference links, not runtime reachability or permission
claims. No URI substring matching, external endpoint reconstruction, API body,
authorizer credentials or environment values are displayed. Missing, unknown,
partial, ambiguous, wrong-provider/type, data, deleted and deposed evidence retains
generic references and unresolved diagnostics. Provider bindings and known Regions
must match. Filtering and named states retain their existing identity boundaries.
An automatic v2 Stage does not acquire a synthetic Deployment.
Stage logging and canary blocks require complete single-element lists. Stage,
account configuration, certificates and logging destinations retain their existing
independent placement. Log formats, certificates and stage variables are not displayed.

The contracts were checked against AWS Provider 6.70.0 schema and a synthetic
Terraform 1.9.8 `plan -refresh=false`. See the official
[REST Integration](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/api_gateway_integration.html.markdown),
[REST VPC Link](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/api_gateway_vpc_link.html.markdown),
[v2 Integration](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/apigatewayv2_integration.html.markdown)
and [v2 Stage](https://github.com/hashicorp/terraform-provider-aws/blob/v6.70.0/website/docs/r/apigatewayv2_stage.html.markdown) contracts.
The quick-create and Cloud Map extensions use the scalar fields in the pinned
[v2 API](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/apigatewayv2/api.go)
and [Integration schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/apigatewayv2/integration.go).
Service-action request maps remain opaque. A present or unknown integration subtype
does not establish a Lambda backend; role references are independently verified.
Route contracts also follow the pinned [route schema and tests](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/apigatewayv2/route_test.go)
and [authorizer documentation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/apigatewayv2_authorizer.html.markdown).
Routes, models and authorizers retain independent logical rows. JWT issuer URLs,
model schemas and request payloads are not interpreted or displayed.
Synthetic examples: REST [SVG](../examples/api-gateway-rest.svg) /
[HTML](../examples/api-gateway-rest.html), v2 [SVG](../examples/api-gateway-v2.svg) /
[HTML](../examples/api-gateway-v2.html). Generate HTML in Docker using
`cargo run --locked -- tests/fixtures/api-gateway-rest-plan.json --format html -o examples/api-gateway-rest.html`
(replace `rest` with `v2` for the second example).

REST helper references (#933) require explicit current managed endpoints and matching
known values. Documentation parts/versions, gateway responses, models, validators,
policies and API update helpers link to their REST API without interpreting payloads.
Method/integration responses verify the same API/resource/method tuple; integration
response status also identifies the matching method response. Response model maps
must be complete and API-qualified; built-in `Empty`/`Error` are not managed models.
Method settings link only to a same-API stage; wildcard paths do not infer methods.
Resource parents require an unambiguous acyclic chain ending at the same API's root.
These helpers retain independent logical cards, coverage and filtering behavior.
Contracts use AWS Provider 6.70.0 at commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`; field shapes and source paths are recorded
in `aws-semantic-decisions.json`.

REST authorization references (#931) require matching API/resource context. CUSTOM
methods accept TOKEN/REQUEST authorizers; COGNITO_USER_POOLS methods accept matching
Cognito authorizers. TOKEN/REQUEST authorizer roles use explicit IAM role ARNs, while
Cognito pool sets and request model maps must be complete. Validators and models
must belong to the same API. Built-in Empty/Error models remain unresolved.

REST custom domains (#932) retain independent logical cards. Mapping stages must
belong to the explicitly referenced API; private mappings also require a reference
to the same domain's matching domain_name_id. ACM certificates require EDGE
(us-east-1), REGIONAL or PRIVATE endpoint types as appropriate; ownership verification
is regional. Private access associations require a PRIVATE domain ARN and VPCE
Interface endpoint. Certificate registration names and truststore URLs stay opaque.

V2 domain mappings (#939) require a REGIONAL domain and an explicitly referenced
REST, HTTP or WebSocket API with a stage from that same API family and owner.
V2 domain certificate blocks must be single and REGIONAL. Routing-rule actions
require exactly one invoke_api block targeting an explicitly EDGE/REGIONAL REST API
and its stage; PRIVATE, unknown or missing endpoint types remain unresolved. The V2
resource name does not permit HTTP/WebSocket targets. Conditions remain opaque.

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

The AWS Provider 5.100.0 [State Machine](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/sfn_state_machine)
and [Activity](https://registry.terraform.io/providers/hashicorp/aws/5.100.0/docs/resources/sfn_activity)
schema defines regional resources and a scalar `role_arn`; ASL is an opaque string.

## EventBridge buses and rules

`aws_cloudwatch_event_bus` and `aws_cloudwatch_event_rule` are regional spatial
nodes with the Amazon EventBridge service label. They preserve Terraform identities,
managed/data mode, actions and lifecycle metadata. Verified configuration links
remain dependencies; event patterns, names and policy text do not establish
message flow or VPC/Subnet containment. Literal-only and implicit default buses
do not create resources or ownership. See the [EventBridge example](../examples/eventbridge.svg).

### Bounded configuration references (#748)

The following attributes require one complete, explicit managed AWS endpoint,
the listed output field and equal known values. Consumer and endpoint must be
current, non-deposed resources with compatible provider/account/region evidence;
IAM roles are global. Unknown, ambiguous, literal-only, foreign, historical and
missing endpoints remain unresolved. Selection and named states never borrow an
endpoint from another state.
Duplicate resource changes with the same address and deposed key are rejected
before parsing can overwrite evidence. Distinct current and deposed identities
can coexist; historical instances do not invalidate a valid current reference.

| Consumer attribute | Endpoint output |
| --- | --- |
| Rule `event_bus_name` | Event bus `name` or `arn` |
| Bus policy / permission `event_bus_name` | Event bus `name` |
| Archive `event_source_arn` | Event bus `arn` |
| API destination `connection_arn` | Event connection `arn` |
| Rule / target `role_arn` | IAM role `arn` |
| Target `dead_letter_config.arn` | SQS queue `arn` |

Archives and API destinations retain independent regional logical identities;
shared connections are not embedded. These links describe configuration, not
delivery, permissions or health. They do not interpret policy/authentication
bodies, HTTP URLs, event patterns or partner sources. Global endpoint bus pairs
and connection VPC Lattice references remain deferred. Existing target-to-rule
and SQS/Lambda relationships keep their separate proof contract.

Contracts were checked against AWS Provider 6.70.0 at
[`5e13eed`](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/events)
and its resource documentation. The [fixture](../tests/fixtures/eventbridge-references-plan.json)
covers each supported input/output pair; native and Wasm use the same verifier.

### Schema registry navigation (#755)

Schema and registry policy `registry_name` references must explicitly identify
one registry's `name`; discoverer `source_arn` must identify an event bus `arn`.
They use the same complete, unique, known-value and compatible provider/account/
region checks described above. Registry, schema, policy and discoverer retain
independent logical identities, including shared registries. Missing, filtered,
historical and ambiguous endpoints never borrow another state's identity.

Schema content, inferred event types, policy bodies and implicit AWS registries
are not interpreted or synthesized. The contract was checked against the pinned
[Schemas source](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/schemas)
and resource documentation. The [fixture](../tests/fixtures/eventbridge-schemas-plan.json)
is covered by semantic, safe SVG/HTML and native/Wasm parity tests.

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
[type-specific review data](aws-semantic-audit.md).
The [fixture](../tests/fixtures/msk-plan.json) and [SVG](../examples/msk.svg)
show separate client/cluster VPCs, a multi-subnet cluster, Serverless, shared
configuration and both SCRAM association forms.

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
literal universal-target fallback.

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
The additional managed-source and qualified-function contracts are described below.
Kinesis consumers and Kafka URLs do not identify managed event-source endpoints.
See [the event-source example](../examples/event-sources.svg), including duplicate
endpoints, a cross-region data queue, an unsupported source and a literal fallback.

### Additional managed sources and configuration (#950)

The extension requires current, non-deleted managed endpoints with complete unique
Terraform provenance, matching known values and compatible provider/account/region
evidence. It does not broaden the legacy SQS/Kinesis data/deletion contract above.
New source types use DynamoDB Table `stream_arn` (never table `arn`), MSK Cluster or
Serverless Cluster `arn`, MQ Broker `arn`, and DocumentDB Cluster `arn`.
Conflicting self-managed Kafka, managed Kafka and DocumentDB source settings cannot
prove these endpoints. Unknown, sensitive, literal-only or ambiguous values retain
the mapping's Logical row and unresolved context.

Function `qualified_arn` must equal the same Function's `arn` plus its known positive
numeric `version`. An Alias `arn` must agree with its name and the ARN of its
explicitly verified parent Function. Neither qualifier creates an extra Terraform
resource or merges the Alias into its Function. The mapping links to the Alias;
its event-source edge ends at the verified parent Function's card.
New references remain state-local;
filtered destinations become unavailable. Replacement and moved mapping identities
retain their actions and exactly-once coverage.

| Mapping input | Verified configuration reference |
| --- | --- |
| `destination_config.on_failure.destination_arn` | SQS/SNS ARN for proven DynamoDB/Kinesis/Kafka sources; S3 bucket ARN for Kafka only |
| `kms_key_arn` | KMS Key `arn`, describing configured filter encryption, not encryption effectiveness |
| `source_access_configuration.uri` | Secret or Secret Version `arn` for BASIC_AUTH, SASL_SCRAM_256_AUTH, SASL_SCRAM_512_AUTH, CLIENT_CERTIFICATE_TLS_AUTH and SERVER_ROOT_CA_CERTIFICATE |
| `source_access_configuration.uri` | Subnet `id` with the exact `subnet:` prefix for VPC_SUBNET; Security Group `id` with `security_group:` for VPC_SECURITY_GROUP |
| `amazon_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri` | Glue Registry `arn` with a proven MSK source |
| `self_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri` | Glue Registry `arn` with explicit self-managed Kafka configuration |

Access configuration is a set of typed URI entries: every known entry must match
exactly one compatible referenced instance and the correct output attribute, and
every referenced instance must be represented. Mixed secret/subnet/SG collections
are checked per instance. Unknown types, VIRTUAL_HOST, incomplete sets and duplicate
entries remain unresolved. These references do not place the Function or mapping
inside poller subnets or imply ownership, permissions or network reachability.
Confluent URLs, Kafka broker URLs, queue/topic names, schema bodies and credentials
remain opaque. No additional summaries are inferred.

Shapes and identities were checked against the pinned AWS Provider 6.70.0
[schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/lambda/event_source_mapping.go)
and [resource documentation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/lambda_event_source_mapping.html.markdown).
Nested configuration blocks are singleton lists; access configuration is a set.
The [SVG](../examples/lambda-events.svg) / [HTML](../examples/lambda-events.html)
exercise each new source and configuration family in native/Wasm parity checks.

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


## Usage plans and keys (#936)

Provider 6.70.0 commit 5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f defines
usage-plan api_stages as a set of required api_id/stage string pairs. Every row
must match explicit REST API id and stage stage_name outputs, and each stage
must independently prove that API via rest_api_id. Equal stage names in different
APIs remain distinct. Missing rows, partial references, unknown values and
conflicting qualifiers invalidate the pair collection. A shared usage plan stays
independent and is never embedded under an arbitrary API.

Usage-plan-key key_id and usage_plan_id reference API-key id and usage-plan id
only when key_type is known API_KEY. The association retains its own identity;
no key value is emitted. References require current managed explicit compatible
provider/account/region evidence. Literal-only, ambiguous, data, deleted/deposed
or filtered endpoints remain unresolved; named states remain isolated.
No permissions, throttling effects or runtime access is inferred. See the
[fixture](../tests/fixtures/api-usage-plan.json) and
[tests](../tests/unit/semantic/api_usage.rs).


## WebSocket response identities (#940)

AWS Provider 6.70.0 commit 5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f:
integration responses use api_id and integration_id, with a known nonempty
integration_response_key. Route responses use api_id and route_id, with the
supported $default route_response_key. Both require a known WEBSOCKET API and
an explicit parent whose independently proven API is that same endpoint.
Models retain independent API-qualified identity. response_models is a complete
string map of model names; the pinned route_response_test.go verifies model.name,
not model.id. Every model and the response's route must prove that same API.

All references require unique explicit current managed endpoints, exact output
fields, equal known nonempty values and compatible provider/account/region.
Unknown, conflicting, literal-only, partial, data, deleted/deposed and filtered
endpoints retain fallback. Named states stay isolated. Helpers remain independent;
no embedding or network containment is introduced. Model schema and response
body/template values are excluded. See the
[fixture](../tests/fixtures/api-responses-plan.json) and
[tests](../tests/unit/semantic/api_responses.rs).
