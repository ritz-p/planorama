# Relationship styles

Spatial cards show a Uses section for verified references to top logical-resource
cards. Known attributes use Cluster, Task definition, Instance profile, AMI,
Role, Execution role or Task role;
other verified references use References. Graph-only references such as
`depends_on` use Dependency, not an attribute-specific usage label. Logical cards
show their own direct references, including references to spatial resources.
Incomplete references show unresolved without asserting candidate targets.
Filtered targets show Unavailable without a dangling link. All rows and full
Terraform addresses are visible, including module/instance keys and remote state
qualifiers. Long labels wrap without ellipsis; card widths and heights grow before
packing and routing. The same metrics apply to logical cards and reserve space
above the footer and placement note. Accessible titles supplement the visible text.

CloudWatch alarms use Monitors and inverse Monitored by rows only with the
[verified monitoring evidence](aws.md) required by the AWS projection. These
rows never create network connections or additional dependency edges.

Dependency lines are thin and solid. Ordinary associations are undirected and
dashed (7/4), with no arrowheads. Route next hops use directed Associations from
[child-card ports](routes.md).
Inferred connections are undirected, without arrowheads, and use circular dots
(`stroke-width="2.5"`, `stroke-dasharray="0 3.5"`, round caps). Dots repeat every
3.5 SVG units, leaving a 1-unit gap between their painted edges. The same pattern
is used for paths, legend samples, and relationship-number badge outlines.
Connection titles use `↔`. These
patterns remain distinct in grayscale. Containment uses nested boundaries;
an edge represented by nesting does not receive a redundant line.
Route child cards use presentation nesting; their safe ownership and entry details
are in `route-presentations`, and original reference evidence remains in metadata.

Small white circles mark final card ports: the source of a Dependency and both
ends of Association/Connection paths. Containment has no endpoint circles.
Their solid outlines use the relationship action color. Circles render above
cards, with directed target arrowheads above the circle and numbered badges above
both. Internal filled junction dots remain distinct from endpoint circles.
Opposite-role endpoints on the same card retain distinct final coordinates,
including mixed line kinds and bundle/refinement paths. Intentional same-role
bundle terminals may still share a circle position.

Colors and directed arrowheads describe the Terraform change. Every edge retains
`data-edge-kind`, stable source/target IDs and a descriptive title. Cross-state dependencies use
blue dashed gutter routes and visible cross-state legend.

Visible paths with Terraform resource provenance carry compact `[N]` markers and
a matching **Relationship resources** numbered list above the architecture diagram.
Embedded Routes appear in that list and show the matching number on their child
cards. Index entries do not add resource identities or increase resource counts.
Each card contains up to 10 complete entries: 1–10, 11–20, and so on. Cards continue
to the right, wrapping only when the available width requires it. A multi-source
entry stays in one card regardless of its wrapped line count. Full Terraform-native
addresses are wrapped without truncation;
multiple provenance resources and deposed keys remain visible. Reference-only
inferred relationships receive no resource number. Numbering sorts by source
identity, then endpoints and relationship kind, independent of edge-vector order.
Graph filtering can renumber the visible subset; named states have separate indexes.
Routing and rendering share these entries. An absent path in an existing layout
does not renumber the remaining entries or change their marker dimensions.

Undirected relationships display the same number near both endpoints, with one
relationship-index entry. Directed relationships display it before the destination.
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
[21-entry example](../examples/relationship-index.svg) is a synthetic rendering
fixture demonstrating 10/10/1 grouping, not a deployable Terraform configuration.
Cross-state routes avoid number badges. Existing edge titles and provenance
metadata remain available, and attribute values are excluded.

Cards placed through indirect reference chains show `inferred placement` inside
the card. Direct parent references do not receive this note. The model determines
the distinction from containment provenance; the full evidence remains in SVG
relationship metadata. Card height reserves space for the note and module footer.
