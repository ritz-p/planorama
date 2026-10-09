# Multiple plans and state identity

The #124 integration combines #125 (named inputs), #126 (output provenance),
#127 (explicit remote-state resolution) and #128 (architecture/SVG relationships).
Cross-state endpoints are independent of the evidence mechanism: the core stores
a `CrossStateProvenance` variant, initially `TerraformRemoteState`. Future explicit
mechanisms can supply their own evidence without changing state identity or
inventing remote-state addresses. No additional mechanism is currently supported.

Known limit: Terraform may serialize a full splat and a computed single-element
selection into the same bare collection reference. Without distinguishing expression
information, that output remains incomplete; Planorama does not infer all instances
from a bare collection or compare output values to recover missing syntax. Explicit
complete provenance still resolves multiple sources. Missing, ambiguous or incomplete
provenance produces diagnostics and no guessed cross-state relationship.

Resolved mappings appear as dashed cross-state dependency lines connecting actual resource cards. `data-source-state`, `data-target-state`, `data-remote-state` and `data-output` retain the full chain; source/target DOM references point into their own state sections. These relationships live separately from state-local graphs and never introduce containment. Source nodes removed by semantic lowering are not replaced with a guessed endpoint. Unmapped inputs retain independent-state output.

Use `--remote-state CONSUMER:DATA_ADDRESS=PRODUCER` to explicitly connect an exact `data.terraform_remote_state.NAME.outputs.OUTPUT` traversal to a loaded producer state. Repeat mappings as needed; conflicting producers remain ambiguous and emit no relationship. Root output provenance must be complete and nonempty. `--diagnostics` reports successful chains and unmapped, missing or incomplete sources without backend configuration or output values. This initial resolver handles direct exact traversals, including statically identified module instances; it does not guess backend identity or resolve dynamic output selections.

Name each input explicitly with repeatable `--state ID=PATH` arguments:

```sh
planorama --state network=network/plan.json --state application=application/plan.json -o architecture.svg
```

IDs are case-sensitive, unique within an invocation, and contain 1–64 ASCII letters, digits, periods, underscores or hyphens. The first `=` separates the ID from the path, so paths can contain spaces or further `=` characters (quote the entire argument when necessary). Planorama does not infer a persistent backend identity from filenames, directories, Terraform workspaces or remote services. Choose consistent IDs across runs.

One named input may use `-` for stdin. Multiple stdin inputs and mixing named inputs with the existing positional input are errors. The existing `planorama plan.json` and `planorama -` forms keep their rendering and diagnostic behavior. `--address-format`, `--diagnostics`, and `-o` also work with named inputs. Every input is validated before output is written, and the output cannot overwrite any input file.

`PlanInput` pairs a validated `StateId` with JSON; `MultiPlan.states` owns a sorted map of independent `TerraformPlan` partitions. A resource identity is the tuple of state ID, Terraform address and local change identity (including deposed key). Entity/reference indices, moves, checks, drift provenance and plan status stay within their input state partition. Semantic transformation creates state-local architecture graphs and synthetic components. No address rewriting, backend lookup, resource deduplication across states, or automatic cross-state edge inference occurs; explicit mappings resolve the output provenance described above.

SVG output places states in ID order. Each section has a visible state label and `data-state-id`; all descendants inherit that provenance. Terraform addresses and deposed metadata retain their original values. DOM IDs and fragment references (resource links, markers, icons and accessibility labels) receive an injective state-derived prefix, independent of input paths or order. Synthetic component IDs are local identities qualified by the enclosing state. Plan status and check summaries remain separate, rather than combining flags or results from different plans.

With `--diagnostics`, every diagnostic line carries `state="ID"`. Parse/read errors name their state. Attribute values and check messages remain excluded from SVG and diagnostics under the same rules as single-plan input. See [the multi-plan example](../examples/multi-plan.svg), generated from the two [regression fixtures](../tests/fixtures/multi).
