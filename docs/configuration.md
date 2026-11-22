# Generation configuration

## CLI output and addresses

`-o PATH` chooses the destination; `--format svg|html|json` chooses the format.
The default is SVG with `diagram.svg`; HTML defaults to `diagram.html` and JSON to `architecture.json`.
`--address-format qualified` (default) displays Planorama-qualified labels;
`--address-format terraform` uses native Terraform addresses. Both preserve full
identities and wrap long addresses rather than truncating them.

The selected format applies to resource/container cards, embedded members and
routes, Logical Resources, Logical Component members and source-backed headings,
Relationship Resources sources/endpoints, and Uses/parent/reference rows in SVG
and offline HTML. Named-state prefixes, data/managed identity, instance keys and
deposed identity remain distinct. Measurements reserve space for either format.
Known provenance/reference identities use the same module-aware formatter even
when their resource is lowered or absent from the selected view; this does not
create a new navigation target. Unresolved identifiers and unavailable component
members without unique identity context remain verbatim and unlinked. Historical
`Moved from:` addresses retain the reported spelling because a previous object's
full context may be unavailable. Free-form labels and check-object names are not
interpreted as resource identities. DOM IDs, canonical metadata, search identities
and the viewer's Copy Terraform address behavior are unchanged.

`-` selects stdin or stdout. For example, in a POSIX shell:

```sh
planorama - -o - < plan.json > diagram.svg
```

## Configuration files

`--config PATH` reads one UTF-8 TOML file. `--preset NAME` selects one of its
named presets and requires `--config`. Paths are relative to the process working
directory. No file is discovered automatically, including `planorama.toml` in the
current or parent directories. There is no user-wide configuration, environment
variable expansion, include, or preset inheritance.

```sh
planorama examples/plan.json --config examples/planorama.toml --preset review -o review.html
planorama examples/plan.json --config examples/planorama.toml --preset review --format svg --no-changed-only -o review.svg
```

Priority is **built-in defaults → `[generation]` → `[presets.NAME]` → explicit
CLI options**, regardless of where `--config` and `--preset` appear on the command
line. Omitting a key preserves the lower layer. Explicit `false` overrides `true`.
Without `--preset`, only the common settings apply. Without `--config`, existing
CLI defaults and generation behavior apply.

Both tables accept exactly these keys, using the same spelling as CLI options:

| Key | TOML value | Built-in default |
| --- | --- | --- |
| `cloudflare-value-matches` | boolean; typed Cloudflare value matches distinct from verified provenance | `false` |
| `cloudflare-accounts`, `cloudflare-zones` | Arrays of Cloudflare IDs or `unknown` | `[]` |
| `cloudflare-groups` | Boolean | `false` |
| `anonymize` | boolean; [full identifier anonymization](anonymization.md), incompatible with reports/diagnostics | `false` |
| `format` | `"svg"`, `"html"` or `"json"` | `"svg"` |
| `icon-mode` | `"service-name"` (`"icon"` is rejected) | `"service-name"` |
| `address-format` | `"qualified"` or `"terraform"` | `"qualified"` |
| `hide-relationship-legend` | boolean | `false` |
| `omit-relationship-legend` | boolean; entire card, overriding partial hide | `true` when neither legend key is set |
| `changed-only` | boolean | `false` |
| `modules` | array of module selector strings | `[]` |
| `spatial-types` | array of allowed AWS type strings (see below) | `[]` |
| `focus` | array containing zero or one resource address | `[]` |
| `focus-depth` | nonnegative integer, requires focus | `1` when focused |
| `diagnostics` | boolean | `false` |
| `timings` | boolean | `false` |
| `html-diagnostics` | boolean, requires HTML | `false` |
| `html-semantic-diagnostics` | boolean, requires HTML | `false` |
| `layout-diagnostics` | boolean | `false` |

All builds exclude icon artwork and reject explicit `icon` mode from CLI flags
or configuration. Use `service-name` or omit the option.

Every boolean can be disabled on the CLI with `--no-` followed by its key, for
example `--no-changed-only`, `--no-hide-relationship-legend` or
`--no-html-semantic-diagnostics`. Repeated scalar/boolean CLI options use the last
value. Booleans use flags on the CLI, not `--key=false` syntax.

After merging config, preset and CLI settings, an explicit `omit-relationship-legend` controls whether the card is present. If only `hide-relationship-legend` is set, `true` shows action colors only and `false` shows the full card. Use `--no-omit-relationship-legend` to restore a card omitted by a lower layer.

