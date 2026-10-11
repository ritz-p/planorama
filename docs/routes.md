# Route cards

Managed AWS `aws_route` resources render as action-colored child cards inside their
verified `aws_route_table`. Ownership requires a complete, unique `route_table_id`
ID reference and a known, nonempty planned value equal to the referenced table ID.
The comparison retains only a boolean proof. Names, module proximity, unknown
values, literal table IDs and dynamic selectors cannot establish ownership.
Unverified and deposed routes remain visible standalone cards with diagnostics.

Embedding is presentation metadata, not Terraform Containment. Each Route retains
its graph node, native address, stable ID, action, import/move/deposed information
and provenance. It is counted once and does not also enter the relationship index.
The table keeps its VPC containment and Subnet associations. Filtering a Route
retains its table even at focus depth zero or when the table is unchanged.

Next-hop resolution is independent of ownership. Complete unique ID references
also require known planned values matching the target ID. Supported attributes
are `gateway_id` (Internet/VPN Gateway), `nat_gateway_id`, `transit_gateway_id`,
`vpc_peering_connection_id`, `network_interface_id`, `egress_only_gateway_id`,
`carrier_gateway_id` and `local_gateway_id`, each checked against its AWS type.
VPC Endpoint subtype semantics remain unsupported. Missing, ambiguous, dynamic or
unsupported targets display a fallback and retain diagnostics.

When the verified next hop has a visible spatial card, a directed Association runs
from the Route child's own port to the target port. It retains the Route's action,
lifecycle metadata and resource provenance. The child shows `Next hop [N]`, matching
the destination marker, and omits the `Target:` row. It has no duplicate resource
entry in the relationship index, parent-to-target line or parent-to-child line.
Other incident references remain in metadata and child reference rows. Unknown
reference candidates are labeled References, not asserted next-hop targets.
Filtered, nonspatial and unresolved targets retain a target/status fallback without
a dangling spatial path. Cross-state paths
attach to the owning table border, retain the actual Route endpoint ID and expose
the attachment ID in `data-source-port` / `data-target-port`.

Destinations come only from validated configuration constants: IPv4/IPv6 CIDRs
or prefix-list IDs. Evaluated values are never displayed. Sensitive masks, unknown
values, invalid/ambiguous constants and historical delete/deposed configurations
use an unresolved fallback. No variable or expression evaluation is attempted.

Inline `aws_route_table.route` constants and structured entries render as separately
labeled entries using the table's action. They create no Terraform identities or
extra resource counts. Literal next-hop IDs can resolve to a unique known planned
resource ID of the supported AWS type; only the resource identity is retained.
Unproven inline expressions keep a fallback and diagnostics. Duplicate destinations
remain separate; inline entries sort deterministically. An unknown route collection
has an unresolved entry, while an empty table needs no Routes section.

Shared row measurement wraps complete child addresses and reserves their height
inside the table footprint before placement and routing. The final router uses
child bounds, protects the parent header and sibling entries, and reserves target
ports and marker corridors. Routes to the same next hop remain individual paths.
See [routes.svg](../examples/routes.svg)
and the [large example](../examples/terraform-large/diagram.svg).
