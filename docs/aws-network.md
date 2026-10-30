# AWS Network and edge

[AWS 共通契約・サービス一覧](aws.md)。以下は各節に記載した Provider 版・証拠範囲の契約です。 [HTML 生成手順](aws.md#html-example-generation)。

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

Multiple origins and ordered behaviors require complete indexed expression/value correlation. Unknown, sensitive or dynamic expressions, missing endpoints, data,
deleted/deposed resources and wrong-provider/type endpoints retain ordinary
references and safe unresolved diagnostics. Custom domains and literal IDs are
never matched by name or value alone. Selection marks filtered endpoints
unavailable, and states never resolve through another state's matching address.
Headers, authentication values, domains, URLs and policy bodies are not emitted.

Additional typed paths cover function ARN, Lambda qualified ARN, key-group ID,
field-encryption ID, realtime-log ARN, continuous-deployment policy ID, anycast
list ID, connection-function ID, trust-store ID, VPC-origin ID, origin mTLS ACM
ARN, IAM certificate ID and legacy logging bucket domain name. Both behavior
types use the same rules. Continuous deployment staging DNS items require
explicit distribution domain-name references and a matching known quantity.
Global consumer identity never moves regional endpoints into Global scope.

Checked against AWS Provider 6.70.0 at commit
5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f and the official
[distribution contract](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/cloudfront_distribution.html.markdown).
The synthetic [fixture](../tests/fixtures/cloudfront-plan.json),
[SVG](../examples/cloudfront.svg) and [offline HTML](../examples/cloudfront.html)
show S3/ALB origins, OAC/OAI, policies and an unresolved custom origin.

## CloudFront VPC-origin endpoint context (#916)

A VPC origin retains its Global logical connector identity. Its singleton
vpc_origin_endpoint_config.arn can reference a current managed ALB, NLB or EC2
instance's arn with matching known, non-sensitive values and compatible account
evidence. The VPC-origin output arn and id are never endpoint evidence.
Verified ALB aliases follow the same contract; data, gateway LBs, ambiguous,
deleted/deposed, foreign-provider and incomplete references remain unresolved.

Endpoint subnet context is informational: it requires complete explicit endpoint
subnet references, matching known subnet IDs, compatible regional/account evidence
and a common known VPC. NLB subnet_mapping uses the existing correlated mapping
proof. Missing network evidence does not invalidate a proven endpoint ARN.
No ENI is invented and no Global control resource or regional endpoint is moved.
Selection and named-state identity apply to both endpoint and context references.

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
ALB/stage associations and unresolved configuration.

## Route 53

Route 53 Hosted Zones and Records are spatial nodes with Global scope and the
Amazon Route 53 service label. The [synthetic example](../examples/route53.svg) also has
[offline HTML](../examples/route53.html). `Hosted zone` links require a complete,
unique explicit `zone_id` reference to a managed Zone's `zone_id` or `id`, with
equal known planned values. `Alias target` requires both `alias.name` and
`alias.zone_id` to reference the same managed ALB/NLB (`dns_name`, `zone_id`) or
CloudFront distribution (`domain_name`, `hosted_zone_id`) with matching values.
The LB type must be known as `application` or `network`.

Private-zone VPC sets require a complete explicit ID reference for every element,
matched uniquely by evaluated ID regardless of set order. Each VPC region must
match `vpc_region`, or the provider region when omitted. Unknown and sensitive
IDs/regions remain unresolved. The original singleton contract without provider
configuration is retained; it does not extend to multiple VPCs.
`delegation_set_id` accepts a managed delegation set's `id`. Standalone zone
associations and authorizations expose independently verified zone and VPC
references. Authorization uses `Authorized VPC`, never evidence of an established
association. These extensions require compatible provider/account identities;
`owning_account` is not provider identity. Cross-account and cross-state inference
is unsupported. Zones remain global and independent; helpers remain logical and
do not establish ownership or network containment. These contracts were checked
against the pinned AWS Provider 6.70.0 schema and official source.

These service links supplement ordinary Terraform references. They preserve
source identity, state, actions, provenance, coverage and selection. Incomplete,
dynamic, ambiguous, literal-only, wrong-type/provider and unknown-value proofs
remain unresolved with `UnprovenEndpointValue`; filtered targets are unavailable.
Deleting/deposed objects do not borrow current configuration to prove historical
service links. Local/module aliases without a direct attribute traversal
remain ordinary references. DNS values are compared transiently
and never displayed. Records, DNSSEC, routing policies and DNS resolution are not
evaluated. Links do not guarantee reachability or successful DNS configuration.
The field contract follows the AWS Provider 5.100.0
[zone](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/route53_zone.html.markdown)
and [record](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/website/docs/r/route53_record.html.markdown)
schemas and Terraform 1.9.8 nested expression paths.

Regenerate the Route 53 SVG with `sh scripts/update-examples.sh` and HTML with
`cargo run --locked -- tests/fixtures/route53-plan.json --format html -o examples/route53.html`
inside the development container.

## PrivateLink endpoints and services (#902)

Endpoint network references require a known `vpc_endpoint_type`: `route_table_ids`
for Gateway; `subnet_ids` for Interface, GatewayLoadBalancer, Resource and
ServiceNetwork; `security_group_ids` for Interface, Resource and ServiceNetwork.
Every ID must match a complete unique explicit managed reference and the same
known VPC ID. GatewayLoadBalancer accepts one subnet. `subnet_configuration` is
a set: all known `subnet_id` entries must be unique members of the known
`subnet_ids` set. IP fields are not displayed or used to discover ENIs. These
references retain independent subnet/SG/table identities and the endpoint's VPC
context; they do not assign subnet ownership to a shared service.

`service_name` matches endpoint-service `service_name`, with Interface or
GatewayLoadBalancer subtype agreement. A known `service_region` must equal the
producer service region; a known absent value uses the consumer region.
Endpoint and service provider/account/region scopes remain independent.
Cross-account references require known non-conflicting accounts on both bindings
(or the identical binding); unknown account aliases cannot establish the pair.
Resource `resource_configuration_arn` and ServiceNetwork `service_network_arn`
match the corresponding managed VPC Lattice `arn` in the same region, also
allowing independently proven accounts. The three service selectors are
exclusive. AWS service-name literals do not invent endpoint-service resources.

Endpoint services are independent Spatial connectors. Complete
`network_load_balancer_arns` / `gateway_load_balancer_arns` sets require network /
gateway LB types and Interface / GatewayLoadBalancer service types respectively;
the other LB set must be known absent. `allowed_principals` and permission-helper
`principal_arn` accept explicitly referenced managed IAM role/user ARNs only.
Account roots, wildcards and STS sessions do not identify IAM resources.
`supported_regions` does not create network placement or copy producer scopes.

Route-table, subnet and SG association helpers require known endpoint/peer IDs,
compatible scopes, subtype agreement and the same known VPC ID. This replaces
the legacy reference-only endpoint/route-table lowering: unknown, deleted or
incompatible endpoints preserve the original helper card and dependencies.
Connection accepters pair endpoint/service IDs in the service provider's scope,
while preserving the consumer's independent account/region; matching service
name, subtype and service region are required. Acceptance is not connectivity.

Policy and Interface-only private-DNS helpers belong to their verified endpoint.
Notification helpers select exactly one endpoint/service owner and independently
reference SNS topic `arn`; events remain configuration. Permission and private
DNS verification helpers belong to endpoint-service `id` through
`vpc_endpoint_service_id` and `service_id` respectively. All retain independent
action/identity coverage. Policy bodies, DNS strings, generated ENIs and hosted
zones never establish additional ownership or expose payloads.

New paths require current managed resources, explicit unique output provenance,
known matching values and complete sets. Unresolved, literal-only, ambiguous,
data/deleted/deposed and filtered endpoints preserve conservative fallback.
Legacy direct endpoint VPC containment is unchanged; endpoint SG connections
now additionally require the subtype/VPC/value checks above.

The [fixed Provider 6.70.0 source](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2)
defines optional/computed network sets and generated outputs. Resource and
ServiceNetwork subnet/SG support is also shown in the
[AWS creation examples](https://docs.aws.amazon.com/ec2/latest/devguide/example_ec2_CreateVpcEndpoint_section.html),
beyond the older Interface-only wording in Provider docs. See
[regressions](../tests/unit/semantic/privatelink.rs).

## VPC gateways and Elastic IPs (#895)

Internet gateways and Spatial egress-only gateways use an explicit `vpc_id`
reference to a known matching managed VPC `id` for VPC context. An internet
gateway attachment connects independent gateway/VPC IDs; its composite helper
ID is never an endpoint ID. Ordering dependencies do not establish attachment.

EIP `instance` / `network_interface` references require a known VPC `domain`,
exactly one known endpoint and a known absent alternative. IPAM `ipam_pool_id`
references managed IPAM pool `id`. `public_ipv4_pool` remains unverified: the
pinned inventory has no managed public IPv4 pool endpoint contract. Computed
values and `amazon` literals do not establish pool relationships. EIPs never inherit subnet
containment from an association or advertisement border group.

EIP association helpers connect `allocation_id` to EIP `allocation_id` and
exactly one instance/ENI `id`. Reverse-DNS helpers use that same allocation
output for EIP-owned configuration. Domain/PTR strings do not identify Route53
resources. EIP `id`, public IP and association IDs are not accepted in place of
the selected `allocation_id` output contract.

New NAT allocation references require known `availability_mode` and
`connectivity_type`. Public zonal NAT accepts scalar `allocation_id` and the
complete `secondary_allocation_ids` set. Public regional NAT accepts `vpc_id`
and the complete `availability_zone_address` set, with unique known
`availability_zone_id` values and complete nonempty `allocation_ids` sets.
AZ-name-only or partially unknown blocks retain fallback. Regional mode rejects
subnet, primary and secondary zonal allocation inputs. Private NAT does not gain
EIP references. NAT/EIP association helpers require a public zonal NAT endpoint;
their computed association IDs do not establish allocation identity. Generated
NAT route tables and ENIs do not establish configured ownership.

Legacy NAT `subnet_id` reference containment is retained for zonal or absent
mode; explicitly regional, invalid or unknown mode cannot enter a subnet.
This legacy path does not acquire the stricter value checks of new references.
All new paths require complete unique explicit references, matching known
endpoint outputs, current managed instances and compatible provider/account/
region evidence. Unknowns, literals, data/deleted/deposed or filtered endpoints
retain independent cards and ordinary dependencies. Helpers preserve action and
exactly-once resource coverage when embedded or lowered.

Contracts were checked against the schema/source and docs of
[AWS Provider 6.70.0](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2),
including optional/computed EIP fields and the nested NAT sets. See
[regressions](../tests/unit/semantic/vpc_gateways.rs).

## Security group rule relationships (#899)

The six exact managed types in #899 use the AWS Provider 6.70.0
[pinned EC2 implementation](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2)
and the inventory schema (SHA-256 `f04a44109a82886c3e392ae88e73dfa533f677ccd8a1b253d7d7b0e3d3ea9857`).
The implementation is `security_group_rules.rs`; existing SG VPC containment and
workload attachment proofs retain their previous contracts.

| Source | Input path and shape | Accepted endpoint output |
| --- | --- | --- |
| `aws_security_group` | Optional/computed `ingress` / `egress` sets of objects; `security_groups` string sets and `prefix_list_ids` string lists | Managed `aws_security_group.id` or `aws_ec2_managed_prefix_list.id`, respectively |
| `aws_security_group_rule` | Required scalar `security_group_id`; optional/computed scalar `source_security_group_id`; optional string list `prefix_list_ids` | Managed SG `id` for owner/peer; managed prefix-list `id` |
| `aws_vpc_security_group_ingress_rule`, `aws_vpc_security_group_egress_rule` | Required scalar `security_group_id`; optional scalar `referenced_security_group_id` / `prefix_list_id` | Managed SG `id` for owner/peer; managed prefix-list `id` |
| `aws_vpc_security_group_rules_exclusive` | Required scalar `security_group_id`; required string sets `ingress_rule_ids` / `egress_rule_ids` | Managed SG `id`; corresponding modern ingress/egress rule `id` or `security_group_rule_id` |
| `aws_vpc_security_group_vpc_association` | Required scalar `security_group_id` / `vpc_id` | Managed SG `id` / VPC `id` |

Legacy rules require known `type = ingress` or `egress`. Peer SG references reject
conflicting CIDRs or `self`; prefix lists may coexist with other legacy selectors.
Modern peer references require known `ip_protocol` and no competing selector.
Their resource type supplies direction. Inline paths retain ingress/egress
direction and require complete known collections across the corresponding block.
Names accepted by AWS in a default VPC are not accepted as universal SG IDs.

Each relationship requires explicit, unambiguous Terraform output references,
matching known nonempty values, current managed AWS identities and compatible
provider/account/region evidence. Unknown, literal-only, partial, mismatched,
ambiguous, data, deleted or deposed endpoints retain fallback presentation.
Collections use one output attribute consistently and must contain exactly the
referenced endpoint values. Different
endpoints with the same value are ambiguous. No resources are synthesized from
CIDRs, IPs, names, policies or service-created prefix-list outputs.

Rules are presentation members of their verified SG, retaining separate actions
and coverage. The SG remains shared configuration. Modern rule `id` and
`security_group_rule_id` are computed string outputs; legacy `id` represents a
different, potentially multi-CIDR identity and is not accepted in exclusive sets.
An exclusive helper requires both direction sets, including known empty sets;
every referenced rule must have the same independently verified SG owner.
The helper belongs to the SG and never takes ownership of its rules.

The additional-VPC helper lowers only with both verified endpoints, preserving
its action and exactly-once coverage. It does not move or clone the SG or alter
its original VPC containment. Its computed `state` is runtime status and does not
gate configuration display. Filtering and named-state identity stay independent;
the shared engine applies these rules to native and Wasm SVG/HTML generation.
Descriptions, ports, protocol summaries, CIDR contents and runtime reachability
are not exposed by this change. Existing deferred summaries remain deferred.

## VPC peering (#903)

`aws_vpc_peering_connection.vpc_id` and `peer_vpc_id` accept explicit unique
managed `aws_vpc.id` references with matching known non-sensitive values. The
connection keeps requester control scope. Peer account and region are checked
independently against the peer provider; optional/computed `peer_owner_id` and
`peer_region` never overwrite requester scope. Absent selectors require compatible
same-account/region evidence, while unknown selectors do not imply defaults.
Incoming route next-hop behavior is unchanged. Peering does not imply containment.

Accepter and options helpers reference one managed connection's `id` through
required scalar `vpc_peering_connection_id`. Both configured VPC endpoints must be
proven before helper navigation is established. The accepter uses peer control
scope. Computed accepter `vpc_id`, `peer_vpc_id` and `peer_owner_id`, when present,
must follow the provider read orientation: cross-account reads reverse requester
and accepter; same-account reads retain requester orientation. Its `peer_region`
is always the accepter region. Computed fields never originate new endpoint links.

Options `requester` and `accepter` are optional/computed singleton lists with a
boolean `allow_remote_vpc_dns_resolution`. Only configured blocks select the side
to validate; computed blocks do not request management of the other side. Configured
blocks require known booleans and matching side scope. Helpers embed only when
presentation scope matches the connection; cross-scope helpers retain independent
Logical identities and verified navigation. DNS options never identify hosted
zones or imply reachability. Data, deleted/deposed, filtered, ambiguous, unknown,
sensitive and incompatible endpoints retain conservative fallback.

Contracts follow AWS Provider 6.70.0 at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`, including the shared
`resourceVPCPeeringConnectionRead` implementation for requester and accepter.
