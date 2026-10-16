# Coverage metadata

`Graph.coverage` records input and effective-view accounting independently of
rendered cards. Collection runs after semantic transformation; filtering retains
only selected identities and findings while preserving input counters. Named
states own separate records in the architecture's ordered state map. An identity
is the enclosing state ID, Terraform address, mode, and optional deposed key.
Single-plan input has no named state. Mode is retained in the resource record;
Terraform data addresses include the `data.` prefix.

Input resources are distinct parsed Terraform instances. Selected resources are
the intersection of input identities and node or lowered relationship provenance.
Logical rows and embedded routes count once. Components and annotation/group
cards add no resources. Input minus selected counts resources outside the effective
view, including semantic metadata suppression and CLI selection. It does not mean
the resource is absent from Terraform. Focus retains required context; changed-only
may also retain unchanged context.

Account denominators include managed AWS instances. Region denominators include
classified, regional managed AWS instances; global, data and non-AWS instances do
not become unknown regions. Unclassified types have no assumed regional scope.
Unknown counts mean applicable scope lacks verified evidence. They are not errors.
Both input and selected counters carry their own denominators. Aggregate counters
are the sum of per-state counters, even when states have equal resource addresses.

Findings use stable snake-case reason codes, an optional attribute path, and source
identity. Categories distinguish unresolved, ambiguous, dynamic, unavailable
endpoints, classification, scope and drift. Category counters count deduplicated
records, not unique affected resources; categories can overlap on a resource.
The existing semantic diagnostics are reused with exact current/deposed identity.
Original input finding counts remain separate from selected finding counts.

Collection covers supported semantic rules only, not exhaustive Terraform
validation. Cross-state mapping diagnostics remain on stderr and are not part of
these counters. `collected = false` means analysis is unavailable, not zero.
`collected = true` with zero findings means zero findings within supported rules.
`input_complete` independently preserves Terraform's complete flag: absent is
unknown, false is incomplete, and true does not make semantic analysis exhaustive.
`unassociated_findings` counts input findings such as outputs or unmatched drift
that cannot safely be attributed to an input instance; their identities are not
retained. Their categories contribute to input counters, but never selected
counters. No inference assigns them to a similarly named resource.

Resources and states use ordered maps. Findings sort by source address, deposed
key, optional attribute, code and category, then deduplicate. Consumers qualify
identities with the enclosing state before aggregating. HTML consumers must use
selected records only and disclose any additional record or display limits.
The contract contains counts and safe identifiers, never attribute values,
provider settings, raw expressions or plan payloads. Existing stderr formatting
is unchanged by collection.

## Diagram summary

SVG and HTML display the same measured Diagram Coverage annotation for each
state, with a whole-diagram summary above named states. Input, selected and
excluded resources come from shared accounting, not rendered cards. Unknown
account/region fractions use the selected applicable-resource denominator;
zero applicable resources display `not applicable`. Reference finding counts
are a labeled subset of selected findings and may overlap on a resource.
Unassociated findings are explicitly labeled as input findings. Counts never
imply invalid infrastructure.

Summaries distinguish complete, incomplete and unknown input status, supported-rule
analysis, and unavailable collection. CLI focus/changed-only modes and retained
context are labeled. `no CLI filter` still allows semantic projection to exclude
resources. Summary text and state headings use measured wrapping.
Whole-diagram input completeness is incomplete if any input is incomplete,
unknown if any remaining input has no flag, and complete only if all inputs say so.
This summary does not combine or change the state-local plan-status flags.
The `data-view-coverage` metadata carries version 1 counters and per-state totals,
including explicit named state IDs in local and aggregate summaries, with no
resource identities. HTML uses the same SVG summary without DOM counting.
