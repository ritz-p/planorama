# Coverage and semantic diagnostics JSON

Use `--coverage-report-json coverage.json` for a deterministic, versioned report
from the same coverage records and counters as the Markdown `--coverage-report`.
Both reports can be requested together. This is separate from geometry/routing
`--layout-diagnostics-json` and does not change diagram bytes or exit policy.

```sh
planorama plan.json -o diagram.svg --coverage-report-json coverage.json
planorama plan.json --focus aws_instance.app --focus-depth 0 \
  --coverage-report-json coverage.json --coverage-json-details 100
```

The output must be a separate file, not stdout. Input, config, provider evidence,
diagram and other report paths cannot alias it, including filesystem links.
Report write errors fail generation before writing the diagram.

## Version 1

The [JSON Schema](coverage-report.schema.json) defines the wire format.
`schema_version` changes for incompatible structure or meaning changes. Consumers
must reject unsupported versions and tolerate additive fields and new diagnostic
reason/category names. There are no timestamps, input paths or raw plan values.

- `analysis` is `state_local_supported_rules_only`; `cross_state` is
  `not_collected`. Cross-state diagnostics printed on stderr are not included.
  Zero findings is not proof of complete analysis or runtime reachability.
- `summary` aggregates all states; `states` retains each state, even with zero
  selected resources. Empty `state` means an unnamed single input.
- `collected=false` makes accounting and findings counters `null`, not zero.
  `input_complete` is `true`, `false` or `null` (unknown); unavailable accounting
  also uses `null`. If any state is unavailable, aggregate accounting is unavailable;
  collected states still have their own counters.
- `input` and `selected` count unique Terraform source instances, including
  embedded/lowered provenance once. `excluded` is their resource-count difference.
  Group cards are not resources. Account/region denominators retain the shared AWS
  applicability rules; a zero denominator is not applicable.
- `input_findings` and `selected_findings` map category names to record counts.
  Missing categories mean zero when collected. Counts overlap and must not be
  added to obtain a unique resource count. Unassociated findings are included only
  in input counters; their identities are not exported.
- `selection` reports direct selection, additional context, and overlapping
  retention-reason counts. Uncollected selection reasons have null counters.
  Direct selection is the represented intersection before context closure;
  projection-suppressed resources are excluded, not additional context.

## Optional records

Details default to disabled. `--coverage-json-details N` accepts 0–1000 and bounds
**each** of the resource and finding arrays across all states. It does not enable
Markdown selection details or HTML diagnostics.

Records contain only effective selected instances. Identity is the tuple
`state`, `address`, `mode`, `deposed`; null `deposed` means current. Resource records
include direct/context selection and retention reasons. Finding records include
the attribute path and bounded semantic reason/category/message, never attribute
values, expressions, provider configuration or credentials. Addresses/selectors
can themselves contain identifiers, so detailed export is explicit.

Each array reports `total`, `omitted`, `records_complete` and `truncated`.
Disabled details still disclose the available total and omitted count; they are
not marked truncated. Enabled details are truncated when the limit omits records.
Unavailable accounting gives null totals and false completeness. Record
completeness concerns only the selected, supported analysis; it is independent
of input completeness. States and resource identities are sorted; findings use
the shared stable identity/attribute/reason order. Excluded identities are absent.
