# Relationship styles

Dependency lines are thin and solid. Lowered associations are dashed (7/4),
and inferred connections are thicker and dotted (2/4 with round caps). These
patterns remain distinct in grayscale. Containment uses nested boundaries;
an edge represented by nesting does not receive a redundant line.

Colors and matching arrowheads describe the Terraform change. Every edge retains
`data-edge-kind`, stable source/target IDs and a descriptive title. Cross-state dependencies use
blue dashed gutter routes and visible cross-state legend.

Visible paths with Terraform resource provenance carry compact `[N]` markers and
a matching **Relationship resources** index below the diagram and component
summaries. Full Terraform-native addresses are wrapped without truncation;
multiple provenance resources and deposed keys remain visible. Reference-only
inferred relationships receive no resource number. Numbering sorts by source
identity, then endpoints and relationship kind, independent of edge-vector order.
Filtering can renumber the visible subset; named states have separate indexes.

Markers are fixed immediately before the destination, aligned with the final
segment with 10 pixels of clearance for a fixed-size arrowhead. Targets with
numbered relationships reserve space between ports to keep the badges readable.
Routing reserves a straight terminal segment long enough for the complete badge
and arrowhead. Other paths, including bundled relationships, avoid this space.
Card and container spacing expands with the number of digits in the index.
Each badge uses the relationship's action color, stroke width and line pattern (solid for dependency,
dashed for association, dotted for connection). The corresponding number badge
in the relationship index uses the same border color and pattern. Index height follows the
number of entries and wrapped lines, accounting for wide Unicode characters.
Cross-state routes avoid number badges. Existing edge titles and provenance
metadata remain available, and attribute values are excluded.

Cards placed through indirect reference chains show `inferred placement` inside
the card. Direct parent references do not receive this note. The model determines
the distinction from containment provenance; the full evidence remains in SVG
relationship metadata. Card height reserves space for the note and module footer.
