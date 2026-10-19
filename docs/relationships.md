# Relationship styles

## Legend and meaning

The neutral **Relationship lines** card explains relationship colors, line
patterns, attachment circles, directions and numbered markers separately from the
resource action legend and the numbered **Relationship Resources** index.
Use `--hide-relationship-legend` to omit line-kind, direction, containment and
endpoint/number explanations, together with the cross-state header explanation.
It works for SVG and self-contained HTML, including named states. Resource action
colors remain, and relationship action colors retain a smaller **Relationship
actions** card. Measurement reserves only that card's actual height; subsequent
annotations and state sections move up. Actual lines, arrows, attachment points,
numbers, links and **Relationship Resources** entries remain unchanged.

```sh
planorama plan.json --hide-relationship-legend -o compact.svg
planorama plan.json --format html --hide-relationship-legend --coverage-report coverage.md -o compact.html
```

Compare the maintained [compact example](../examples/relationship-legend-hidden.svg)
with its [full legend version](../examples/routes.svg). Regenerate both using
`docker compose exec -T dev sh scripts/update-examples.sh`. The option combines
with either icon mode and does not change graph analysis or local path routing.

By default, the full card remains visible.
Action colors and their explanation appear above a grid of relationship kinds.
The card is capped at 960px, matching the maintained viewer example. The grid uses
at most two columns according to the available width, reading left
to right before continuing on the next row: Dependency, Association, Directed
Association, Connection, Containment. Subscription, Event source and Event target are appended when present in
the view. Each cell keeps its complete sample and wrapped explanation.
On narrow diagrams, a single column puts the sample above
the text; each row reserves the height of its tallest cell.
Multiple columns require at least 398px per cell with a 24px gap; individual
cells stop widening at 640px.

Single-plan output reserves the measured legend height before checks and annotations.
Named-state SVG and HTML show one shared legend above all state sections, using
the widest state's content width up to the same cap, excluding the cross-state routing gutter.
Its action colors come from all displayed states; if none has a non-containment
relationship, no legend or legend height is added. State-local checks, annotations
and Relationship Resources indexes retain their own identities and numbering.
State positions and cross-state endpoints include the shared legend height once.

Line color describes the relationship's own Terraform action, independently of
its endpoint resources. A relationship without an independent change
(`Edge.change == None`) uses neutral gray (`#94a3b8`) and the label
`no independent change`. This is distinct from an explicitly unchanged
relationship (`EdgeChange` carrying `Action::Unchanged`), which uses
the unchanged action color. Creating, updating or deleting an endpoint does not
transfer its action color to a relationship without its own change.

Line patterns and arrowheads describe relationship kind and direction:

| Kind | Sample | Meaning |
| --- | --- | --- |
| Dependency | Thin solid line, hollow source circle and target arrow | Referenced resource → dependent resource; a resolved reference, not Terraform's exact execution order. |
| Association | Dashed line, hollow circles and matching `[N]` badges at both ends, without arrowheads | An explicit association between resources; the sample's number identifies its Terraform resource provenance. |
| Directed Association | Dashed line, hollow circles and matching `[N]` badges at both ends, with a target arrow | A directed association, including a Route child → next hop; the arrow gives direction and the two badges identify one relationship. |
| Connection | Thicker circular dots and hollow endpoint circles, without arrowheads | An inferred architectural connection; it does not guarantee network reachability. |
| Containment | Nested boxes | Parent/child membership. An edge represented by nesting receives no redundant line. |
| Subscription | Dashed directed line with matching endpoint numbers | Terraform-proven Topic → endpoint subscription; delivery and access policies are not verified. |
| Event source | Dashed directed line with matching endpoint numbers | Terraform-proven SQS/Kinesis → Lambda event-consumer mapping; active state and processing success are not verified. |
| Event target | Dashed directed line with matching endpoint numbers | Terraform-proven Rule → SQS/Lambda target configuration; delivery, permissions and reachability are not verified. |

Hollow circles identify card attachment points; filled dots within a path identify
bundle junctions. Arrowheads identify direction. A `[N]` marker points to one
Relationship Resources entry and its Terraform provenance; it is not a direction,
resource count or separate relationship at each end. Reference-only relationships
remain unnumbered, so the numbered Association samples do not imply every path
receives a number.

