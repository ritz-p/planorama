# Output provenance

Each parsed state's `outputs` preserves root output names, source resource addresses, completeness and diagnostic categories. Configuration expressions are resolved through the existing local, input and module-output aliases. Output values, sensitive values and backend configuration are not retained. Literal-only outputs have no sources; dynamic or ambiguous instance selection never chooses an arbitrary source. Multi-resource expressions preserve all known sources. Incomplete results must not be used as cross-state relationships. `--diagnostics` reports unresolved output provenance under `output.NAME` and, for named inputs, its state ID.

Whole-module values follow exported outputs only, excluding unrelated internal
resources. Field/index selection from flattened aliases remains incomplete because
the expression metadata does not identify which aggregate field supplied the value.
Bare instance-collection traversals also remain conservative even with only one
current instance: Terraform may serialize a selector as a separate reference.
Independently exact resource traversals remain available alongside these unknowns.
