# AWS Network and edge

## IPAM and configured CIDR references (#900)

These are Provider 6.70.0 contracts pinned at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`. References require one unique,
explicit, current managed AWS endpoint, exact output attributes, equal known
non-sensitive values and independently compatible provider/account/region scope.
The singleton Outpost data lookup below is the only data-mode exception. Unknown,
partial, ambiguous, malformed, foreign, deleted and deposed evidence stays
unproven. Filtering and named states retain their original identities and coverage.

| Input / source helper | Endpoint / additional proof |
| --- | --- |
| Subnet CIDR reservation `subnet_id` | `aws_subnet.id`; known `reservation_type = explicit` or `prefix` |
| VPC/Subnet `ipv4_ipam_pool_id`, `ipv6_ipam_pool_id` | Managed pool `id`; exact `address_family` and known pool `locale` matching the consumer's deployment Region |
| Pool `ipam_scope_id` | Managed IPAM scope `id`; default-scope outputs do not stand in for a managed scope |
| Pool `source_ipam_pool_id` | Parent pool `id`, equal known address family and `ipam_scope_id`, compatible control scope |
| Pool `source_resource.resource_id` | `aws_vpc.id`; one complete block with `resource_type = vpc`, matching `resource_owner` and actual VPC `resource_region`; the independently verified parent pool's `locale` must match that Region; a supplied planning-pool locale must agree |
| Scope `ipam_id` | Managed IPAM `id`; address-allocation scope is not deployment scope |
| Pool CIDR provision/allocation/managed preview `ipam_pool_id` | Managed pool `id`; helper composite IDs, `ipam_pool_cidr_id` and allocation IDs are distinct namespaces |
| IPv4/IPv6 VPC CIDR association `vpc_id` | Managed VPC `id`; allocation content does not identify a subnet |
| IPv4 CIDR association `ipv4_ipam_pool_id` | Pool `id`, IPv4 family and allocation locale |
| IPv6 CIDR association `ipv6_ipam_pool_id` | Pool `id`, IPv6 family and locale; generated-IPv6 mode must be absent/false and `ipv6_pool` absent |
| IPAM organization admin `delegated_admin_account_id` | Explicit Organizations account `id`; delegated identity is separate from the administration/control account |
| Discovery association `ipam_id`, `ipam_resource_discovery_id` | Complete IPAM/discovery `id` pair; known `ipam_region` must match the IPAM home Region and the association must use the discovery home Region; `owner_id` must independently match the discovery provider account |

A pool's control Region can differ from its allocation locale and the VPC's
Region. Such links require independently known compatible accounts; a matching
locale or ID never supplies account evidence. Planning VPC proof also requires
the parent reference's final account/control binding. Discovery associations can
cross IPAM home Regions and discovery-owner accounts only with the complete
independent home/owner proof. Missing either endpoint prevents both dedicated
navigation and association lowering.

`aws_subnet.outpost_arn` accepts only the exact `arn` of a current singleton
[`data.aws_outposts_outpost`](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/d/outposts_outpost.html.markdown).
Its provider Region must be known and agree, without a data-resource Region
override. A known `owner_id` must match independently verified provider accounts;
at least one known subnet AZ qualifier must match, and every other known qualifier
must agree. The schema makes `owner_id` optional, while the pinned
[data-source implementation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/outposts/outpost_data_source.go)
populates it on read. There is no managed Outpost alias, plural-lookup shortcut,
ARN substring lookup or Outpost network container.

Owned reservations, IPAM scopes, pool helpers and VPC CIDR helpers retain their
own lifecycle, actions and provenance. Existing member presentation embeds a
verified helper only when its owner has a Spatial presentation and compatible
independent scope. Pools, scopes and discovery resources keep Logical defaults;
allocation hierarchy does not contain VPCs/subnets. Independently verified
provider accounts are retained for these managed IPAM endpoints/helpers. The
organization-admin helper stays Global with its account unresolved; its
Organizations ARN never supplies a deployment account.

CIDRs, IP overlaps, AZ equality alone, computed allocation `resource_id` or
`resource_type`, default SG/ACL/route/scope outputs and preview CIDRs never create
resources or consumers. In this pinned Provider there is no registered managed or
data IPv6-pool endpoint for BYOIP `ipv6_pool` strings; these stay opaque and
unsupported until an exact endpoint contract exists. A managed preview is not a
data alias or allocation. Authorization messages/signatures, arbitrary summaries
and runtime allocation/health inference are excluded. Existing VPC/Subnet
containment and legacy reference contracts are unchanged.


## Managed prefix-list entries (#891)

`aws_ec2_managed_prefix_list_entry.prefix_list_id` accepts only an explicit,
unique, current managed `aws_ec2_managed_prefix_list.id` with equal known
non-sensitive values and compatible provider/account/region evidence. Data lookups
and AWS-managed prefix lists do not acquire this customer-managed ownership rule.
CIDRs, descriptions, ARN substrings and literal ID matches do not identify owners.

Verified entries retain their own actions, provenance and coverage, plus member
owner metadata and navigation. List and entry defaults stay Logical. Existing
member presentation can embed the entry when both are presented spatially; filtering
the list makes that owner unavailable. Ownership is not network containment, and
independently verified provider accounts are retained for these managed types.
The `prefix_list_id` input and `id` output were checked against AWS Provider 6.70.0
at `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`.

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

Route 53 Profiles associations use explicit profile `id` and VPC `id` references.
Resource associations accept only private Hosted Zone, Resolver Rule and DNS
Firewall Rule Group `arn` outputs. A zone requires known nonempty VPC configuration;
public/unknown zones remain unresolved. Query-log and VPC-endpoint variants are
not enabled. `resource_properties` JSON and computed `resource_type` do not create
topology. References require current managed endpoints, known non-sensitive equal
values and compatible provider/account/region evidence. Zones keep global scope;
regional targets keep their own region. Shared profiles remain independent;
incompatible/cross-state endpoints do not establish ownership or VPC containment.

DNS Firewall rules reference a managed rule-group `id` and, for standard rules,
a managed domain-list `id`. The domain-list link requires absent Advanced-rule
threat/confidence fields, including the computed threat-protection ID; no managed
endpoint is synthesized for that ID. A uniquely proven group owns the rule helper.
Rule-group associations independently reference group and VPC IDs without moving
the shared group into a VPC. Provider/account/region, current managed identity,
explicit indexed provenance and known non-sensitive matching outputs are required.
Shared-by-me owner evidence must match the group's provider account; shared-with-me
or incompatible accounts remain unresolved. This does not infer DNS enforcement,
effective policy or network containment.

Query logging uses explicit current managed references and matching known outputs.
`aws_route53_query_log.zone_id` accepts hosted-zone `id`/`zone_id` and supplies
unique helper ownership; `cloudwatch_log_group_arn` accepts only a log-group `arn`
in us-east-1. It does not prove delivery permissions.
Resolver query-log `destination_arn` accepts S3 bucket, CloudWatch log-group or
Firehose delivery-stream `arn`. Associations independently verify configuration
`id` and VPC `id`; neither the shared configuration nor its destination becomes
a VPC child. Regional endpoints must match the configuration region. Cross-account
configuration references require explicit owner-account evidence and SHARED_BY_ME;
unknown owner/share evidence remains unresolved. Destination scope is independent.
Unknown, sensitive, dynamic, ambiguous, deleted/deposed, data, incompatible provider
or mismatching output values do not prove dedicated relationships. Named states
remain separate; no literal ARN search, permission or runtime delivery inference.

Route 53 Hosted Zones and Records are spatial nodes with Global scope and the
Amazon Route 53 service label. The [synthetic example](../examples/route53.svg) also has
[offline HTML](../examples/route53.html). `Hosted zone` links require a complete,
unique explicit `zone_id` reference to a managed Zone's `zone_id` or `id`, with
equal known planned values. `Alias target` requires both `alias.name` and
`alias.zone_id` to reference the same managed ALB/NLB (`dns_name`, `zone_id`) or
CloudFront distribution (`domain_name`, `hosted_zone_id`) with matching values.
The LB type must be known as `application` or `network`.
Classic ELB uses `dns_name` / `zone_id`; Global Accelerator uses `dns_name` or
`dual_stack_dns_name` / `hosted_zone_id`. These extensions require compatible
provider/account evidence and preserve the endpoint's independent scope.
Other alias endpoint families remain unsupported.

Records can reference a health check by `id`, a CIDR collection by `id`, and a
CIDR location by `name` only with a proven reference to that same collection.
The location's deprecated composite `id` is not a collection or location-name
identity. CIDR locations expose collection membership. Traffic policy instances
reference their hosted zone and a policy `id` with an equal known positive numeric
`version`; policy documents and service-generated records are not interpreted.

Exclusive record controllers use `resource_record_set.alias_target.dns_name` /
`hosted_zone_id`, distinct from a record's `alias.name` / `zone_id`. Their complete
sets are correlated through explicit typed endpoint references, including nested
health-check, CIDR, traffic-policy-instance and alias references. Ambiguous rows,
literal-only endpoint values and incomplete lists remain unproven. A unique proven
zone reference permits the controller to appear under that zone's record-management
section. No managed record identities are synthesized; shared endpoints stay visible.

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

## Adopted default networks (#889)

AWS Provider 6.70.0 at commit
5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f defines default resources as adoption
contracts. They are not aliases of ordinary VPC resources.

Default VPC and subnet resources are independent spatial containers; a default
route table is a spatial connector. No default ACL, SG or table is synthesized.
Default subnet availability_zone is required, while availability_zone_id,
cidr_block and vpc_id are computed-only. AZ, CIDR and computed VPC values do not
establish containment; the subnet stays independent without configured proof.

Default ACL default_network_acl_id and default table default_route_table_id
must reference the corresponding output on an explicit aws_vpc or aws_default_vpc.
The adopted resource id must agree, and its known vpc_id must equal that VPC id.
Default SG vpc_id is optional/computed: only a configured reference to VPC id
establishes context. Missing configuration never adopts a VPC by value alone.
These references establish network context, not embedded helper ownership.

Default ACL subnet_ids requires a complete set of explicit subnet id references
and agreeing known VPC values. Default SG ingress/egress security_groups and
prefix_list_ids use complete typed id references; default SG is accepted as an
SG endpoint by typed rule references. Names and IDs are distinct.

Default table route is an optional/computed attribute-mode set. Explicit typed
route references cover gateway, NAT, transit gateway, peering, ENI, egress-only
gateway, instance, GatewayLoadBalancer endpoint, core-network ARN and destination
prefix-list ID; propagating_vgws is a complete set of VPN gateway IDs. Unsupported
or mixed incomplete route evidence remains unresolved. Explicit standalone routes
can use a current default table id as owner. No computed route child is invented.

All new proof requires current managed AWS endpoints, complete explicit instance
references, known matching nonempty output values and compatible provider,
account and region evidence. Unknown, literal-only, ambiguous, deleted/deposed,
data or filtered endpoints keep independent resource identity and fallback.
Named states do not borrow evidence. Inline rules, permissions, health and
reachability are not inferred or summarized.

Pinned source contracts:
[default subnet](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2/vpc_default_subnet.go),
[default ACL](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2/vpc_default_network_acl.go),
[default table](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2/vpc_default_route_table.go),
[default SG](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2/vpc_default_security_group.go).

## VPC controls (#901)

AWS Provider 6.70.0's pinned [EC2 resource schemas](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/ec2) define scalar ID contracts:

- Public-access exclusions require exactly one of vpc_id/subnet_id, matching the explicitly referenced VPC/subnet id, and known allow-bidirectional or allow-egress mode. Computed resource_arn is not provenance.
- Encryption controls require a matching VPC id and known monitor/enforce mode. Gateway/peering exclusion flags identify service categories, not endpoints or KMS keys.
- DHCP associations require matching VPC and DHCP-options id references before lowering. The default sentinel is excluded; shared options remain independent.

Unique explicit current managed references, known matching values and compatible provider/account/region bindings are required. Proven control helpers embed with their own actions and coverage; unknown, incomplete, ambiguous, mismatched, deleted/deposed or filtered owners retain independent fallback. No DNS settings, policies, runtime connectivity or reachability are inferred. See the [fixture](../tests/fixtures/vpc-controls-plan.json).

## CloudFront functions and KeyValueStore helpers (#913)

Contracts use AWS Provider 6.70.0 at `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`: the [function schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/function.go), [connection-function schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/connection_function.go), and [key-helper documentation](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r/cloudfrontkeyvaluestore_keys_exclusive.html.markdown).

| Source path | Verified endpoint and gate |
| --- | --- |
| Function key_value_store_associations | Complete nonempty ARN set to current managed KeyValueStore.arn; known runtime cloudfront-js-2.0 |
| Connection function connection_function_config.key_value_store_association.key_value_store_arn | Singleton config and singleton association, exact store.arn and known runtime cloudfront-js-2.0 |
| Key / exclusive-keys helper key_value_store_arn | One exact store.arn; deprecated composite key.id is not store identity |

The connection-function schema uses a singular nested association, despite the documentation's top-level plural argument description. Its connection_function_arn and id remain separate from a normal function's arn. Known `publish = true` is required when existing CloudFront consumer paths reference either function type; absent, false, unknown or sensitive publish values do not establish a live endpoint. Store configuration references remain valid for development functions. No new distribution consumer paths are introduced.

All selected resources retain Logical presentation and Global service scope with independent account identity. Key helpers retain unique verified store navigation without embedding or claiming ownership of other key resources. Complete explicit provenance, exact known outputs and compatible accounts are required; literal, partial, unknown, masked, ambiguous, deleted/deposed, data or filtered endpoints remain unresolved. Function code and all key/value payloads are excluded, including non-sensitive schema fields. Exclusive reconciliation is not a managed key inventory, and destroying it does not delete keys. See the maintained [function/store example](../examples/cloudfront-functions.svg).
## CloudFront keys, field encryption and trust stores (#914)

Contracts follow AWS Provider 6.70.0 at `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`: [key groups](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/key_group.go), [field encryption configuration](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/field_level_encryption_config.go), [profiles](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/field_level_encryption_profile.go) and [trust stores](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/trust_store.go).

| Source path | Verified identity |
| --- | --- |
| Key group items | Complete nonempty string set of managed public_key.id outputs |
| Encryption config content_type_profile_config.content_type_profiles.items.profile_id | Profile.id per indexed set element, through two singleton lists; known absent optional profile IDs are skipped |
| Encryption config query_arg_profile_config.query_arg_profiles.items.profile_id | Required profile.id per indexed set element, through two singleton lists |
| Encryption profile encryption_entities.items.public_key_id | Public_key.id per indexed set element, through a singleton list |
| Trust store ca_certificates_bundle_source.ca_certificates_bundle_s3_location.bucket | Explicit S3 bucket.bucket or bucket.id with known configured region matching the bucket's independently known region |
| Same S3 location key / version | Explicit S3 object.key and optional object.version_id for the same object, itself explicitly referencing the proven bucket; object and bucket regions must match the location region |

Both trust-store blocks are singleton lists. Modern aws_s3_object and deprecated aws_s3_bucket_object are checked as separate exact types. A present version must be known and explicitly reference the same object's version_id; unknown, sensitive or literal-only versions leave object provenance unresolved. Bucket identity can remain verified independently. Global CloudFront provider region does not set the bucket region, and late provider account evidence is checked for the bucket, object and consumer separately.

All key groups, profiles, encryption configurations, public keys and trust stores retain independent Logical identity, Global scope and account evidence. Complete explicit references and matching known outputs are required; reordered sets require corresponding indexed evidence, and malformed masks, missing/literal rows, ambiguous IDs, inactive lifecycle, wrong provider/type or unavailable selected endpoints remain unresolved. These references do not add ownership, network containment or distribution consumer rules. Content/query matching rules, encryption provider_id, encoded keys and certificate contents are opaque and excluded from rendering. See the maintained [security example](../examples/cloudfront-security.svg).
## CloudFront monitoring and realtime log destinations (#915)

Contracts use the pinned AWS Provider 6.70.0 [monitoring subscription schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/monitoring_subscription.go) and [realtime log schema](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/cloudfront/realtime_log_config.go).

| Source path | Verified endpoint and presentation |
| --- | --- |
| Monitoring subscription distribution_id | One current managed CloudFront distribution.id; the helper embeds only under a proven Spatial owner with compatible account scope |
| Realtime log endpoint.kinesis_stream_config.role_arn | Exact IAM role.arn, gated by known endpoint.stream_type = Kinesis |
| Realtime log endpoint.kinesis_stream_config.stream_arn | Exact Kinesis stream.arn with independently known regional scope, under the same stream-type gate |

The schema requires exactly one endpoint and one kinesis_stream_config list element. Empty, multiple, masked or partial blocks are unresolved. All paths require explicit unique managed provenance, exact known values and compatible provider/account identity, including late account checks. CloudFront stays Global, IAM stays Global, and a Kinesis region is never replaced by the CloudFront provider region. Realtime log configurations remain independent Logical resources shared across behaviors; existing distribution-to-log references are unchanged.

Metrics status Enabled/Disabled and unknown status do not change the configured distribution owner. Metrics do not establish a CloudWatch alarm or log group, and IAM references make no effective-permission or delivery claim. Unproven, inactive, ambiguous or filtered endpoints retain fallback and original action/coverage; named states remain distinct. Policy contents, credentials and runtime metrics are excluded. See the maintained [monitoring/log example](../examples/cloudfront-observability.svg).

## Secondary networks (#893)

`aws_ec2_secondary_network` and `aws_ec2_secondary_subnet` are independent spatial
containers, never aliases of VPCs or ordinary subnets. Containment requires a unique
explicit managed subnet `secondary_network_id` reference to the managed network's
computed `secondary_network_id`, equal known values, known `network_type = rdma`,
and compatible provider/account/region evidence. Unknown, sensitive, ambiguous,
deleted/deposed, data-mode and literal-only evidence cannot establish containment.
The generic `id` output is not accepted by this contract.

The pinned AWS Provider 6.70.0 source defines `availability_zone` and
`availability_zone_id` as separate optional/computed strings. Neither is translated
into the other or used to identify the parent network. CIDRs and service-generated
outputs do not create resources. Both containers retain independent actions and
coverage; filtering and named states retain the standard reference boundaries.

## Route Server (#904)

Route servers and endpoints have independent spatial cards. Managed references use
`route_server_id` and `route_server_endpoint_id` outputs; generic `id` is not an
alias. Peers belong only to their uniquely verified endpoint. Propagation and VPC
association helpers require both explicit typed endpoints with matching known
values before lowering, preserving their actions and resource coverage.

Endpoint subnet containment additionally requires equal known endpoint/subnet
`vpc_id` values and exactly one verified association of that server with the VPC.
Missing or ambiguous context stays uncontained. Provider/account/region, current
managed lifecycle, complete provenance, and unknown/sensitive evidence are checked
independently. Generated ENIs, SNS topics, peer IPs, ASNs and propagated routes never
synthesize managed resources or prove reachability.

## Lambda Core connectors (#954)

The managed Lambda Core network connector has independent regional placement.
`operator_role` accepts only a unique explicit managed IAM role ARN with a matching
known value and compatible account/provider evidence. The role is not owned by the
connector.

The single `configuration` and `vpc_egress_configuration` list blocks must contain a
known `MicroVm` compute-type list before egress references are accepted. Complete
subnet and security-group sets must match known managed endpoint IDs. VPC placement
requires every endpoint's unique explicit `vpc_id` to prove the same current VPC,
including known matching values and compatible independent scope evidence. Unknown,
sensitive, incomplete, incompatible or ambiguous evidence stays conservative.
Generated ENIs and consumer images are not children and do not inherit placement.

## Global Accelerator (#917)

Standard accelerators have independent global cards. Listener `accelerator_arn`
and endpoint-group `listener_arn` references accept only the exact standard managed
ARN outputs; custom-routing types cannot cross-match. Verified unique helpers may
embed while retaining their independent actions and coverage.

An enabled singleton `attributes` block can reference an S3 bucket through
`flow_logs_s3_bucket -> bucket`. Each `endpoint_configuration` set element is
correlated with its explicit expression and known evaluated value. Supported
endpoints are application/network LB ARNs (including verified LB aliases) and EIP
`allocation_id`; the known `endpoint_group_region` must match each regional target.
`attachment_arn` accepts the explicit cross-account attachment ARN without claiming
permission or reachability. Provider/account and state boundaries remain independent;
no endpoint is located inside the accelerator.

The pinned Provider 6.70.0 endpoint-group documentation establishes LB ARN and EIP
allocation ID contracts. EC2 instance ID support is not claimed without additional
explicit source evidence. DNS and hosted-zone consumer semantics remain Route53's
responsibility. Literal names, IPs and ARN-string matching are not reference proof.

## CloudFront multitenant (#912)

Multitenant distributions have independent spatial cards and are never runtime
aliases of standard distributions. Their known `connection_mode` must be
`tenant-only` before consumer references or tenant-to-distribution references
are accepted. Missing/unknown mode remains conservative.

Both `default_cache_behavior` and `cache_behavior` support explicit cache-policy,
qualified Lambda (`lambda_function_arn`), published CloudFront function, trusted key
group (`trusted_key_groups.items` in a singleton block), field-encryption and realtime-log references. Origins support the existing
typed domain, OAC, VPC-origin and client-certificate contracts. All nested collection
references remain complete and element-correlated. Tenant templates are not expanded
or substituted. Standard `ordered_cache_behavior`/`lambda_arn` paths are not aliases
on the multitenant type.

Tenants reference only explicit multitenant distribution IDs, connection-group IDs,
and configured ACM/WAF customization ARNs. Viewer certificates require ACM in
us-east-1; WAF references require CloudFront scope. Connection groups reference
Anycast list IDs only. Omitted default groups, routing-endpoint DNS and managed
certificate requests do not create configured children. Shared endpoints keep
independent identity, scope, coverage and selection; these are configuration
references, not network containment or runtime enforcement claims.

## Traffic mirroring (#894)

Filter rules reference only explicit managed filter `id` outputs. Known ingress or
egress direction, accept/reject action and integer rule number are required for
filter-owned rule navigation. Each rule retains its own identity and action.
Sessions reference managed ENI, filter and target `id` outputs independently;
session ordering does not prove packet delivery.

Targets require exactly one configured endpoint: ENI `id`, LB `arn` with known
`load_balancer_type=network`, or VPC endpoint `id` with known
`vpc_endpoint_type=GatewayLoadBalancer`. Competing unknown/sensitive variants
remain unresolved. The pinned Provider schema's three-way ExactlyOneOf constraint
is authoritative over the older two-variant note in its target documentation.

All links require unique explicit references, equal known outputs and compatible
current managed scope. Account compatibility is rechecked after enrichment.
Configuration objects keep logical identities; backing ENIs, LBs and subnets do
not contain targets or sessions. Computed EC2 primary-interface IDs, literals,
CIDRs and packet rules never fabricate resources or establish reachability.

## Lambda capacity providers (#948)

Capacity providers have independent spatial cards. The required singleton
`vpc_config` list block contains complete subnet and security-group ID sets;
`permissions_config` is a separate singleton list block containing the operator
role ARN. References accept managed subnet/security-group `id`, IAM role `arn`,
and KMS key `arn` outputs only. Object-shaped or repeated blocks, unknown or
sensitive values, incomplete sets and literal-only endpoints remain unresolved.

VPC placement requires all subnet and security-group members to reference one
current managed VPC explicitly, with matching known IDs and compatible scope.
Missing or empty network evidence retains the independent card without inferred
containment. Account evidence is rechecked after enrichment. IAM roles keep their
global scope; network and KMS references retain regional scope. No EC2 capacity,
ENIs or function-owned instances are synthesized from provider configuration.

### Lambda MicroVMs (#955)

MicroVMs have independent spatial cards; images retain independent logical identities. Image/base-image references require an exact current managed ARN and a known matching image version. Build/execution roles use exact IAM role ARNs. Connector references require complete lists of managed Lambda Core connector ARNs. They do not establish traffic direction, reachability or network containment; `INTERNET_EGRESS` and AWS-managed identifiers never create managed endpoints.

Singleton `logging.cloudwatch` blocks resolve exact log-group/stream names. A stream additionally requires explicit matching log-group provenance from both the MicroVM and the stream. Unknown/sensitive values, incomplete collections, incompatible scopes and lifecycle mismatches retain conservative fallback. Code URIs, endpoint URLs, payloads and runtime states remain opaque.

### IAM policy management (#1138)

Role policies, direct role attachments and instance profiles resolve explicit role name outputs. Permissions boundaries, direct/legacy policy attachments and deprecated role policy sets resolve exact managed policy ARNs. Global IAM references require same-account evidence; known matching accounts permit cross-region provider aliases. Trust and inline policy documents remain opaque.

Exclusive controllers retain independent change metadata and owner navigation, including empty policy sets (removal intent). Complete inline-policy name sets additionally require explicit matching owner references from each inline policy; these names never resolve against the managed-policy namespace. Multi-principal attachments require complete role/group/user name collections independently. Shared policies and profiles are not embedded, and IAM resources remain logical.

### IAM credential owners (#1139)

Access keys, login profiles, service-specific credentials, signing certificates and SSH keys resolve only their exact configured IAM user name (`user`, `user_name` or `username`). Verified current managed users provide owner navigation while each helper retains its own action and identity. Global references require same-account evidence; known matching accounts permit cross-region aliases. Credential outputs, encrypted values, fingerprints and key/certificate bodies are neither interpreted nor displayed.

### Identity Store membership (#1141)

Membership `group_id` and `member_id` resolve only explicit managed `group.group_id` and `user.user_id` outputs with matching known `identity_store_id`. Composite resource IDs are not principal identifiers. Each endpoint is verified independently with current lifecycle and compatible provider/account/region evidence. Memberships, users and groups retain independent identities and changes; no network containment or personal-data summaries are inferred.

### Identity Center assignments and configuration (#1142)

Application helpers resolve explicit managed `application_arn`/`arn` outputs. Permission-set helpers and account assignments resolve `permission_set.arn` only with a matching known `instance_arn`. Owner navigation preserves each helper action, including empty exclusive policy sets. Shared applications, permission sets and principals stay independent.

USER/GROUP principals use exact managed `user_id`/`group_id` outputs and require instance/store correlation: both the instance owner and principal must explicitly reference the same current `data.aws_ssoadmin_instances`, whose known `arns` and `identity_store_ids` lists each contain exactly one entry. This lookup supplies correlation evidence only; it is not a managed principal. Multiple-instance lists, unknown/sensitive values and literal-only correlation remain unproven. Application assignments obtain the instance through their verified application.

AWS_ACCOUNT targets resolve exact Organizations account IDs independently from principal and permission-set evidence. The represented target account is distinct from the administration provider account. Customer policy name/path applies in target accounts, AWS-managed policy ARNs do not identify in-plan customer policies, and authorized-target strings and policy JSON remain opaque. No configured assignment flag or reference asserts effective access.
### Global Accelerator custom routing (#918)

Custom accelerators have independent architecture cards. Listener and endpoint-group
ownership requires explicit matching custom resource ARNs; standard hierarchy types
are not interchangeable. Enabled flow logs use an exact managed S3 bucket name.
Endpoint groups require complete, element-correlated subnet ID references and a
known endpoint-group region matching each subnet. These references do not place
the accelerator inside a subnet or prove packet delivery.

Cross-account attachments retain independent identity. Shared endpoints require
explicit LB, instance or subnet ARN outputs, complete correlated endpoint entries,
and each entry's known region matching the endpoint. LB types must be application
or network. CIDR entries and principals do not identify managed resources; unknown
or competing CIDR configuration rejects an endpoint entry. All configured endpoints
must share the attachment owner's compatible account evidence. This does not assert
authorization for an accelerator in another account. EIP allocation IDs are not
ARN outputs and are not accepted as shared attachment endpoint identities.

The pinned AWS Provider 6.70.0 schemas use sets for endpoint/resource blocks and a
singleton list for accelerator attributes. The [AWS Resource contract](https://docs.aws.amazon.com/global-accelerator/latest/api/API_Resource.html)
requires ARNs for cross-account endpoint identities. Unknown, sensitive, literal,
ambiguous, historical and incompatible references remain unresolved. Configuration
payloads, generated IP/DNS outputs and runtime health are not interpreted.

### KMS configuration and replication (#1130)

Alias targets, ciphertext operations, grants and key policies resolve exact current
managed key outputs. Standard/replica keys expose id/key_id/arn; external primary
keys expose id/arn. Alias target_key_arn is computed and never creates ownership.
Aliases, grants and policies retain independent changes and unique key-owner
navigation. Ciphertext operations and all key variants remain independent.

Custom key stores use a known AWS_CLOUDHSM mode and exact cluster ID, or known
EXTERNAL_KEY_STORE/VPC_ENDPOINT_SERVICE modes and an exact endpoint service name.
Configured keys reference their store ID; XKS external key IDs and public proxy
URLs never identify managed AWS resources. Grant principals accept only explicit
IAM role/user ARNs; groups and service-name literals do not establish principals.

Replicas require the corresponding standard or external primary key ARN, known
primary multi_region=true, compatible accounts and known distinct regions. Other
key references require the same region; IAM principals retain global scope.
Unknown/sensitive/ambiguous, wrong-output and noncurrent evidence remains unresolved.
Key material, credentials, plaintext, ciphertext, contexts and policy bodies are
excluded from presentation and interpretation.

### Secrets Manager configuration (#1135)

Secrets resolve exact managed KMS key outputs. Replica blocks require complete
correlated key references, known distinct destination regions matching each key,
and compatible account evidence. Blocks never generate standalone secret nodes.
Policy helpers use secret ARN, tags use secret ID, and rotation helpers accept
secret ID/ARN/name with exact output equality. Unique owners retain independent
helper changes and provenance; shared keys and rotation targets remain separate.

Configured rotation Lambda/partner-role references require known enabled rotation
and a known absent competing mode field. This does not assert successful rotation,
permissions or health. Secret values, policy bodies, tag values and partner
metadata remain opaque. Unknown, sensitive, ambiguous, foreign or noncurrent
inputs stay unresolved. Existing secret-version identities are unchanged.

### S3 Files (#980)

File systems have independent spatial cards and resolve configured S3 bucket ARN,
IAM role ARN and KMS key ARN (the pinned provider uses an ARN-typed kms_key_id).
Access points, policies and synchronization helpers use exact S3 Files filesystem
IDs and retain separate changes/provenance. These types are not EFS aliases.

Mount targets remain independent spatial endpoints. Subnet and complete security
group references must prove one compatible managed VPC before the mount target
is placed in its subnet. File systems and backing buckets remain outside that
subnet. Computed network_interface_id/vpc_id and IP addresses do not establish
managed ownership. Literal, unknown, sensitive, incomplete and noncurrent evidence
falls back conservatively; policies, directory paths and synchronization payloads
are not interpreted or displayed.

### S3 Tables (#981)

Table buckets and tables have independent spatial cards. Namespaces resolve exact
bucket ARNs. A table's namespace reference requires matching known table_bucket_arn;
table-bucket references also accept the matching namespace's table_bucket_arn.
Table policies require the complete name/namespace/bucket identity. Unique helper
owners retain independent action/provenance. Warehouse locations, maintenance
settings and table metadata never generate S3 resources or child tables.

Encryption configuration uses whole-object expression references, with a known
aws:kms sse_algorithm and an explicit matching KMS key ARN. Replication helpers
resolve exact source ARNs, IAM role ARNs and a complete destination ARN set under
the single rule block, independent of configuration and planned set ordering.
Configured destinations may be in the same or another known region with
compatible same-account evidence; cross-account proof is deferred. This follows
[AWS's replication contract](https://docs.aws.amazon.com/AmazonS3/latest/userguide/s3-tables-replication-tables.html)
without asserting execution, permissions or health. Unknown/sensitive/incomplete,
noncurrent or foreign evidence stays unresolved; policy/metadata payloads remain
opaque.

### S3 Vectors (#982)

Vector buckets have independent spatial cards. Index owners resolve exactly
vector_bucket_name -> vector bucket.vector_bucket_name; policy owners use
vector_bucket_arn -> vector bucket.vector_bucket_arn. Generic arn/id/name outputs
are not aliases for these contracts, and index_arn is not a generic ARN output.
Helpers retain independent action, provenance and coverage.

Encryption configuration is a singleton list, unlike S3 Tables' object. An exact
KMS key ARN reference is accepted only with known aws:kms sse_type and compatible
regional/account evidence. Missing, ambiguous, unknown/sensitive and noncurrent
evidence stays unresolved. Dimensions, distance metrics, metadata configuration
and policy documents do not create vector resources or network ownership; arbitrary
payloads remain excluded from presentation.

### VPC Lattice services and targets (#790)

Services and service networks have independent spatial cards. A service network
is not an EC2 VPC container. Listeners navigate to a uniquely referenced service;
rules require their explicit service and listener to agree on that parent.
Listener IDs use listener_id or arn, never the composite Terraform id. Forward
target-group lists require complete positional ID/ARN references and exclude
fixed-response actions. Shared targets and target groups retain their identities.

Target attachments require a known group type: INSTANCE uses instance id, LAMBDA
uses function arn, and ALB uses the ARN of a known application load balancer. IP
values do not identify Terraform resources. Network target groups can reference
a configured VPC; VPC associations require complete security-group references
with the same explicitly proven VPC. Neither relationship relocates services.

Policy references accept ARN outputs, matching the pinned Provider validators.
Resource policies support service, service-network and resource-configuration
owners. Access logs require a known SERVICE/RESOURCE variant and an explicit
CloudWatch log-group, S3 bucket or Firehose delivery-stream ARN. Cross-account
service-network association and logging-owner references require ARN outputs
and independently known account/region bindings; scopes are never inherited.
Policies, listener predicates, DNS names, computed endpoints and runtime health
remain opaque. Unknown, sensitive, partial or noncurrent evidence stays unresolved.

### VPC Lattice resource gateways and configurations (#791)

Resource gateways have independent spatial cards. Placement requires explicit
VPC/subnet/security-group IDs, complete collections and common-VPC proof for
each subnet and security group. Known empty optional security groups are allowed;
computed defaults or unknown/partial memberships do not establish placement.

Known SINGLE, GROUP and ARN configurations reference gateway ID/ARN outputs.
CHILD configurations navigate only to an explicitly referenced GROUP id; their
computed inherited gateway does not add a configured link. Explicit domain
verification uses the verification resource id. Resource-configuration id/arn
identify that configuration, despite the pinned docs calling them gateway outputs.
Service-network associations retain independent configuration/network identities
and require ARN outputs plus independent account evidence across accounts.

The initial ARN-target endpoint contract remains deferred. IP/DNS values and TXT
verification outputs never synthesize application endpoints or Route53 records.
Configuration gateway references do not place the represented resource in a VPC
and domain verification references do not assert successful DNS verification.

### Direct Connect connections and virtual interfaces (#786)

Connections, LAGs and DX gateways have independent spatial cards. DX gateways
remain global; connection locations and BGP addresses do not establish VPC/subnet
containment. Dedicated connection associations and LAG migration use connection
ids. Private/public/transit VIFs reference exact connection or LAG ids; private
VIF gateway references require exactly one known DX/VPN gateway variant. BGP
peers reference exact VIF ids without embedding shared interface identities.

MACsec associations reference dedicated connection ids and explicitly configured
Secrets Manager ARNs. CAK, CKN and unmarked BGP authentication values stay opaque.
ARN and ID outputs are not interchangeable. All endpoints require explicit
current managed references, known values and compatible provider/account/region
evidence. Route prefixes, computed endpoints and runtime link health are ignored.
Hosted/cross-account contracts are handled separately by #787.