Arrays replace the lower layer completely; they never concatenate across layers.
`modules = []` or `--no-modules` clears module selection. Multiple CLI `--modules`
options still build a union within the CLI layer; `--no-modules` clears all earlier
CLI module selectors, and later `--modules` starts a new selection.
`focus = []` or `--no-focus` clears focus and the lower layer's depth. A depth
explicitly set in that same layer still requires a focus. `--focus ADDRESS`
replaces the configured focus; the existing single-resource selection contract
does not accept multiple focus addresses. Named-state selectors retain their
`STATE:ADDRESS` spelling. See [selection semantics](filtering.md).

```toml
[generation]
icon-mode = "service-name"
changed-only = true

[presets.review]
format = "html"
hide-relationship-legend = true

[presets.full]
changed-only = false
modules = []
focus = []
```

Inputs, named states, provider evidence, remote-state mappings, output paths,
coverage report options and layout JSON report paths remain explicit CLI arguments.
Configuration cannot invoke external commands. `--list-modules` rejects
`--config`/`--preset`, as it lists the original input rather than generating a view.
The resolved output format determines the usual default `diagram.svg`, `architecture.json` or
`diagram.html`; output and report destinations cannot overwrite the config file.

Unknown keys (including in unused presets), invalid types or enum values, malformed
TOML, missing files and missing preset names are errors. Config parsing errors
identify the supported key or syntax byte position without echoing configuration
contents or invalid values. Final option combinations and resource selectors are
validated by the existing CLI and selection pipeline after overrides. In
particular, an HTML-only setting must be disabled when overriding `format` to SVG.
An unused preset's combinations are not evaluated against the input.

The [sample file](../examples/planorama.toml) contains generation settings only.
File I/O and TOML decoding live in `cli::settings`; its typed `Generation` values
merge independently of file access, then populate the existing render, selection
and diagnostic options. SVG/HTML generation uses the shared pipeline.

## Explicit Spatial types

Repeat `--spatial-type TYPE` to display selected types as independent Spatial cards.
The exact allowlist is `aws_ssm_parameter`, `aws_cloudwatch_metric_alarm`,
`aws_iam_role`, `aws_iam_user`, `aws_iam_group`,
`aws_iam_openid_connect_provider` and `aws_iam_saml_provider`.
These types default to Logical; only HashiCorp AWS identities are eligible.
Unsupported type strings are errors. Foreign-provider lookalikes and unknown
resource types retain their existing fallback.

```sh
planorama plan.json --spatial-type aws_iam_role --spatial-type aws_iam_openid_connect_provider -o federation.svg
```

TOML uses `spatial-types = ["aws_iam_role"]` in either generation table, following
the existing kebab-case convention. Arrays replace lower layers. Repeated CLI
flags accumulate within the CLI layer; `--no-spatial-types` clears earlier and
configured types. Later flags start a new list. An empty TOML array clears the
lower layer. Rust `Options` and the Wasm JSON request use `spatial_types`.

The shared immutable projection applies presentation after selection and context
closure, before reference measurement/layout. Focus alone does not activate it.
Roles, scopes, identities, actions, relationships and coverage stay unchanged.
Service and ordinary configuration references retain their meaning and details,
including unresolved and unavailable endpoints. This adds no Embedded owner,
effective-permission, trust or monitoring-success claim and exposes no attribute
values. See the [comparison](purpose-placement.md) for the five scenarios.

## Architecture JSON

`planorama plan.json --format json -o architecture.json` exports the selected
architecture before layout. Named states, selection and anonymization use the
same projection as the diagram. The default output path for JSON is
`architecture.json`. Schema version 1 has these fields:

- `schema_version` is an integer. Readers should ignore additional fields and reject unrecognized versions. Removing fields or changing their meaning requires a new version.
- `resources` contains resources and synthetic components. Each identity is a pair of nullable state name and opaque architecture ID. Resource records include the Terraform address, deposed key, mode, type, provider source and resolved scope. Components identify their type and members.
- `relationships` contains endpoint identities (`from`, `to`), kind (including parent-to-child `containment`), direction and sorted evidence categories. An empty evidence list indicates missing provenance. The opt-in `configured-value` category identifies a typed value match. Cross-state dependencies carry `remote-state` evidence.

Anonymized IDs are local to one export. The document contains no layout coordinates or raw change values.
