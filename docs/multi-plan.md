# Multiple plans and state identity

Name each input explicitly with repeatable `--state ID=PATH` arguments:

```sh
planorama --state network=network/plan.json --state application=application/plan.json -o architecture.svg
```

IDs are case-sensitive, unique within an invocation, and contain 1–64 ASCII letters, digits, periods, underscores or hyphens. The first `=` separates the ID from the path, so paths can contain spaces or further `=` characters (quote the entire argument when necessary). Planorama does not infer a persistent backend identity from filenames, directories, Terraform workspaces or remote services. Choose consistent IDs across runs.

One named input may use `-` for stdin. Multiple stdin inputs and mixing named inputs with the existing positional input are errors. The existing `planorama plan.json` and `planorama -` forms keep their rendering and diagnostic behavior. `--address-format`, `--diagnostics`, and `-o` also work with named inputs. Every input is validated before output is written, and the output cannot overwrite any input file.

`PlanInput` pairs a validated `StateId` with JSON; `MultiPlan.states` owns a sorted map of independent `TerraformGraph` partitions. A resource identity is the tuple of state ID, Terraform address and local change identity (including deposed key). Graph indices, moves, checks, drift provenance, plan status and synthetic components stay within their state partition. No address rewriting, backend lookup, resource deduplication across states, or cross-state edge inference occurs.

SVG output places states in ID order. Each section has a visible state label and `data-state-id`; all descendants inherit that provenance. Terraform addresses and deposed metadata retain their original values. DOM IDs and fragment references (resource links, markers, icons and accessibility labels) receive an injective state-derived prefix, independent of input paths or order. Synthetic component IDs are local identities qualified by the enclosing state. Plan status and check summaries remain separate, rather than combining flags or results from different plans.

With `--diagnostics`, every diagnostic line carries `state="ID"`. Parse/read errors name their state. Attribute values and check messages remain excluded from SVG and diagnostics under the same rules as single-plan input. See [the multi-plan example](../examples/multi-plan.svg), generated from the two [regression fixtures](../tests/fixtures/multi).
