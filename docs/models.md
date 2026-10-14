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
Alarm monitoring retains a boolean dimension/identifier equality proof; Auto Scaling policy ownership retains a boolean scalar-name equality proof. Namespace, metric, dimension and resource-name values are checked transiently and never stored or rendered.

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

`TerraformEntity.has_region_override` records only the presence of a resource's
configured `region` expression. AWS regional scope resolution treats that unsupported
override as unknown without changing `provider_configuration.region` or retaining
the expression's value. `ArchitectureEntity.scope` remains independent of logical
placement: account and region are independent optional dimensions, and `global`
explicitly distinguishes AWS Global from an unknown region. The shared `view::scope` label helper
consumes this evidence; known non-AWS providers without scope use `Region: not
applicable`, and unknown provider identities use `Region: unknown`. Labels never
participate in architecture, provider-configuration or state identity.

`provider_configuration.account` retains only a validated `AwsAccountScope`:
exactly 12 ASCII digits from literal IAM role ARNs in the exact AWS provider's
`expressions.assume_role[].role_arn.constant_value` or
`expressions.assume_role_with_web_identity[].role_arn.constant_value`.
Only the commercial `aws` partition is currently supported. Dynamic, malformed,
missing or conflicting evidence remains unknown; conflicts produce a
`ConflictingAwsAccount` diagnostic without exposing the role or provider values.
All supplied role blocks must resolve and agree, so cross-account role chains
are deliberately unresolved. This is conservative even when the final role in
a chain would select a target account.

The JSON shape is verified against Terraform 1.9.8's
[provider serializer](https://github.com/hashicorp/terraform/blob/v1.9.8/internal/command/jsonconfig/config.go)
and [nested-expression serializer](https://github.com/hashicorp/terraform/blob/v1.9.8/internal/command/jsonconfig/expression.go),
with AWS 5.100.0's [list-block schema](https://github.com/hashicorp/terraform-provider-aws/blob/v5.100.0/internal/provider/provider.go).
Unsupported JSON representations remain unknown rather than being interpreted
as evaluated provider values.

State, provider binding, AWS account and region are separate identities. One state
can manage several accounts; several bindings can target one account; equal
account IDs across states never merge resource identities. Aliases, profiles,
allowed/forbidden account lists, credentials, environment, resource ARNs/IDs and
caller-identity data are not account evidence. Role ARNs, session names, external
IDs and credentials are discarded. Known account IDs are deployment metadata
intended for display in generated SVG/HTML, so those artifacts can expose them.

Layout groups matching account scopes within each state, then region or explicit
Global scope within that account. These frames are deployment backgrounds, not
architecture entities or containment edges; filtering removes empty frames.
Unknown account retains known region presentation. A subtree with conflicting
member scopes remains outside frames without changing its containment.

## Architecture identity and provenance

Resource nodes and logical components contain `ArchitectureEntity`, with an opaque
`ArchitectureId`, explicit Terraform/synthetic kind, and ordered, deduplicated
`TerraformEntityId` provenance. Terraform source identity includes the deposed
key. Logical-resource display groups do not create architecture entities or combine
their members' identities, actions or relationships. Synthetic identity uses a
component kind and stable inference anchor, so membership and display-label
changes do not become identity changes. Both kinds
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
