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
Logical rows and embedded resource children count once. Components and annotation/group
cards add no resources. Input minus selected counts resources outside the effective
view, including semantic metadata suppression and CLI selection. It does not mean
the resource is absent from Terraform. Module and focus selection retain required context; changed-only
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

## Markdown report

Use `--coverage-report PATH` with SVG or self-contained HTML to write a separate
UTF-8 Markdown report. No report is created without the option. The report has
whole-diagram and per-state input/selected/excluded counts, supported/unsupported
classification counts, scope denominators and finding categories. Logical rows
are not inherently unsupported. It labels effective filters and input completeness.
An unavailable collection reports unknown counts rather than zeros; aggregate
completeness is incomplete if any state is incomplete, otherwise unknown if any
state has no completeness flag.

The report contains aggregate findings without attribute paths. Counts are not
truncated. Input-derived identifiers are escaped for Markdown, and content has no
timestamps or machine-specific paths. Individual selected findings remain
available in the optional HTML panel.

Selection reasons are collected only when a coverage report is requested, during
selection and context closure before projection. Whole-diagram and per-state
summaries distinguish direct selection (the represented source identities in the
module/focus/changed-only intersection), additional context, and effective selected
resources. Input minus effective selected remains excluded, including suppressed
metadata; direct plus additional context equals effective selected. Logical rows,
embedded Routes and lowered relationship sources keep their existing accounting.

Reasons include direct selection, containment ancestors, verified Route Table
ownership, cross-state provenance and lowered relationship representation. An
already direct resource can also be needed as context, so its reasons overlap;
reason totals must not be added together. A lowered relationship selected through
its module can require endpoints outside that module. Focus on a lowered address
still selects its endpoint representation under the existing focus rules.

`--coverage-selection-details N` adds at most N selected identity rows (0-1000;
default 0), ordered by state, address and deposed key, with omitted counts. The
limit applies across all states, not independently to each state. Each row includes
state, managed/data mode, current/deposed identity and all retention reasons. The
report never lists excluded identities, invents fragment links, or includes plan
values, provider settings or raw expressions. This option requires a report.

The [small context example](../examples/selection-coverage.md) can be regenerated:

```sh
planorama tests/fixtures/containment-plan.json --focus aws_instance.app --focus-depth 0 --coverage-report examples/selection-coverage.md --coverage-selection-details 10 -o output/selection.svg
```

SVG and HTML diagrams have no coverage summary cards or reserved summary space.
The report option does not change diagram bytes. Input files, provider evidence,
diagram output and layout JSON output cannot alias the report destination;
validation precedes writes. `-` is not a report destination. Write errors fail the
command and are reported on stderr.

Regenerate the maintained [report example](../examples/coverage.md) in Docker:

```sh
docker compose exec -T dev sh scripts/update-examples.sh
```

The script runs the equivalent of:

```sh
planorama examples/plan.json -o examples/diagram.svg --coverage-report examples/coverage.md
```

The optional `viewer-semantic-diagnostics` JSON bridge has schema version 1 and
contains only selected findings, each explicitly qualified by state and mode.
`total`, `omitted` and `records_complete` describe record retention independently
of `available`, `input_complete` and supported-rule analysis. Fixed explanations
come from existing diagnostic reasons; [viewer limits](viewer.md#semantic-diagnostics)
also bound serialized bytes and displayed rows.
