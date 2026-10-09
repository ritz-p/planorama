# Architecture and dependency contracts

The data pipeline is `CLI -> TerraformPlan -> ArchitectureGraph -> ViewGraph -> Layout -> SVG`.
These arrows describe processing, not Rust imports: a consumer imports its input
contracts from `model`, not the implementation of the preceding stage. The CLI
coordinates the stages and owns file I/O. Multi-state input follows the same stages
with named state identities and explicit cross-state relationships.

| Module | Owns | Must not do |
| --- | --- | --- |
| `plan` | Terraform JSON ingestion, native identity, changes, reference/output facts | Import layout, SVG, provider rules or view selection |
| `model` | Shared domain contracts, architecture identity and value-free provenance | Invoke parsing, inference or rendering in production |
| `semantic` | Complete architecture transformation and native relationship evidence | Filter the input to satisfy a requested view |
| `provider` | Static provider dispatch | Introduce a dynamic plugin system without a demonstrated need |
| `provider/aws` | AWS types, attributes, classification, containment, lowering, components, diagnostics and icon mapping | Change another provider's lookalike resource semantics |
| `view` | Immutable selection, neighbor expansion, ancestor/component context and endpoint retention | Parse Terraform or infer new architecture relationships |
| `layout` | Derived containment, placement, bounds, ports and routes | Match AWS resource types, inspect Terraform JSON or infer architecture semantics |
| `svg` | Self-contained drawing, text, legends, metadata and presentation | Create semantic relationships or inspect Terraform JSON |

## Contracts and visibility

`TerraformEntity` and `TerraformReference` preserve ingestion facts without roles
or architecture identity. `Node` is a classified architecture card; it is not the
ingestion entity. Synthetic components share `ArchitectureId` with card entities.
`ArchitectureRelationship` uses architecture IDs and multiple resource/reference
provenance records, including an explicit inferred flag. `Graph.edges` is the
indexed geometry adapter; layout need not understand relationship evidence.

The semantic stage finalizes native relationships after provider transformations.
Projection retains both the indexed adapter and native relationships only when
their required endpoints survive. Source architecture data remains immutable.
`ViewGraph` and `ViewArchitecture` have private constructors and no mutable deref.
The CLI passes the resulting view to layout/rendering, including the unfiltered case.

The model may retain value-free Terraform identity and lifecycle metadata as
architecture provenance. Terraform JSON traversal and attribute-value analysis
stop at ingestion; downstream rendering formats already-decided metadata. Provider
diagnostics reuse the semantic input and the same rules as transformation.

Implementation submodules remain private. Entry points are crate-visible rather
than public library APIs. Unit tests are child modules (often loaded from `tests/unit`
using `#[path]`) so they can test private code without widening production visibility.
Test-only ingestion helpers are gated by `#[cfg(test)]`; they are not reverse
production dependencies.

## Verification and further work

Existing tests cover ingestion immutability, provider identity, synthetic identity,
native relationship evidence, projection immutability and ancestor retention,
deterministic routing, SVG escaping and captured Terraform input. Run them and
Clippy through the Docker development service. Refactors must preserve generated
examples unless an intentional presentation change is documented.

Layout implementation cleanup remains in #82–#86. Geometry features such as
side-aware attachment belong in #81, not in Terraform or AWS semantics. Adding a
provider extends `provider` dispatch and a provider-owned module; it does not add
resource-type tables to generic semantics, layout or SVG.
