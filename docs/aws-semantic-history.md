# AWS semantic audit history

[監査方法・現行判断・更新手順](aws-semantic-audit.md) に戻る。以下は各節の checkout 時点の監査記録です。完了済み監査は実装完了を意味しません。後続実装後の現行状態は [判断データ](aws-semantic-decisions.json)、[生成台帳](aws-semantic-gaps.md)、[サービス契約](aws.md) を優先し、証拠・未対応範囲・再検討条件を照合してください。

## Shared audit method

All reviews use AWS Provider 6.70.0, commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f` (source archive `5e13eed`), with
schema SHA-256 and managed denominator checked against the pinned inventory.
Each type's [review record](aws-semantic-decisions.json) separates placement,
ownership, network, references, scope and safe summaries, recording inspected
fields, collection ancestry, input/output flags, managed/data differences,
official documentation and applicable implementation/test paths.
The [runtime matrix](resource-support.md) and [inventory tests](../tests/unit/aws_inventory.rs)
prove basic defaults, not dedicated semantics; shared service labels confer no
behavioral guarantees. Generic dependencies do not prove dedicated semantics.
Historical conclusions stay tied to each checkout; later implementations update
current status/gap text. Source/helper issues own their listed paths; endpoint
audits do not duplicate them or reopen completed #688–#702 scopes.

New relationships require unique explicit compatible references, matching known
outputs, complete element-correlated collections and independent provider,
account, region, named-state and lifecycle proof. Current configuration cannot
prove deleted/deposed links; unknown, sensitive, dynamic, mismatched or filtered
evidence remains conservative. Managed/data registrations are separate contracts.
Shared objects retain independent identity, action and exactly-once coverage;
reference evidence alone does not justify ownership or network containment.
Credentials, payloads, arbitrary maps/JSON, policy interpretation and runtime
reachability/health inference are excluded. Summary candidates require a separate
allowlist and known/unknown/sensitive tests, not merely an available schema field.
Service-specific exceptions and revisit conditions remain below and per type.

Common implementation checks: [aliases](../src/provider/aws/aliases.rs), [containment](../src/provider/aws/containment.rs), [spanning placement](../src/provider/aws/containment/spanning.rs), [SG proof](../src/provider/aws/security_groups.rs), [associations](../src/provider/aws/associations.rs), [references](../src/provider/aws/service_references.rs), [classification](../src/provider/aws/classification.rs), [basic defaults](../src/provider/aws/basic_entries.rs), [presentation](../src/provider/aws.rs), [scope](../src/provider/aws/scope.rs), [unknown scope reasons](../src/provider/aws/basic.rs), [indirect placement](../src/provider/aws/containment/indirect.rs), [membership](../src/provider/aws/members.rs), [summary allowlists](../src/provider/aws/encryption.rs), [VPC scope](../src/provider/aws/containment/vpc_scope.rs), [network collections](../src/provider/aws/network.rs), [embedded presentation](../src/provider/aws/service_members.rs).
Per-type records identify which paths were checked; service-specific evidence follows.

These documentation/data audits changed no runtime or rendering inputs, so SVG
regeneration and Layout quality were inapplicable. Docker verification used the
pinned-contract verifier, semantic audit/register tests and resource-support
consistency checks; #715 additionally verified alias schemas/factories and data
selectors, and #714 verified same-name data selectors. The #724 audit test covers
65 types; #720's verifier checks shapes and input/output flags. Commands and
acquisition instructions are maintained in the
[audit guide](aws-semantic-audit.md#maintenance-and-verification).

## Load balancing, edge and DNS review (#715)

Reviewed planorama commit: `f7f5d8bfb39ded416c786a21037935232b8f1eda`.
86 managed types: `elb`, `elbv2`, `cloudfront`, `cloudfrontkeyvaluestore`,
`globalaccelerator`, `route53`, `route53domains`, `route53profiles`, `route53resolver`.

Additional evidence: [components](../src/provider/aws/components.rs), [CloudFront](../src/provider/aws/cloudfront.rs), [WAF](../src/provider/aws/waf.rs).
The support matrix already agrees
with runtime defaults, so no source classification or matrix correction is needed.

Preserve #489 Route 53 zone and paired alias references, #510 singleton CloudFront
origin/OAC/OAI/policy references, ACM certificate and #496 WAFv2 proof,
#297 listener proximity, #299 target-group component membership and verified ALB
aliases. Component membership is not embedded presentation or network containment.
Legacy LB subnet/SG and instance-target association rules do not enforce the same
complete known-value proof as newer service references. Their extensions must not
be described as already supported. Completed #688–#702 service work is unaffected.

Contract distinctions checked:

- Six managed `aws_alb*` aliases share the canonical ELBv2 schemas and factories;
  only LB/listener/target-group have corresponding registered data aliases.
  `aws_lb_listener_rule` now has a data lookup, but `aws_alb_listener_rule` does not.
  Classic `aws_elb` and the `aws_lb_*_policy` legacy helpers are not ELBv2 aliases.
  Classic helper owners are ELB names, while ELBv2 owners use ARNs. Classic
  `policy_names` is an optional set despite required wording in both listener and
  backend-policy docs; `instance_ports` is a set of strings, not numbers.
- LB `subnets`, `security_groups` and Classic ELB `instances` are optional/computed
  sets. `subnet_mapping` is a set, not a positional list. EIP allocation IDs,
  IPAM pool IDs and log bucket names need separate typed proof. Target-group
  `load_balancer_arns` is computed, not configured ownership. Lambda targets omit
  VPC; IP targets do not identify Terraform workloads. Listener forward groups
  are nested sets; Cognito, ACM/IAM certificates and verify-mode trust stores have
  different contracts. Trust-store revocation IDs are computed numbers.
- Standard CloudFront `origin` is a set and `ordered_cache_behavior` a list.
  Existing singleton proof does not cover arbitrary collections. Multitenant
  distributions use `cache_behavior` and `lambda_function_arn`, unlike standard
  `ordered_cache_behavior` and `lambda_arn`. Their trusted-key groups are a nested
  block with `items`, not the standard list attribute. Tenant connection group is
  optional/computed; omitted service defaults and managed certificate requests
  do not create configured Terraform children. Tenant parameters remain opaque.
- Connection-function KVS association is nested at
  `connection_function_config.key_value_store_association.key_value_store_arn`;
  the documented top-level plural association does not exist in schema/source.
  Its ARN output is `connection_function_arn`. Ordinary functions instead use a
  set of store ARNs and have stage-qualified data lookups. Key/exclusive helpers
  reference the store ARN without exposing keys/values; exclusive destruction
  stops reconciliation rather than deleting the store's entries.
- Cache-policy `min_ttl` and continuous-deployment staging `items` are optional
  in schema/source despite required wording in docs. Staging names require
  explicit distribution output references, not DNS lookup. Encryption-profile
  `provider_id` is not Terraform provider identity. CloudFront trust-store S3
  bundle region belongs to the bucket, not the Global trust store. VPC-origin
  input endpoint ARN differs from output origin ARN/ID; configured regional
  ALB/NLB/EC2 endpoints do not put the distribution inside their VPC. The
  [AWS VPC-origin guide](https://docs.aws.amazon.com/us_en/AmazonCloudFront/latest/DeveloperGuide/private-content-vpc-origins.html)
  clarifies the endpoint variants beyond the pinned resource's generic ARN wording.
- Global Accelerator standard endpoint sets use LB ARNs or EIP allocation IDs;
  custom routing uses subnet IDs. EC2 instance-ID extension requires its explicit
  [AWS endpoint contract](https://docs.aws.amazon.com/global-accelerator/latest/api/API_EndpointIdentifier.html),
  which the pinned Terraform endpoint documentation omits. Endpoint-group region
  is optional/computed and describes targets, not Global control deployment.
  Cross-account principals are account IDs; CIDR selectors and service-created SGs
  do not establish managed resources or effective authorization.
- Route 53 private-zone VPC associations, including authorization prerequisites,
  never contain the Global zone. Record alias proof needs both DNS and zone
  outputs from the same endpoint. Exclusive record sets use
  `alias_target.dns_name` / `hosted_zone_id`, not `alias.name` / `zone_id`, and do
  not create managed records or own default NS/SOA. CIDR-location IDs are
  deprecated composites; traffic-policy instances require ID and version.
  Health-check `request_interval` is optional in schema/source despite required
  wording in docs. Check regions and alarm region are target facts, not placement.
- KSK source builds its composite ID from hosted-zone ID and **KSK name**, despite
  docs describing the second part as KMS identifier. KMS signing keys and public
  query-log groups retain their us-east-1 contracts. DNSSEC configuration is not
  proof of validation. Domain creation exports a computed hosted zone; existing
  registration management is a distinct resource. Their `name_server` schemas
  differ: a list-of-object attribute versus a list block. Contacts, WHOIS and
  public-key material are excluded.
- Resolver endpoints configure subnet sets and SG sets; `host_vpc_id` is computed.
  Rule endpoint proof must be type/direction gated; target IPs are not resource
  identities. Resolver `resource_id` controls refer to VPCs. Firewall fail-open is
  optional/computed despite required wording in docs; DNSSEC validation status
  is computed. Advanced firewall rules omit domain-list IDs and have computed
  threat-protection IDs. Shared rules/groups retain independent producer scope.
- Profile resource associations configure `resource_arn` despite docs calling it
  an ID; `resource_type` is computed and `resource_properties` is opaque JSON.
  The bounded initial endpoints are private zones, Resolver rules and firewall
  groups; additional endpoint/logging variants require explicit contract tests.
  Profile association does not expand effective DNS settings or transfer ownership.

Schema/docs discrepancies were checked in pinned
`internal/service/{elb,cloudfront,route53,route53resolver,route53profiles}` source;
per-field `sourceContracts` retain exact files/tokens. Data selectors/flags do not
extend the managed denominator. Safe-summary exclusions and revisit conditions
remain per type.

Actionable follow-ups: Classic ELB #906; LB network/logging #907; TLS/trust stores
#908; Cognito actions #909; target attachments #910; standard CloudFront #911;
multitenancy #912; functions/KVS #913; signing helpers #914; monitoring/logging
#915; VPC origins #916; standard Accelerator #917; custom/cross-account Accelerator
#918; zone associations #919; records/routing helpers #920; health checks #921;
DNS query logging #922; DNSSEC #923; Resolver endpoint/rules #924; VPC DNS controls
#925; DNS Firewall #926; Profiles #927. Each owns its listed source/helper paths;
consumer-side and endpoint-side work are separated in the issue boundaries.
Independent shared identities with no outgoing architecture contract remain logical
where appropriate. Classic ELB, accelerators, multitenant distributions and
Resolver endpoints have explicit architecture-card gaps, not blanket promotion.

## Lambda, APIs and application integration review (#714)

Reviewed planorama commit: `80b70c8fec8b5a3c75b0457bf0f4061fccea441e`.
71 managed types: `lambda`, `lambdacore`, `lambdamicrovms`, `apigateway`,
`apigatewayv2`, `appsync`, `serverlessrepo`. Stack base [#929](https://github.com/ritz-p/planorama/pull/929),
main `f7f5d8bfb39ded416c786a21037935232b8f1eda`.

Runtime evidence includes classification/basic defaults, scope, reference and
membership rules, [Lambda proofs](../src/provider/aws/lambda.rs),
[API Gateway proofs](../src/provider/aws/api_gateway.rs),
[messaging](../src/provider/aws/messaging.rs),
[subnet-derived placement](../src/provider/aws/containment/vpc_scope.rs) and
[SG relationships](../src/provider/aws/security_groups.rs). Per-type records link
their behavioral tests. Inventory and support-matrix tests establish basic
presentation only; AppSync has no dedicated backend/owner rules at this baseline.

Preserved completed scopes: #513 function/alias/version, layer and URL navigation;
#509 REST/v2 API context, Lambda/private integration, stage and VPC-link proofs;
#394/#502 SQS/Kinesis event-source lowering; #181/#182 Lambda network context.
At this audit baseline, the older Lambda subnet/SG paths did not enforce every
modern known-value, provider and lifecycle gate. [#949](aws-compute.md#lambda-configuration-and-network-proof-949)
subsequently strengthens these proofs and adds bounded configuration references.
#688–#702 remain separate.

Contract distinctions checked:

- Lambda function/layer `version` outputs are strings, while layer permission
  `version_number` is numeric. A layer `arn` includes its version; `layer_arn`
  does not. Alias names, published versions, `$LATEST` and `$LATEST.PUBLISHED`
  belong to different qualifier contracts and never create extra Terraform nodes.
- Event-source `function_arn` is computed and differs from configured
  `function_name`. DynamoDB needs `stream_arn`, not table ARN. Kafka/MQ/DocumentDB
  require typed source contracts; poller access and failure destinations do not
  imply function network containment. Kafka URLs, topic/queue names and schema
  payloads remain opaque.
  [#950](aws-messaging.md#additional-managed-sources-and-configuration-950) subsequently
  implements these bounded managed-source and configuration references while
  preserving the legacy SQS/Kinesis contract.
- REST integration `integration_target` supports ALB/NLB with VPC Link V2;
  its `uri` then carries Host context rather than backend identity. Legacy REST
  VPC links use one NLB ARN. V2 private integration uses listener or Cloud Map
  service ARN. Neither places the API inside endpoint subnets.
- REST model, resource, method, response, stage and documentation identities are
  API-qualified. Built-in models, wildcard method paths, OpenAPI bodies and
  deployment triggers cannot manufacture endpoints. Private domain access uses
  `VPCE` plus endpoint ID; its deprecated `id` is distinct from `arn`.
- Despite its package name, `aws_apigatewayv2_routing_rule.action.invoke_api`
  targets a **REST** API and stage. V2 route targets require explicit
  `integrations/ID` provenance. Auto-deployed stages differ from explicit
  deployment selection; HTTP/WebSocket discriminators remain necessary.
- AppSync Event API `api_id`/`api_arn` differs from GraphQL `id`/`arn`. Event log
  configuration is nested under `event_config`. GraphQL `GLOBAL` visibility is
  not global deployment scope. Current basic API cards are Logical; architecture
  presentation belongs to the linked implementation work, not this audit.
- AppSync data-source names and pipeline `function_id` values require API
  agreement; function names need not be unique. Backend and Cognito regions are
  endpoint selectors, not the API's deployment region. Source association
  configuration is a list-of-object attribute in the schema, not a nested block.
  Custom-domain certificates require us-east-1 evidence; generated CloudFront
  names do not prove managed distribution ownership.
- Lambda Core egress subnet/SG sets describe connector ENIs. MicroVM ingress
  connectors and image egress settings are distinct; `INTERNET_EGRESS` and
  AWS-managed base-image ARNs are not managed resources. MicroVM workload
  presentation needs review; image code, environment maps and hook payloads do not.
- Serverless Repository application ARNs have no managed application endpoint in
  this denominator. Stack outputs and parameters cannot discover deployed children;
  revisit only with an explicit supported application/stack provenance contract.

Bounded follow-ups: REST settings/auth/domain/helpers/integrations/private API/
usage plans #930–#936; V2 integrations/routes/domains/helpers/stages #937–#941;
AppSync Event/helpers/backends/domains/resolvers/GraphQL #942–#947; Lambda capacity,
configuration, event sources, async destinations, qualified helpers and policies
#948–#953; Core networking and MicroVMs #954–#955. Every actionable type links its
own issue. Shared resources remain independent; payloads, credentials, arbitrary
summaries, policy interpretation and runtime reachability remain excluded.

## EC2 VPC, subnet, endpoint and network-interface review (#710)

Reviewed planorama commit: `a874b24ecfe4f0361ac81dc5afcb87e6570cd1d3`.
83 exact managed types assigned to #710.

Additional evidence: [routes](../src/provider/aws/routes.rs).
The [support matrix](resource-support.md) already agrees with runtime defaults.
An incoming route target alone does not prove originating network context.
#895 now adds verified gateway VPC, NAT allocation and EIP/helper contracts;
see [the implemented scope](aws-network.md#vpc-gateways-and-elastic-ips-895).

Preserve #19/#173 explicit containment, #106/#182 SG connections and #107/#296
route ownership/presentation. Route ownership and standalone next hops require
matching known ID values; legacy direct containment and two-endpoint association
lowering do not enforce that same complete proof contract. Association tests even
exercise reference-only inputs. Inline routes have their own bounded literal-ID
resolution described in [routes.md](routes.md). New capabilities must not be
described as already guaranteed by those older rules. None of completed
#688–#702 service work is reopened: for example, an App Runner ingress connection
does not supply missing PrivateLink endpoint/service pair proof.

Contract distinctions checked:

- Default VPC/subnet adoption is not an alias of ordinary resources. Default
  subnet `vpc_id` and default VPC CIDR/tenancy are computed. Default ACL/table
  adoption references VPC `default_network_acl_id` / `default_route_table_id`,
  not VPC `id`. Their current Logical defaults omit architecture-bearing context.
  Default DHCP server fields are computed strings, including NetBIOS fields that
  the docs describe as optional lists; see
  `internal/service/ec2/vpc_default_vpc_dhcp_options.go` at the pinned commit.
- Regional NAT uses `availability_mode` and `vpc_id`, not the existing zonal
  subnet rule. `availability_zone_address` is a set, and its `allocation_ids`
  set is optional in schema despite required wording in docs. Generated ENIs,
  route tables and automatic EIPs are not configured managed children.
  `aws_nat_gateway_eip_association.association_id` is computed in
  `internal/service/ec2/vpc_nat_gateway_eip_association.go`, omitted from its docs.
  EIP `allocation_id`, association ID and deprecated EC2-Classic `public_ip`
  are distinct contracts. #895 implements these bounded references with explicit
  mode/value proof; automatic output and IP/DNS inference remain excluded.
- ENI `attachment` is a set with `instance`, while standalone attachment uses
  `instance_id`. ENI SG sets are not covered by the workload SG allowlist.
  Instance Connect generated ENI IDs do not prove Terraform ownership.
  Secondary RDMA networks/subnets are distinct from ordinary VPC/subnet types.
- Route-table `route` and SG inline rules are attribute-as-blocks set objects;
  default ACL rules are actual set blocks. Route `instance_id` is computed.
  Endpoint/core-network/ODB next hops and gateway/main table associations now have
  bounded [#897 contracts](routes.md#typed-route-references-897).
  #902 adds subtype/value/VPC-checked endpoint subnet, SG and route-table
  associations. SG exclusivity sets contain rule IDs, not SG IDs; legacy
  multi-CIDR rules are discouraged but not aliases of modern one-rule resources.
- PrivateLink supports Gateway, Interface, GatewayLoadBalancer, Resource and
  ServiceNetwork variants. `service_region` is producer scope, not endpoint
  deployment region. Service name is distinct from service ID/ARN; service-created
  ENIs/private hosted zones and allowed account-root/wildcard principals do not
  establish managed children. Shared producer LBs remain independent. The bounded
  [#902 implementation](aws-network.md#privatelink-endpoints-and-services-902) adds these
  references and helper owners while preserving the historical review below.
- IPAM pool locale, planning resource region and control/home region differ.
  Pool `source_resource` is a list; CIDR allocation consumer fields are computed.
  Preview is not reservation. OU wildcard paths do not enumerate accounts.
  IPAM organization admin is Global in current defaults; delegated account does
  not overwrite provider account. VPC peering currently has Unknown scope despite
  Spatial Connector classification; requester/accepter orientation needs separate
  evidence and neither VPC contains the other.
- Network Insights path/access-scope selectors configure analyses, not runtime
  topology. Access-scope statements are lists despite set wording in docs.
  Traffic mirror filter/rule/session IDs come from API ID fields despite docs
  calling them names (`vpc_traffic_mirror_filter.go`,
  `vpc_traffic_mirror_filter_rule.go`, `vpc_traffic_mirror_session.go`). Mirror
  target variants require ENI/NLB/GWLB-endpoint discrimination.
- Route Server uses `route_server_id`, endpoint `route_server_endpoint_id`, and
  peer `route_server_peer_id`, without generic `id` outputs. Generated SNS/ENI
  outputs are not configured children; peer IP and ASN do not identify appliances.
  Flow-log source variants and destination type/ARN need separate gates and scope.
  VPC encryption exclusions and block-access flags are configuration, not proof
  of effective encryption, policy compliance or reachability.

Same-name data selectors/flags are recorded separately: a data VPC `default`
lookup is not `aws_default_vpc` adoption. No reviewed managed type is a runtime alias.

Actionable follow-ups are adopted defaults #889; ENI/Instance Connect #890;
prefix-list entries #891; Network Insights #892; secondary networks #893;
traffic mirroring #894; gateways/NAT/EIP #895; flow logs #896; routes/tables #897;
ACLs #898; SG rules #899; IPAM/CIDR #900; VPC controls/DHCP #901;
PrivateLink #902; peering #903; and Route Server #904. These own the listed
source/helper relationships, coordinating endpoint audits without duplicating
their work. Eight types retain sufficient logical identity with concrete
exclusions or summary deferrals; missing functionality remains in these issues.

## Machine learning and generative AI review (#725)

Reviewed planorama commit: `f5c320438d8abf81de31c884bbd03dde5369773e`.
90 managed types: `sagemaker`, `bedrock`, `bedrockagent`, `bedrockagentcore`,
`agentregistry`, `comprehend`, `rekognition`, `transcribe`.

The runtime [matrix](resource-support.md), `basic_entries.rs`, `classification.rs`
and `scope.rs` establish basic support. `service_references.rs`, containment,
ownership rules and summary allowlists have no dedicated rules for these types
at this commit. Inventory and support-matrix tests verify defaults, not complete
relationships. Endpoint and notebook are Spatial; other reviewed types retain
Logical presentation. Architecture-bearing Studio domains, AgentCore runtimes,
gateways, agents, knowledge bases, flows, MLflow services and stream processors
need individual presentation work, not blanket promotion of configuration objects.

Existing [#701](https://github.com/ritz-p/planorama/issues/701) owns the endpoint
configuration/model chain and model role/network references. Remaining source-type
work is bounded by [#872](https://github.com/ritz-p/planorama/issues/872)
(Bedrock), [#873](https://github.com/ritz-p/planorama/issues/873) (agents and RAG),
[#874](https://github.com/ritz-p/planorama/issues/874) (AgentCore execution),
[#875](https://github.com/ritz-p/planorama/issues/875) (gateways and identity),
[#876](https://github.com/ritz-p/planorama/issues/876) (memory/evaluation/policies),
[#877](https://github.com/ritz-p/planorama/issues/877) (Studio/notebooks/images),
[#878](https://github.com/ritz-p/planorama/issues/878) (training/monitoring),
[#879](https://github.com/ritz-p/planorama/issues/879) (catalog/inference auxiliaries),
[#880](https://github.com/ritz-p/planorama/issues/880) (workforce/labeling), and
[#881](https://github.com/ritz-p/planorama/issues/881) (specialized AI services).
Cross-service endpoint audits retain their own types; these issues own the source
paths. No completed #688–#702 scope is reopened.

Contract distinctions checked against the pinned Provider documentation and schema:

- SageMaker `endpoint_config_name` and production-variant `model_name` match
  exported `name`, not ARN. Model names can be omitted for inference components.
  Models/configurations are shared resources, not exclusive endpoint children.
  Model `vpc_config` is a list block containing subnet/SG sets; training-job
  subnet/SG attributes instead use lists. Model connectivity does not establish
  endpoint subnet placement. Container environment and artifact URIs stay opaque.
- Bedrock custom models expose both `custom_model_arn` and `job_arn`.
  Foundation-model IDs, public JumpStart content and builtin evaluators are not
  managed Terraform nodes. Agent guardrail/prompt configurations and alias
  routing are nested **attributes**, unlike many nested blocks elsewhere.
  DRAFT agent configuration, published aliases and numerical guardrail versions
  require distinct identities and version agreement.
- Knowledge-base storage and query-engine discriminators select different
  OpenSearch, RDS, Redshift, Kendra, Neptune and S3 Vectors endpoints. Their VPC
  does not contain the knowledge base. Database/index/variant names need their
  parent context. Flow nodes and prompt variables are internal labels.
- AgentCore runtime VPC mode differs from JWT/OAuth discovery private endpoints.
  Private endpoints can select managed VPC/subnet/SG configuration or an explicit
  Lattice resource; neither proves gateway/registry containment. Browser PUBLIC,
  interpreter SANDBOX and runtime PUBLIC do not inherit VPC context.
  Service-created harness runtimes/memory and MANAGED credential secrets are not
  automatically Terraform-owned children. EXTERNAL secret references never expose
  API keys, OAuth secrets or write-only values.
- Deprecated `aws_bedrockagentcore_registry` and `aws_agentregistry_registry`
  have distinct service registrations and schemas; migration advice is not an
  alias contract. Memory-strategy role/namespaces are deprecated, logging `id`
  denotes region, and Rekognition `stream_processor_arn` is deprecated in favor
  of `arn`. Preserve those distinctions instead of inventing additional identities.
- Studio ownership requires domain-qualified profile/space identity. Generated
  home EFS/ENI outputs do not prove independently managed children. Workforce
  Cognito references need pool/client/group agreement; OIDC usernames, URLs and
  IP policy conditions do not establish resource or network identity.

All dedicated links remain missing at this baseline. Endpoint/notebook cards
do not establish dedicated semantics. Form bodies, scripts, prompts and model
data are excluded; summary candidates require the shared allowlist review.
Data lookups and list registrations are outside the managed denominator.

## Compute and scaling review (#712)

Reviewed planorama commit: `2bef52d045db4c5cf72374a0dca24d0629ed84e3`.
52 managed types: `autoscaling`, `autoscalingplans`, `appautoscaling`, `batch`,
`imagebuilder`, `lightsail`, `outposts`, `computeoptimizer`.

The [runtime support matrix](resource-support.md) and
[inventory tests](../tests/unit/aws_inventory.rs) establish basic defaults.
Auto Scaling groups are Spatial Controllers because explicit classification
overrides their basic inventory entry. Batch compute environments and six
Lightsail endpoint types already have Spatial defaults. Configuration resources
are not promoted simply because the schema contains an ARN.

Existing #361 Launch Template Uses presentation is covered by
[logical rendering tests](../tests/unit/svg/logical.rs); it is not a complete
known-value network validator. #362 policy/group name verification and inverse
policy context are implemented in [autoscaling.rs](../src/provider/aws/autoscaling.rs)
and [its tests](../tests/unit/semantic/autoscaling.rs). Neither implies subnet
containment, instance ownership or complete support for other ASG helpers.
The other reviewed types have basic presentation and generic dependencies,
without dedicated semantic rules or behavioral tests to borrow from ASG.

Follow-up implementations #700 (Batch network/queue) and #810 (ASG
network/launch/traffic) are documented in [AWS semantics](aws.md). Remaining gaps are
#811 (ASG auxiliary owners/notifications), #812 (typed scaling targets),
#813 (Image Builder chains/build infrastructure), #814 (distribution/lifecycle),
#815 (Lightsail relationships and distribution/static-IP presentation), and
#816 (Batch auxiliary compute references). These issues own the listed source
and helper types; endpoint audits must not duplicate their relationships.
Compute Optimizer selectors, Outposts exclusions and Batch job-definition
payloads have concrete deferred contracts in the decisions, not unreviewed
fallbacks. Existing completed #361/#362 and #688–#702 scopes remain intact.

Contract distinctions checked against the pinned schema and documentation:

- ASG subnet IDs are a set; AZ names do not identify default subnets. Direct
  launch templates and base mixed-policy templates retain #361; overrides do
  not inherit that claim. Classic ELB names, ELBv2 and VPC Lattice target-group
  ARNs require their traffic-source discriminator. Shared notifications can
  refer to multiple ASGs and cannot be embedded under an arbitrary one.
- Application Auto Scaling identity includes namespace, scalable dimension
  and resource ID. Formatted ECS/DynamoDB identifiers require complete explicit
  provenance. Scaling-plan tags and metric resource labels are not ownership.
- Batch `compute_resources` is a list block containing subnet/SG sets.
  `compute_environment_order` is a list of blocks in the schema and
  `internal/service/batch/job_queue.go` (`ListNestedBlock`), despite the docs
  calling it a set. It has an explicit numeric `order` and an environment ARN.
  Queue policy input is an ARN. The environment
  name is `name`, while `ecs_cluster_arn` is computed. `instance_role` uses an
  instance profile (the official example references its ARN). Deprecated
  `image_id` differs from `ec2_configuration.image_id_override`. MANAGED and
  UNMANAGED environments and EC2/SPOT/FARGATE variants cannot share assumed
  network evidence. Job definition ARN includes revision; `arn_prefix` does not.
- Image Builder recipes/components/workflows are reusable versioned resources.
  Build infrastructure subnet context does not contain a pipeline or produced
  image. Distribution region/account selectors qualify destination references,
  not the configuration's own scope. Wildcard recipe versions, parent image
  strings, YAML, Dockerfiles and policy/tag selection remain opaque.
- Lightsail attachment inputs use names; instance/database `id` is an ARN,
  while disk/LB/bucket IDs are names. LB certificates are LB-qualified and
  distinct from general Lightsail certificates and ACM. Domain-entry composite
  IDs changed separator from underscore to comma; parse neither as ownership.
  Origin region differs from distribution control region. AZs, public IPs and
  service-default networking cannot prove customer VPC containment. Public-port
  CIDRs and certificate validation records do not identify managed endpoints.
- Compute Optimizer enrollment/member counts do not identify Organizations
  resources. Recommendation selectors distinguish Organization, AccountId and
  ResourceArn. Outposts accepts an Outpost ID or ARN, but the managed denominator
  has no Outpost endpoint; excluded instance IDs are workflow exclusions, not
  children, and the API does not return them after import.

No reviewed type is an alias in [the runtime alias table](../src/provider/aws/aliases.rs).
Commands, environment maps, key material and health/capacity inference are excluded.

## Network connectivity and traffic protection review (#716)

Reviewed planorama commit: `d906716ae153a3d5bd320e64a6205f835850489b`.
68 managed types: `directconnect`, `networkmanager`, `networkfirewall`,
`vpclattice`, `networkflowmonitor`, `networkmonitor`.

All 68 types have basic Logical presentation at this commit.
No dedicated behavioral tests exist for these types.

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

No reviewed type shares a verified managed alias factory. Data lookups remain
outside the managed denominator and do not inherit managed defaults/ownership.
Per-type summary deferrals require an allowlist; rule/policy contents stay excluded.

## Search and business intelligence review (#724)

Reviewed planorama commit: `09b71007288ddc6a7d8662e750acc39278f64ba6`.
65 managed types: `opensearch`, `opensearchserverless`, `elasticsearch`,
`cloudsearch`, `osis`, `kendra`, `qbusiness`, `quicksight`, `finspace`.

OpenSearch domain retains its Spatial card, common-VPC subnet placement #181
and SG connections #182, covered by
[VPC tests](../tests/unit/semantic/vpc_scope.rs) and
[SG tests](../tests/unit/semantic/security_groups.rs). Those legacy rules do
not establish the stricter known-value network contract for other services.
No other reviewed type has dedicated relationship/ownership behavior at this
commit. Serverless collection already has a Spatial card; the other 63 types
are Logical. Basic regional eligibility clears account evidence; OpenSearch
domain retains its pre-existing provider-account handling in scope.rs.

Bounded follow-ups are #797 (search domain/endpoints/configuration), #798
(cross-domain connections), #799 (Serverless group/direct key/identity),
#800 (OSIS), #801 (Q Business), #802 (Kendra), #803 (FinSpace), #804
(QuickSight connectors), #805 (QuickSight content), and #806 (QuickSight
identity/key registration). Existing #696 owns Serverless collection/card and
endpoint VPC/subnet/SG support. Source/helper ownership is assigned to these
issues; endpoint audits must not duplicate their cross-service relationships.
The review does not reopen completed #181/#182 or adjacent #688–#702 work.

Contract distinctions checked against pinned resource documentation and schema:

- OpenSearch and Elasticsearch registrations live in different service packages;
  identically named factories do not make them verified aliases. Their managed
  and registered data schemas remain separate contracts. Data-source lookup
  results do not inherit managed ownership or enter this denominator.
- Domain policy docs describe `access_policies` as optional, while the schema
  requires it. Domain `domain_name`, `domain_id`, and ARN are distinct identities.
  KMS ID inputs may normalize to ARNs. Connection local/remote domain names need
  their respective owner IDs and regions; accepter and initiator remain separate.
- Serverless collection has no VPC input. Collection-group names and direct
  `encryption_config` are configured inputs; top-level `kms_key_arn` is computed.
  The direct encryption configuration is a list-of-object attribute, distinct
  from SDK block schemas. Endpoint `subnet_ids` and `security_group_ids` are sets;
  the latter is optional/computed. Policy JSON cannot prove collection membership.
- OSIS pipeline `id` is deprecated and means its name, while endpoint
  `pipeline_arn` references the ARN. Endpoint `vpc_id` is computed. CUSTOMER and
  SERVICE endpoint-management modes need separate proofs; pipeline YAML remains
  opaque. Enabled log destination uses a log-group name, not ARN.
- Kendra child service IDs differ from composite Terraform IDs. Experience
  data-source/FAQ collections require same-index proof. Legacy S3/web-crawler
  configuration is deprecated; template JSON is opaque. Structured Lambda hooks
  and S3 bucket references remain actionable without reading documents or secrets.
- FinSpace cluster/network settings describe the PrivateLink endpoint, not the
  location of service compute. Its docs misdescribe `vpc_id` as endpoint ID;
  `internal/service/finspace/kx_cluster.go` maps it to API `VpcId`. Environment
  infrastructure account is service infrastructure, not customer ownership.
  Database/volume/scaling-group names need environment qualification; AZ IDs and
  computed attached clusters do not establish subnet or resource membership.
- QuickSight IDs are commonly composite; relationships use the resource-specific
  ID or ARN plus `aws_account_id`, region and namespace. Role membership targets
  a group and a QuickSight enum, not an IAM role. Folder member type discriminates
  analysis/dashboard/dataset. Built-in themes and default namespace literals do
  not manufacture managed nodes. Dataset table-map keys are local identities;
  SQL, visual definitions and RLS policy contents are not interpreted.
- QuickSight VPC connection does not contain its consumers. Data-source parameter
  variants need type evidence; host/port/URL values cannot reconstruct databases.
  IP restriction maps require map-key reference provenance before safe navigation
  can be reconsidered. Key registration does not prove encryption of each dataset.

No safe-summary allowlist covers these services at this baseline; each type
records its exclusion or revisit condition. Shared proof and privacy rules apply.

## Containers and application hosting review (#713)

Reviewed planorama commit: `f30289f73269a8a348818dd6e2a8bc44c22f22aa`.
57 managed types: `ecs`, `eks`, `ecr`, `ecrpublic`, `apprunner`,
`elasticbeanstalk`, `appmesh`, `servicediscovery`.

Behavioral evidence is [ECR reference tests](../tests/unit/semantic/service_references.rs),
[ECS tests](../tests/unit/semantic/ecs_clusters.rs),
[EKS tests](../tests/unit/semantic/eks.rs),
[basic inventory tests](../tests/unit/aws_inventory.rs) and the runtime support matrix.
Basic-only types do not inherit adjacent services' tested semantics.

Existing ECR repository-policy/lifecycle references, ECS cluster navigation and
service network placement, and EKS cluster/role references and network placement
remain bounded by [their contracts](aws-compute.md#ecr-repositories). They do not establish
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

No reviewed type is a verified managed alias. Scope eligibility proves neither
account identity nor member region. Environment/secret maps, container JSON,
policy bodies and catalog payloads stay excluded even without sensitive markers.
References require correct output attributes and known nonempty values.

## NoSQL, caches and time-series review (#718)

Reviewed planorama commit: `bebadbbeac605bae9efd6810957a89130616d947`.
39 exact managed types owned by #718. Current status/gap text includes later
implementation such as #691; review objects retain this baseline.

Additional evidence: [DynamoDB verification](../src/provider/aws/dynamodb.rs), [Backup](../src/provider/aws/backup.rs).
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
55 managed types in the fixed #721 scope.

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

## Relational and graph database review (#717)

Reviewed planorama commit: `5d233e4a467f323b46082a35ede1467eec999898`.
59 managed types: `rds`, `docdb`, `docdbelastic`, `neptune`, `neptunegraph`,
`dsql`, `odb`.

Additional evidence: [proxy validation](../src/provider/aws/rds_proxy.rs), [Backup](../src/provider/aws/backup.rs).
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

No reviewed type is a verified managed alias. Shared parameter/option/subnet
groups stay independent. Passwords, SQL, connection endpoints, clone data and
policy bodies are excluded. Per-type deferrals specify contract/use-case prerequisites.

## Filesystems, transfer and backup review (#720)

Reviewed planorama commit: `514c17316e596fbbd0a8c7aa4942a91e3462e09d`.
70 managed types: `efs`, `fsx`, `storagegateway`, `datasync`, `transfer`,
`backup`, `dlm`, `rbin`, `drs`.

Additional evidence: [EFS ownership](../src/provider/aws/efs.rs), [Backup selection](../src/provider/aws/backup.rs).
EFS #497/#548 is covered by [EFS tests](../tests/unit/semantic/efs.rs) and
[reference tests](../tests/unit/semantic/service_references.rs); Backup #508 by
[Backup tests](../tests/unit/semantic/backup.rs). Other types cannot borrow those
behavioral guarantees from a shared service label.

EFS filesystems stay regional; access-point embedding is ownership, while mount
target subnet placement is network containment. Existing EFS SG collection proof
is less strict than newer complete known-value network validators; #824 owns that
bounded extension. Backup retains single-rule vault proof and explicit supported
resource selections; multiple rules, tag filters and wildcards are not verified
coverage. FSx filesystems are already Spatial; cache, Transfer server/connector/web
app, DataSync agent, Storage Gateway and air-gapped vault defaults need explicit
architecture review in their follow-ups. Helpers do not all become spatial cards.

Actionable gaps are Backup vault/helpers #820, plan rules #821, reports #822,
restore testing #823, EFS extensions #824, Lustre backup/logging #825, other FSx
networking #826, FSx children/associations #827, DataSync agents/locations #828,
tasks #829, DLM/DRS #830, Storage Gateway #831, Transfer servers #832,
identity/key helpers #833, AS2/connectors #834, web apps #835 and workflows #836.
Existing #702 exclusively owns Lustre subnet/SG/KMS proof. These source/helper
relationships coordinate with endpoint audits #710/#719/#726/#727/#730 and
VPC Lattice #791, without reopening completed EFS/Backup implementations.

Contract distinctions verified against pinned documentation and schema:

- Lustre `subnet_ids` is a list and `security_group_ids` a set. Its KMS input is
  an ARN for persistent deployment types. Computed VPC/ENI outputs and S3 import/
  export URI strings are not configured endpoint proof. Other FSx variants have
  different preferred-subnet, route-table, AD and volume contracts.
- OpenZFS `parent_volume_id` can match a filesystem `root_volume_id` or another
  volume `id`; an origin snapshot is clone provenance, not ownership. ONTAP
  volume ownership uses SVM ID, while its filesystem ID is computed. Repository
  association `arn` is not a filesystem endpoint despite its documentation label.
- EFS replication destination KMS accepts ID/ARN/alias/alias ARN, unlike the
  filesystem's documented ARN input. Optional/computed destination filesystem
  output cannot create a configured replica link. Destination region must remain
  distinct from source/control scope.
- DataSync uses subnet/SG ARNs, not IDs. ONTAP location configures SVM ARN and
  exports filesystem ARN. Lustre/OpenZFS/Windows location SG ARNs are **required**
  in schema, despite docs calling them optional (confirmed in
  `internal/service/datasync/location_fsx_lustre_file_system.go` and sibling
  resource schemas). HDFS KMS URI is not AWS KMS. S3 input can be an Outposts
  access-point ARN. Task report `bucket_access_role_arn` is an IAM role despite
  the docs saying policy; `internal/service/datasync/task.go` maps it to
  `BucketAccessRoleArn`.
- Storage Gateway iSCSI `network_interface_id` accepts IPv4 addresses, not EC2
  ENI IDs. Local disk IDs/paths are not EBS IDs. Gateway EC2 host is computed;
  endpoint DNS and activation IP do not establish configured containment.
  File-share backing storage can include a prefix and has its own bucket region.
- Backup vault ID is its name; plan ID is distinct from display name. Plan rules
  are sets, so positional matching cannot extend existing single-rule proof.
  Copy destinations have independent scope; restore include/exclude wildcards,
  framework controls and tag conditions do not identify concrete protected sets.
- DLM tags and snapshot regex selectors remain opaque. SSM execution handlers
  accept custom name/ARN or AWS-owned sentinels; copy keys require destination
  scope. DRS `ebs_encryption_key_arn` is optional in schema and
  `internal/service/drs/replication_configuration_template.go`, despite required
  wording in docs. Templates do not synthesize EC2 replication servers.
- Transfer server FTPS certificate is ACM, while AS2 profiles use Transfer
  `certificate_id`. User ownership is server-qualified; SSH key proof needs both
  server ID and username. PUBLIC/VPC/VPC_ENDPOINT and identity-provider modes
  need separate contracts. VPC Lattice egress does not contain the connector.
  Web app computed outputs are nested at
  `identity_provider_details.identity_center_config.application_arn` and
  `endpoint_details.vpc.vpc_endpoint_id`, despite the flat documentation list.
  Workflow normal/exception steps need type-gated Lambda/EFS/S3 references;
  file expressions and object paths remain opaque.

No reviewed type is a runtime alias. Data lookups do not add managed types;
credentials, contents, policy interpretation and runtime health/recoverability
remain excluded under the shared proof and privacy rules.

## Analytics and data governance review, batch 1 (#722)

Reviewed planorama commit: `c50d830c5737a51a362708b71f67e6c407884dbe`.
81 managed types: Athena, provisioned/Serverless Redshift, Redshift Data,
Glue, Lake Formation and the three EMR packages.

Additional evidence: [database proof](../src/provider/aws/databases.rs), [Glue catalog proof](../src/provider/aws/glue.rs).
[Inventory tests](../tests/unit/aws_inventory.rs) and the
[runtime support matrix](resource-support.md) establish defaults, not full semantics.
[Redshift](../tests/unit/semantic/redshift.rs),
[Serverless](../tests/unit/semantic/redshift_serverless.rs) and
[Glue](../tests/unit/semantic/glue.rs) tests establish their bounded existing proof.
Other types do not borrow those behavioral guarantees merely from a source mention.

Preserve #504/#545 Glue role and catalog/table ownership, #505/#547 independent
Athena cards and generic provenance, #506 provisioned Redshift subnet/parameter
and complete network collection proof, and #695 Serverless networking/name references.
EMR cluster and Serverless application are already Spatial; Studio, virtual cluster,
Glue catalog/development endpoint and provisioned Redshift endpoint need architecture
review in their follow-ups. Shared configurations do not become embedded children.

Actionable extensions are Athena #840; EMR cluster/helpers #841, Studio #842,
Containers #843, Serverless #844; Glue catalog hierarchy #845, networking #846,
workloads #847, encryption #848; Lake Formation #849; Redshift core #850,
snapshots/copy/schedules #851, endpoints #852, workload helpers #853,
catalog integrations #854 and Serverless auxiliaries #855. These issues own the
listed source/helper relationships, coordinating IAM/KMS/S3/EKS/catalog endpoint
audits without duplicating them. RDS-origin zero ETL remains #769, and DMS remains
#699/#723. No runtime behavior changes to close this audit.

Contract distinctions checked against pinned schema and official documentation:

- Athena database `bucket` is optional in schema despite required wording in docs;
  bucket/encryption are not read after import. Workgroup customer-content KMS uses
  `kms_key`, and managed logging uses `kms_key`, despite docs saying `kms_key_arn`
  and `kms_keys`. These paths are confirmed in `internal/service/athena/workgroup.go`.
  Prepared statement uses `name`, qualified by workgroup. SQL and catalog parameter
  values do not prove runtime lineage; typed Lambda parameters need catalog-type gates.
- EMR additional SG inputs are comma-delimited strings, unlike subnet sets.
  Optional/computed service-created SGs and subnets do not supply configuration
  provenance. Instance profile differs from service/scaling role. Fleet/group IDs
  are not EC2 instance identities. Containers provider `id` is EKS cluster name;
  Kubernetes namespace is not an AWS managed child. Templates have no cluster owner.
- Glue names need catalog/database qualification. Existing table ownership requires
  equal known catalog IDs and region; crawler database name alone lacks that proof.
  Connection network belongs to the connection, not every consuming job/crawler.
  Catalog managed workgroup is computed. Registry `id` is ARN; schema version ID is
  distinct from schema ARN. Encryption fields ending `_id` can require KMS ARNs.
  Metastore function owners are not automatically IAM principals.
- Lake Formation principal selectors include account IDs and special principals;
  they are not all IAM ARNs. Tag expressions and wildcards never enumerate matched
  resources. Singular `resource_lf_tag.lf_tag` is a list; plural resource uses a set.
  Federated resource registration can identify Glue connections, not just S3.
  Computed Identity Center application/RAM share are not configured children.
- Provisioned Redshift `encrypted` is a string in schema. Snapshot source ownership
  differs from destination cluster scope; copy grants require destination region.
  Namespace ARN differs from cluster ARN. Data share consumer accepts account/ADX
  selectors; the inventory has no managed datashare producer. Authorization VPCs
  belong to the grantee account. Logging CloudWatch exports contain no log-group
  identifier, and logging `id` is deprecated. Serverless namespace name, UUID and
  ARN remain distinct; snapshot KMS/namespace ARNs are computed outputs.

Data registrations and managed resources remain separate contracts. None of these
types is a runtime alias. New links need unique explicit compatible references,
matching known values, complete collections and independent provider/account/region/
named-state evidence. Current/deposed/deleted/filtered identities stay conservative.
Credentials, query/script/schema bodies, arbitrary maps, policy interpretation and
runtime reachability/health are excluded. Safe capacity summaries are deferred until
configured-versus-observed and unknown-value display contracts are selected.

## Analytics and data governance review, batch 2 (#723)

Reviewed runtime commit: `c50d830c5737a51a362708b71f67e6c407884dbe` on main;
stacked on #722. Scope: 31 managed types in DMS, DataZone, Data Exchange,
Data Pipeline and Clean Rooms.

At this baseline these types have basic display only: no dedicated originating
rules in service_references.rs, databases.rs, containment, service_members.rs or
encryption.rs. Runtime evidence is basic_entries.rs, classification.rs and scope.rs;
[inventory tests](../tests/unit/aws_inventory.rs) and the
[support matrix](resource-support.md) verify defaults. No adjacent service's tests
establish these missing capabilities. DMS replication instance is already Spatial;
its subnet group, task and endpoint are logical configuration identities.
Collaboration/dataset/pipeline/domain/environment/serverless replication defaults
need architecture review in their follow-ups, without promoting every helper.

Keep #699 for provisioned DMS instance/subnet-group/task relationships and exact
`aws_dms_endpoint` task endpoints. Other bounded work is Clean Rooms #857,
Data Exchange #858, Data Pipeline #859, DataZone metadata #860,
environments/deployment scope #861, users/grants #862, DMS endpoint auxiliaries
and distinct S3 endpoints #863, encryption/events #864, migration projects #865,
and Serverless replication #866. These own their source/helper relationships;
coordinate Glue/Lake Formation #722 and IAM/KMS/S3 endpoint audits without duplication.

Contract distinctions checked against official docs, schema and source:

- DMS uses `replication_instance_arn` and `endpoint_arn`, not a generic `arn`
  output. Subnet group identifier is a name; subnet/SG collections are sets.
  `replication_subnet_group_arn` is computed in
  `internal/service/dms/replication_subnet_group.go` but omitted from its docs.
  Serverless nested subnet group is required in schema, despite optional wording.
  DMS certificate is not ACM, and DMS instance profile is not IAM instance profile.
  Engine variants gate certificates, roles and secrets; hostnames, URLs, broker
  lists and database names never reconstruct managed databases or network topology.
- Clean Rooms membership's result bucket is under three list blocks, confirmed by
  `internal/service/cleanrooms/membership.go`. Collaboration accounts are invitations,
  not account-resource ownership. Configured table names need Glue catalog/account
  qualification; the schema has no catalog ID or collaboration-owner input.
- Data Exchange revision Terraform ID combines dataset and revision (confirmed in
  `internal/service/dataexchange/revision.go`), despite misleading dataset-ID docs.
  Revision assets creates its own revision; there is no existing revision ID input.
  Asset blocks form a set and computed access points are not managed children.
- Data Pipeline ID is a pipeline ID (`internal/service/datapipeline/pipeline.go`),
  despite client-certificate wording. Definition `ref_value` references internal
  pipeline objects, not Terraform resources; object fields remain opaque.
- DataZone domain/project/form/glossary references require domain qualification.
  Asset form `type_identifier` and `type_revision` are required under a set in
  `internal/service/datazone/asset_type.go`. Glossary `is_a` is an optional set in
  `internal/service/datazone/glossary_term.go`. These omitted nested docs are verified
  through explicit source-contract entries as well as schema shapes. Environment
  profile `aws_account_id` and glossary-term `domain_identifier` are optional in
  schema despite required documentation. Environment deployment account/region are
  distinct from domain control scope. Built-in blueprints and computed provisioned
  resources are not managed configuration aliases or synthesized children.

No reviewed type is a runtime alias. Personal data, SQL, scripts and schema
contents remain excluded under the shared proof, privacy and verification rules.

## Monitoring, logging and observability review (#730)

Reviewed runtime `00bf0c990596acc196f37c10baf11d2b379c7eaa` against AWS Provider
6.70.0 at `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`. All 77 exact managed types
in the 13 assigned service packages have six separate decisions, pinned input/output
shapes, data-source selectors, official documentation and implementation/test evidence
in [the decisions](aws-semantic-decisions.json). Internet Monitor's undocumented
nested bucket-name field is additionally checked against the pinned Provider source.

Existing RDS `Monitors` / `Monitored by` proof remains limited to a known metric,
AWS/RDS namespace, one DBInstanceIdentifier, matching provider and explicit current
DB instance evidence, without metric queries. These rows do not imply ownership,
spatial edges or health. Log Groups retain #544 independent regional cards and
incoming CloudTrail/Network Firewall/Lambda contracts; #549/#568 alarm placement
and #601's existing encryption allowlist are unchanged.

Follow-ups cover [application monitoring #985](https://github.com/ritz-p/planorama/issues/985),
[alarms #986](https://github.com/ritz-p/planorama/issues/986),
[metric delivery #987](https://github.com/ritz-p/planorama/issues/987),
[log helpers #988](https://github.com/ritz-p/planorama/issues/988),
[log delivery #989](https://github.com/ritz-p/planorama/issues/989),
[Observability Admin #990](https://github.com/ritz-p/planorama/issues/990),
[Omni #991](https://github.com/ritz-p/planorama/issues/991),
[Grafana helpers #992](https://github.com/ritz-p/planorama/issues/992),
[Grafana network/workspace #993](https://github.com/ritz-p/planorama/issues/993),
[OAM #994](https://github.com/ritz-p/planorama/issues/994),
[AMP workspace #995](https://github.com/ritz-p/planorama/issues/995),
[AMP scraper #996](https://github.com/ritz-p/planorama/issues/996),
[RUM #997](https://github.com/ritz-p/planorama/issues/997),
[Synthetics #998](https://github.com/ritz-p/planorama/issues/998) and
[X-Ray KMS #999](https://github.com/ritz-p/planorama/issues/999).

The pinned detailed resource-metrics contract supports only ElastiCache replication
groups. Grafana data-source VPC access differs from workspace ingress restrictions.
AMP EKS/VPC sources and AMP/CloudWatch destinations are exclusive alternatives;
logging ARNs require an explicit `:*` normalization contract, not unrestricted
string matching. OAM accounts remain independent. Omni spaces may be in a different
region from their domain. Computed table buckets, scraper roles, RUM log groups and
canary Lambda/layer outputs cannot synthesize managed resources.

Evidently is deprecated and is not an AppConfig alias; its follow-ups concern retained
plans. RUM's Evidently destination is an experiment ARN, not a project or launch.
The pinned managed inventory has no Evidently experiment or Identity Center instance
resource, so those endpoint contracts remain explicitly deferred. X-Ray `NONE`
means default encryption, not disabled encryption. Grafana token keys, query/SQL/YAML
bodies, policy effects, runtime status and telemetry contents are excluded. Singleton
prerequisites and resource selectors remain ordinary dependencies unless an exact
configuration identity contract is separately established.

This audit changes no runtime behavior or rendering input. SVG regeneration and
Layout quality are not applicable. The audit test, generated-register check, full
pinned contract verifier and generated support-matrix check run in Docker.
