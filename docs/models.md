# Terraform facts and architecture projection

`plan::parse` returns `TerraformPlan`: a deterministic, value-free collection of
`TerraformEntity` facts, resolved `TerraformReference` source/consumer pairs,
attribute provenance, outputs, remote references, checks, status and diagnostics.
Entities retain native address, deposed key, previous address, provider, module,
mode, action and safe lifecycle metadata. Current and deposed objects at one
address remain distinct. Before/after values and sensitive import identities are
not stored. Collection/scalar identity proofs are boolean facts, not raw values.

The parser has no dependency on architecture `Graph`, `Node`, `EdgeKind` or role
classification. Its `nodes`/`edges` fields are Terraform entities/references, not
rendering objects. A reference has only source and consumer; it does not claim
containment, connection or association semantics.

`semantic::transform` consumes this domain model. Its private `Input` projection
classifies provider/resource roles at the boundary, then applies containment,
connection, association, visibility and component rules. Diagnostics use the same
projection. Lifecycle metadata can evolve in the Terraform model independently;
the current card projection copies the safe metadata required for existing SVGs.
Layout/rendering consume architecture graphs only. Named plans keep separate
Terraform models under explicit `StateId`s.

This separation intentionally preserves existing diagrams: snapshot tests cover
the boundary and regression tests retain provider resolution, reference evidence,
deposed/moved/import/removed/drift behavior and non-mutation of parsed input.
