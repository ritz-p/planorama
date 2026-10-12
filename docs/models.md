# Terraform facts and architecture projection

`plan::parse` returns `TerraformPlan`: a deterministic collection of
`TerraformEntity` facts, resolved `TerraformReference` source/consumer pairs,
attribute provenance, outputs, remote references, checks, status and diagnostics.
Entities retain native address, deposed key, previous address, provider, module,
mode, action and safe lifecycle metadata. Current and deposed objects at one
address remain distinct. Before/after values and sensitive import identities are
not stored. Collection/scalar identity proofs are boolean facts, not raw values.

Route entries retain validated, nonsensitive configuration destination constants
and safe target identities. `Node.routes` holds presentation ownership, inline
entries and projected content height; it does not create semantic Containment.

The parser has no dependency on architecture `Graph`, `Node`, `EdgeKind` or role
classification. Its `nodes`/`edges` fields are Terraform entities/references, not
rendering objects. A reference has only source and consumer; it does not claim
containment, connection or association semantics.

References retain static instance keys and module scopes. Dependency resolution
can expand unknown selections to candidate instances, while semantic rules require
their own completeness checks. Module inputs/outputs and serialized local definitions
are followed with cycle detection; absent local definitions cannot be reconstructed.
Nested block fields retain dotted paths, and repeated blocks require complete
provenance in every entry. Literal values are not traversed as expressions.
Attribute evidence also records whether every referenced instance key was explicit
through alias resolution. Target-group component ownership requires this proof;
it does not narrow existing dependency edges or change other semantic rules.
Target-group ownership also consumes a value-free ARN equality proof for every
hop, calculated from known planned strings and complete nested action values.
Alarm monitoring retains a boolean dimension/identifier equality proof; namespace,
metric and dimension values are checked transiently and never stored or rendered.

`semantic::transform` consumes this domain model. Its private `Input` projection
classifies provider/resource roles at the boundary, then applies containment,
connection, association, visibility and component rules. Diagnostics use the same
projection. Lifecycle metadata can evolve in the Terraform model independently;
the current card projection copies the safe metadata required for existing SVGs.
Layout/rendering consume architecture graphs only. Named plans keep separate
Terraform models under explicit `StateId`s.

`Node.placement` classifies spatial, logical and unresolved presentation separately
from role. Logical nodes retain their native identity, action and provenance.
`Node.references` retains direct attribute, verified inverse monitoring and graph-level reference evidence,
resolved source identities, completeness and diagnostics; it never expands a
resource dependency transitively. Module/input/output resolution follows the
parser's existing evidence. Graph-level references such as `depends_on` are
distinct from configured attribute usage. Evaluated attribute values are not retained.
In SVG `resource-references` metadata, remote targets include `state` alongside
their address and deposed key. Targets without `state` belong to the enclosing
state section, or the single input in unnamed diagrams.

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

Source resolution prefers `resource_changes.provider_name`, then
`planned_values` resource `provider_name`, then configuration `full_name` for
current objects. Two-part source names receive the `registry.terraform.io/` prefix.
Terraform's built-in provider is recognized only by the canonical source
`terraform.io/builtin/terraform`. `hashicorp/terraform`, `builtin/terraform` and
sources on other hosts do not identify that provider.

Only legacy inputs with no provider source or configuration binding infer AWS
from `aws_` types, or `InferredTerraform` from the exact types `terraform_data`
and `terraform_remote_state`. No other `terraform_` type is inferred. An explicit
foreign or empty source, or an unresolved configuration binding, blocks this fallback;
resource type spelling cannot override it. The fallback records its inferred
identity separately from an explicit source. Icon selection consumes this
identity without changing roles, relationships or lifecycle metadata.

Configuration keys are state-local: cross-state identity requires `(StateId, key)`.
Matching aliases in two states do not imply a shared account or region. Both the
Terraform facts and architecture cards retain these identities.

`provider_configuration.region` is an optional `RegionScope`, resolved only from
an AWS configuration's literal `expressions.region.constant_value`. Both the
configuration source and resource provider must identify HashiCorp AWS. A nonempty
string of lowercase ASCII letters, digits and hyphens is retained; references,
dynamic expressions, malformed values and missing settings remain unknown. Module
resources use their exact provider binding. Aliases, names, tags, ARNs, IDs,
environment variables and Terraform input variable values are never used to infer
a region. Credentials and unrelated provider expressions are not retained.
Region is provider deployment metadata, independent of state identity and resource
containment; it does not classify global AWS resources as regional.

## Architecture identity and provenance

Cards and logical components contain `ArchitectureEntity`, with an opaque
`ArchitectureId`, explicit Terraform/synthetic kind, and ordered, deduplicated
`TerraformEntityId` provenance. Terraform source identity includes the deposed
key. Synthetic identity uses a component kind and stable inference anchor, so
membership and display-label changes do not become identity changes. Both kinds
are available through `Graph::entities()`; synthetic components have no required
Terraform address of their own and do not acquire a spatial container role.

Identity is scoped by the containing state. Cross-state identity is the pair of
`StateId` and architecture/source ID. Existing Terraform card fields and component
member labels remain display projections; their native entity metadata is the
identity/provenance boundary. Layout uses structural ordering with identity
tie-breakers. SVG composition positions logical annotation panels independently
of architecture geometry, retaining those IDs. Positional
edge indices remain an internal graph representation.

SVGs expose `data-architecture-id` and an `architecture-entities` JSON metadata
element containing the source addresses/deposed keys for every entity. Existing
Terraform address metadata and stable resource DOM IDs remain available. Synthetic
DOM IDs derive from architecture identity instead of panel position.
