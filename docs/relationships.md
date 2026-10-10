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

Markers avoid cards, container boundaries, arrowheads and junctions. Dense paths
can use a marker in the right margin with a thin leader. Index height follows the
number of entries and wrapped lines; card placement and relationship routes stay
unchanged. Existing edge titles and provenance metadata remain available, and
attribute values are excluded.

Cards placed through indirect reference chains show `inferred placement` inside
the card. Direct parent references do not receive this note. The model determines
the distinction from containment provenance; the full evidence remains in SVG
relationship metadata. Card height reserves space for the note and module footer.