Dependency paths use `stroke-width="1.5"`; Association paths use
`stroke-width="2"` and `stroke-dasharray="7 4"`.
Connection paths use `stroke-width="2.5"`, `stroke-dasharray="0 3.5"` and round
caps. The dots repeat every 3.5 SVG units, leaving a 1-unit gap between their
painted edges. Paths, legend samples and relationship-number badge outlines share
these patterns, which remain distinct in grayscale. Connection titles use `↔`.
Route child cards use presentation nesting; their safe ownership and entry details
are in `route-presentations`, and original reference evidence remains in metadata.
See [Route cards](routes.md) for next-hop evidence and direction.

RDS Cluster Instances, EFS Access Points, KMS Aliases and Secret Versions use
presentation nesting, recorded in `member-presentations`. Their original
dependencies remain in relationship metadata and appear as links inside each
member card; display nesting does not add
VPC/Subnet containment. `data-embedded-member` and `data-member-owner` identify the
displayed member and owner without duplicating either resource's DOM identity.

Cross-state dependencies retain blue dashed gutter routes and their own visible
cross-state legend. Blue identifies the cross-state relationship, independently
of state-local relationship action colors and producer output-action metadata.
State-local edges retain `data-edge-kind`, stable source/target IDs and a
descriptive title.

## Reference rows and endpoints

