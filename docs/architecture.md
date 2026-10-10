# Architecture and dependency contracts

The processing pipeline is `CLI → TerraformPlan → ArchitectureGraph → ViewGraph → Layout → SVG`.
The CLI owns file I/O and coordinates each stage. Named-state inputs preserve the
same boundaries and carry explicit cross-state relationships.

| Module | Responsibility |
| --- | --- |
| `plan` | Terraform JSON ingestion, native identity, changes, reference/output facts |
| `model` | Shared domain contracts, architecture identity and value-free provenance |
| `semantic` | Architecture transformation and final relationship evidence |
| `provider/aws` | AWS classification, containment, connections, lowering, components, diagnostics and icons |
| `view` | Immutable selection, neighbor expansion and required context |
| `layout` | Containment hierarchy, placement, bounds, ports and routes |
| `svg` | Drawing, text, legends and metadata |

Ingestion does not import provider rules, layout or SVG. Layout, view and SVG use
the `model::architecture` facade, which excludes Terraform ingestion contracts.
AWS resource-type and attribute rules belong in `provider/aws`; rendering and
layout do not infer semantic relationships.

The semantic input projects Terraform facts into classified cards without mutating
the plan. AWS transformation applies containment, SG connections, association
lowering, data visibility and logical components. Native relationships are finalized
after these transformations. See [models](models.md) and [AWS rules](aws.md).

`ArchitectureRelationship` uses stable architecture IDs and resource/reference
provenance. `Graph.edges` is the indexed adapter used by geometry. Layout uses the
model's derived-containment predicate to reserve annotation space.

View projection retains relationships only when their required endpoints survive.
`ViewGraph` and `ViewArchitecture` have private constructors and no mutable deref.
Even unfiltered CLI output passes through projection.

Implementation submodules remain private; entry points are crate-visible.
Unit tests can access private code through child modules loaded from `tests/unit`.
Integration tests invoke the CLI. Run the Docker checks in the [README](../README.md).
