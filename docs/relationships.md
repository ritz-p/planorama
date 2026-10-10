# Relationship styles

Dependency lines are thin and solid. Lowered associations are dashed (7/4),
and inferred connections are thicker and dotted (2/4 with round caps). These
patterns remain distinct in grayscale. Containment uses nested boundaries;
an edge represented by nesting does not receive a redundant line.

Action colors and matching arrowheads remain unchanged: line patterns describe
the relationship, while colors describe the Terraform change. Every edge retains
`data-edge-kind`, stable source/target IDs and a descriptive title. The document
description explains the patterns. Cross-state dependencies retain their separate
blue dashed gutter routes and visible cross-state legend.

Cards placed through indirect reference chains show `inferred placement` inside
the card. Direct parent references do not receive this note. The model determines
the distinction from containment provenance; the full evidence remains in SVG
relationship metadata. Card height reserves space for the note and module footer.
