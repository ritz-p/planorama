# Generation configuration

## CLI output and addresses

`-o PATH` chooses the destination; `--format svg|html` chooses the format.
The default is SVG with `diagram.svg`; HTML defaults to `diagram.html`.
`--address-format qualified` (default) displays Planorama-qualified labels;
`--address-format terraform` uses native Terraform addresses. Both preserve full
identities and wrap long addresses rather than truncating them.

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
| `format` | `"svg"` or `"html"` | `"svg"` |
| `icon-mode` | `"icon"` or `"service-name"` | `"icon"` with `icon-artwork`; otherwise `"service-name"` |
| `address-format` | `"qualified"` or `"terraform"` | `"qualified"` |
| `hide-relationship-legend` | boolean | `false` |
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

Builds with `--no-default-features` exclude icon artwork and reject explicit
`icon` mode from either CLI flags or configuration. Only `service-name` is
available in those builds; `--help` reflects the enabled feature set.

Every boolean can be disabled on the CLI with `--no-` followed by its key, for
example `--no-changed-only`, `--no-hide-relationship-legend` or
`--no-html-semantic-diagnostics`. Repeated scalar/boolean CLI options use the last
value. Booleans use flags on the CLI, not `--key=false` syntax.

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
The resolved output format determines the usual default `diagram.svg` or
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