Spatial cards show a Uses section for verified references to top logical-resource
rows, supported service references and ordinary references to CloudWatch Log Group
cards. Links target the individual resource, not its group card. Known attributes
use Cluster, Task definition, Instance profile, AMI, Launch template,
Role, Execution role or Task role. Verified [AWS service references](aws.md) use
their service-specific labels, such as File system; other references use References.
[Firehose references](aws.md#amazon-data-firehose) distinguish Kinesis source,
S3 delivery destination and S3 backup destination. Their dependency arrows retain
Terraform reference semantics; these labels do not add runtime-delivery edges.
[IAM membership and direct policy references](aws.md#iam-users-and-groups) retain
each membership/attachment resource and its action; they do not evaluate inherited
or effective permissions.
[IAM identity provider references](aws.md#iam-identity-providers) are configuration
dependencies only; trust-policy JSON and matching issuer URLs do not create edges.
Graph-only references such as
`depends_on` use Dependency, not an attribute-specific usage label. Logical-resource
rows show their own direct references, including references to spatial resources.
Incomplete references show unresolved without asserting candidate targets.
Filtered targets show Unavailable without a dangling link. All rows and full
Terraform addresses are visible, including module/instance keys and remote state
qualifiers. Long labels wrap without ellipsis; card widths and heights grow before
packing and routing. The same metrics apply to logical-resource rows, whose group
cards reserve their complete reference content before each action underline and
the next resource. Spatial cards reserve space above the footer and placement
note. Accessible titles supplement the visible text.

CloudWatch alarms use Monitors and inverse Monitored by rows only with the
[verified monitoring evidence](aws.md) required by the AWS projection. These
rows never create network connections or additional dependency edges.

Auto Scaling Policies use `Auto Scaling group` and inverse `Scaling policy` rows
under the [AWS ownership proof](aws.md#resource-roles-and-data). These are
provider-specific reference rows, not additional graph edges.

Small white circles mark final card ports: the source of a Dependency and both
ends of Association/Connection paths. Containment has no endpoint circles.
Their solid outlines use the relationship action color. Circles render above
cards, with directed target arrowheads above the circle and numbered badges above
both. Internal filled junction dots remain distinct from endpoint circles.
Opposite-role endpoints on the same card retain distinct final coordinates,
including mixed line kinds and bundle/refinement paths. Intentional same-role
bundle terminals may still share a circle position.

## Relationship resource index

Visible paths with Terraform resource provenance carry compact `[N]` markers and
a matching **Relationship Resources** numbered list above the architecture diagram.
Embedded Routes appear in that list and show the matching number on their child
cards. Index entries do not add resource identities or increase resource counts.
Each number is a native SVG fragment link to its entry, including both endpoint
badges and Route `Next hop [N]` labels. The entry heading preserves every provenance
resource's complete address, action, move annotation and deposed identity. An
indented list shows the verified endpoint pair with kind and direction: `Connects`
for undirected associations and `From → To` for directed relationships. Endpoints
link to their rendered cards or rows. These labels remain visible in image embeds;
navigation requires an interactive SVG surface. No Terraform values are added.
Each provenance resource has its own `Resource scope` line, independently of the
scope appended to each linked endpoint. Region, AWS Global, unknown and not
applicable use the common scope labels; known accounts are included. The
relationship itself never inherits an endpoint's scope. Missing endpoints remain
unavailable text without a scope claim or link. Multiple provenance sources keep
their own scope evidence and measured lines. See the synthetic
[cross-region example](../examples/relationship-scopes.svg); its JSON-only input
leaves accounts unknown. It illustrates
scope presentation, not deployability or network reachability.
Entry targets derive from provenance and endpoint identities, independently of
the visible number, and named states namespace all links and path memberships.
The hierarchy and all wrapped lines are measured before packing cards.
Each card contains up to 5 complete entries: 1–5, 6–10, and so on. Cards continue
to the right with a 20px gap, wrapping only when the next complete card would exceed
the final available width. There is no 20-entry break or maximum column count;
an exact fit stays on the current row. The surrounding section frame ends after
the widest occupied row plus padding, without changing packing or canvas width.
Card space excludes the 40px outer margin
and 16px section padding on each side, totaling 112px. Named-state diagrams use
the final shared content width, including any width required by a wider state or
annotation, while reserving the cross-state routing gutter separately. A multi-source
entry stays in one card regardless of its wrapped line count. Full Terraform-native
addresses are wrapped without truncation. Each moved provenance source shows its
own `Moved from: ...` annotation alongside its current address and optional deposed
key. Multi-source entries keep each previous address associated with its source,
and their measured height includes every wrapped line. Reference-only
inferred relationships receive no resource number. Numbering sorts by source
identity, then endpoints and relationship kind, independent of edge-vector order.
Graph filtering can renumber the visible subset; named states have separate indexes.
Routing and rendering share these entries. An absent path in an existing layout
does not renumber the remaining entries or change their marker dimensions.

Numbered Associations, Subscriptions, Event source mappings and Event targets display the same standalone number near both endpoints,
including directed Associations from embedded Route cards, with one
relationship-index entry. Other undirected numbered relationships use the same
two-end rule. Directed Dependencies display their number before the destination
only. A Route child's inline `Next hop [N]` text supplements its standalone source
badge; both badges and the child text refer to the same index entry.
Markers align with the terminal segment, 10 pixels from the endpoint to leave
room for directed arrowheads. Endpoints with
numbered relationships reserve extra space only around their numbered ports.
Ordinary left/right ports reserve at least 16 pixels; numbered ports reserve
24 pixels on left/right sides (18px badge height plus 6px). Top/bottom slots use
each badge's actual width plus 6px. Card height follows the denser side rather than
the combined count. Straight approaches reserve the 10px gap, the badge's width
on left/right or 18px height on top/bottom, and 8px of turning clearance. Other
paths, including bundled relationships, avoid this space. Packing retains
conservative spacing for the largest number; individual terminals do not inherit
unrelated labels' widths.
Each badge uses the relationship's action color, stroke width and line pattern (solid for dependency,
dashed for association, dotted for connection). The corresponding number badge
in the index uses the same border color and pattern. Card heights follow actual
wrapped lines and source counts, accounting for wide Unicode characters; each row
reserves its tallest card's height. An empty index adds no cards or height. The
[21-entry example](../examples/relationship-index.svg) demonstrates 5/5/5/5/1 grouping.
The [40-entry wide example](../examples/relationship-index-wide.svg) uses five linked
`terraform_data` stages to create a wider architecture and eight index cards.
Cards wrap according to their measured content and the available width.
Both are synthetic rendering fixtures with repeated associations,
not deployable Terraform configurations. Their SVGs are maintained by
[local or manual CI generation](ci.md) from the [21-entry](../tests/fixtures/relationship-index-plan.json)
and [40-entry](../tests/fixtures/relationship-index-wide-plan.json) inputs.
Cross-state routes avoid number badges. Existing edge titles and provenance
metadata remain available, and attribute values are excluded.

Cards placed through indirect reference chains show `inferred placement` inside
the card. Direct parent references do not receive this note. The model determines
the distinction from containment provenance; the full evidence remains in SVG
relationship metadata. Card height reserves space for the note and module footer.
