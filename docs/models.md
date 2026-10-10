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

Provider source (`ProviderIdentity`, such as `registry.terraform.io/hashicorp/aws`)
is separate from `provider_configuration`. The latter preserves Terraform's opaque
`provider_config_key` and the alias explicitly supplied in configuration metadata.
Default and aliased bindings retain their exact keys, including module bindings;
aliases are not guessed from key spelling. An unresolved key is retained with no
alias, while absent bindings remain `None`. Resource-level provider source keeps
precedence over configuration metadata, preserving provider classification.
Current configuration enriches only current objects. Deposed predecessors retain
their resource-level provider source or legacy fallback, with no configuration key
or alias inferred from the current resource at the same address.

Configuration keys are state-local: cross-state identity requires `(StateId, key)`.
Matching aliases in two states do not imply a shared account or region. Both the
Terraform facts and architecture cards retain these identities; provider expressions,
credentials and region/account values are neither stored nor inferred.

This separation intentionally preserves existing diagrams: snapshot tests cover
the boundary and regression tests retain provider resolution, reference evidence,
deposed/moved/import/removed/drift behavior and non-mutation of parsed input.

## Architecture identity and provenance

Cards and logical components now contain `ArchitectureEntity`, with an opaque
`ArchitectureId`, explicit Terraform/synthetic kind, and ordered, deduplicated
`TerraformEntityId` provenance. Terraform source identity includes the deposed
key. Synthetic identity uses a component kind and stable inference anchor, so
membership and display-label changes do not become identity changes. Both kinds
are available through `Graph::entities()`; synthetic components have no required
Terraform address of their own and do not acquire a spatial container role.

Identity is scoped by the containing state. Cross-state identity is the pair of
`StateId` and architecture/source ID. Existing Terraform card fields and component
member labels remain display projections; their native entity metadata is the
identity/provenance boundary. Layout ordering uses architecture IDs, and logical
panel bounds are keyed by those IDs independently of source addresses. Positional
edge indices remain an internal graph representation.

SVGs expose `data-architecture-id` and an `architecture-entities` JSON metadata
element containing the source addresses/deposed keys for every entity. Existing
Terraform address metadata and stable resource DOM IDs remain available. Synthetic
DOM IDs derive from architecture identity instead of panel position.
