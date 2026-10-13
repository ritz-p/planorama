# Multiple plans and state identity

Named states keep their own Terraform identities. Explicit remote-state mappings
can connect resources across states using `TerraformRemoteState` provenance.

Cross-state relationships also remain in `cross-state-relationships` metadata.
Edges incident on top logical-resource rows are excluded from spatial routing;
their state-qualified provenance remains available.
Consumer cards and logical-resource rows link to the actual state-qualified
source cards or rows.
Remote-output evidence remains distinct from a directly configured attribute;
equal addresses in separate states never share a DOM target.
Logical-resource grouping is local to each state. Equal provider/type/mode keys
in different states do not merge their groups or member identities.

Output references must be complete; bare collections and ambiguous selectors do
not establish cross-state relationships. See [output resolution](outputs.md).

Embedded Route cards keep state-qualified identities and reference links.
Cross-state paths involving them attach to their owning Route Table border;
`data-source-port` and `data-target-port` identify the spatial attachment cards.

Root `output_changes` actions are matched to configured outputs by name within each state and retained separately from provenance. Resolved cross-state paths expose the producer output action through `data-output-action` (including create, update and delete), without changing resource colors or provenance resolution. Missing changes omit the attribute; unrecognized actions use `other`, and unmatched entries create no output or relationship. Output values, including `before`, `after` and sensitive values, are never retained or rendered.

Resolved mappings between spatial cards appear as blue dashed cross-state dependency lines with one explanation in the document header. Their blue color identifies cross-state provenance and does not follow endpoint or producer output actions. These paths expose `data-source-state`, `data-target-state`, `data-remote-state` and `data-output`; source/target DOM references point into their own state sections. Mappings with a logical endpoint retain the chain in `cross-state-relationships` metadata without a path or path DOM-reference attributes. These relationships live separately from state-local graphs and never introduce containment. Source nodes removed by semantic lowering are not replaced with a guessed endpoint. Unmapped inputs retain independent-state output.

One shared **Relationship lines** legend precedes the state sections in SVG and
HTML. It includes relationship action colors from every displayed state, even
when the first state has no relationships. No qualifying relationships means no
legend or reserved height. Resource/action summaries, checks, plan status,
annotations and **Relationship Resources** indexes stay within their states;
equal visible relationship numbers in different states refer to independent entries.

Use `--remote-state CONSUMER:DATA_ADDRESS=PRODUCER` to explicitly connect an exact `data.terraform_remote_state.NAME.outputs.OUTPUT` traversal to a loaded producer state. Repeat mappings as needed; conflicting producers remain ambiguous and emit no relationship. Root output provenance must be complete and nonempty. `--diagnostics` reports successful chains and unmapped, missing or incomplete sources without backend configuration or output values. The resolver handles direct exact traversals, including statically identified module instances; it does not guess backend identity or resolve dynamic output selections.

Name each input explicitly with repeatable `--state ID=PATH` arguments:

```sh
planorama --state network=network/plan.json --state application=application/plan.json -o architecture.svg
```

Add `--format html -o architecture.html` for a [local searchable viewer](viewer.md).
Search results include state identity, and selection follows the existing
state-qualified target IDs and cross-state path directions.

IDs are case-sensitive, unique within an invocation, and contain 1–64 ASCII letters, digits, periods, underscores or hyphens. The first `=` separates the ID from the path, so paths can contain spaces or further `=` characters (quote the entire argument when necessary). Planorama does not infer a persistent backend identity from filenames, directories, Terraform workspaces or remote services. Choose consistent IDs across runs.

One named input may use `-` for stdin. Multiple stdin inputs and mixing named inputs with the existing positional input are errors. The existing `planorama plan.json` and `planorama -` forms keep their rendering and diagnostic behavior. `--address-format`, `--diagnostics`, and `-o` also work with named inputs. Every input is validated before output is written, and the output cannot overwrite any input file.

`PlanInput` pairs a validated `StateId` with JSON; `MultiPlan.states` owns a sorted map of independent `TerraformPlan` partitions. A resource identity is the tuple of state ID, Terraform address and local change identity (including deposed key). Entity/reference indices, moves, checks, drift provenance and plan status stay within their input state partition. Semantic transformation creates state-local architecture graphs and synthetic components. No address rewriting, backend lookup, resource deduplication across states, or automatic cross-state edge inference occurs; explicit mappings resolve the output provenance described above.

SVG output places states in ID order. Each section has a visible state label and `data-state-id`; all descendants inherit that provenance. Terraform addresses and deposed metadata retain their original values. DOM IDs and fragment references (resource links, markers, icons and accessibility labels) receive an injective state-derived prefix, independent of input paths or order. Synthetic component IDs are local identities qualified by the enclosing state. Plan status and check summaries remain separate, rather than combining flags or results from different plans.

State sections share the final content width required by the widest architecture
or annotation. Each state's annotations reflow within that width, so Relationship
Resources can use additional horizontal space before wrapping. State heights and
cross-state endpoint offsets follow the reflowed annotations. The cross-state
routing gutter is reserved outside the shared content width and is not available
for index cards.

Resolved deployment groups form background bases within each state's architecture
area: State → AWS Account → Region → VPC/Subnet containers and resource cards.
Account is optional and independent of State: one state can manage multiple
accounts, and equal account IDs in separate states never merge panels or resource
identities. Global resources share their proven account parent without a region;
unknown accounts preserve known Region backgrounds. Generated SVG/HTML explicitly
display known account IDs. A single spatial
resource qualifies, even in a single-region state. Filtering resizes the panel;
removing its final spatial resource also removes its title and padding. Logical
annotations stay above the architecture and do not qualify a regional group.
Equal region names in different states retain separate panels and resource
identities. See [deployment scopes](aws.md#deployment-scopes) for Global,
unresolved and conflicting metadata.

The [regional-state example](../examples/regional-states.svg) uses explicit
provider-region bindings and includes a singleton state in the same region as
the other two states. Generate it with:

```sh
planorama --state network=tests/fixtures/multi/network.json --state application=tests/fixtures/multi/application.json --state singleton=tests/fixtures/multi/single-region.json -o regional-states.svg
```

With `--diagnostics`, every diagnostic line carries `state="ID"`. Parse/read errors name their state. Evaluated attribute values and check messages remain excluded; safe Route destination constants follow the same rules as single-plan input. See [the multi-plan example](../examples/multi-plan.svg), generated from the two [regression fixtures](../tests/fixtures/multi).

The maintained [combined AWS example](../examples/multi-account/README.md) provides
250 inputs across three states and two accounts, 15 resolved cross-state
references, singleton regions, Global/unknown scope and substantial relationship
indexes. Its SVG and offline HTML share the same inputs and mappings.
