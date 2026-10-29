# Architecture and dependency contracts

The pipeline is JSON → TerraformPlan → ArchitectureGraph → ViewGraph → Layout →
SVG, optionally wrapped in self-contained HTML. Native CLI and Wasm share parsing,
semantics, immutable selection and composition. The CLI alone owns files,
Terraform processes, destinations and detailed diagnostics. Public API/error and
memory contracts are in [browser](browser.md) and [engine](browser-engine.md).

## Boundaries

- `plan` retains Terraform facts and provider identity without importing rendering
  or AWS interpretation. Native saved-plan [provider evidence](provider-evidence.md)
  attaches verified accounts before semantic projection; JSON alone leaves them unknown.
- `provider` owns exact-type/attribute semantics. Neither layout nor rendering may
  infer relationships, ownership or scopes from names or proximity.
- `semantic` projects facts without mutating the plan; `model::architecture` is the
  facade consumed by view/layout/SVG, excluding ingestion contracts.
- `view` applies selection/context closure before presentation. Even unfiltered
  output passes through immutable projection. Relationships require surviving
  endpoints; optional Spatial projection must not change semantic evidence.
- `layout` consumes measured spatial geometry. Logical nodes keep identity but
  have zero-size bounds and no incident routed paths or obstacles.
- `svg` serializes shared measurements; `html` adds local interaction over the
  selected resource index. Public parser/graph/layout implementation types stay private.

## Measurement and identity

Measure wrapping, headings, labels, badges, reference rows and embedded children
before packing. Rendering must reuse those measurements rather than recomputing
wrapping. Reserve Unavailable prefixes and both native/qualified address forms.
Address wrapping preserves delimiters and quoted instance keys; forced splitting
uses UTF-8 boundaries. [Presentation contracts](resource-presentation.md) define
owner selection, fallback and exactly-once resource accounting.

Annotations are outside architecture layout. Header/legend/annotation height and
extra top clearance translate cards, paths and badges together. Named-state
endpoint positions and cross-state routes include that translation. Empty sections
reserve no height. A section frame ends at its occupied width, independently of
the width available for packing. [Viewer reflow](viewer.md#viewer-source-modules)
uses exported measurements and must keep this coordinate contract.

Logical grouping uses canonical provider source, exact resource type and mode;
alias, module, action and lifecycle do not merge resource identities. Group wrappers
are not resources. Links target the original row, including when its presentation
changes. Counts include each selected Terraform identity once. Unknown role strings
in SVG metadata are descriptive and must not break consumers.

Provider-declared presentation ownership and semantic membership/containment remain
distinct. The selected effective owner must drive card bounds, ports, routing ancestry,
masks, SVG metadata and HTML export consistently. Routes retain their specialized
ownership/context proof; generic children must not inherit it automatically.
[Reference rows](relationships.md#reference-rows-and-endpoints) do not create routed
edges merely because the viewer can navigate them.

## Optional outputs and verification

Coverage, semantic/layout diagnostics and timings are opt-in observations of the
selected result; they must not change geometry or retain arbitrary values.
The CLI applies destination alias protection to reports as well as diagram files.
Disabled timing spans perform no clock reads or reporting allocations.

Private-module tests live in tests/unit; integration tests exercise the CLI.
Fixtures have no shared mutable state or test-order dependency. Run the relevant
[Docker checks](../CONTRIBUTING.md#検証); expensive large routing comparisons retain
[separate coverage](ci.md#deferred-routing-regressions-253).
