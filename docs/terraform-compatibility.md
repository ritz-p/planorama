# Terraform compatibility

JSON acceptance, deployment-region evidence and saved-plan account evidence have
different boundaries. Accepting `format_version` major 1 does not certify every
Terraform language feature or provider release.

| Boundary | Verified versions | Limits |
| --- | --- | --- |
| JSON interpretation | Terraform 1.9.8, 1.10.5, 1.11.4, 1.12.2, 1.13.5, 1.14.9, 1.15.9, 1.16.6; AWS 5.100.0 / 6.0.0 in selected combinations; JSON 1.2 | The committed cases below, not every resource or language feature |
| Resource region override | Terraform 1.9.8 and 1.16.6 / AWS 6.0.0 | VPC, Subnet, CloudWatch Log Group and the [static evidence contract](aws.md#deployment-scopes) |
| Saved-plan provider evidence | Terraform 1.9.8 or 1.16.6 / AWS 5.100.0 | The separate [archive and account contract](provider-evidence.md); AWS 6.0.0 is excluded |

Fixed Core archives and checksums are pinned in
[`matrix.json`](../tests/fixtures/compatibility/matrix.json), which also selects
the cases for each Core. Provider versions and checksums are pinned in each
case's `.terraform.lock.hcl`. CI never resolves a floating release.

## Reproduction

Run from the repository root with the development Docker image built:

```sh
docker compose exec -T dev node scripts/terraform-compatibility.cjs prepare
docker run --rm --network none --mount type=bind,source="$PWD",target=/workspace planorama-dev node scripts/terraform-compatibility.cjs capture
docker compose exec -T dev node scripts/terraform-compatibility.cjs check
docker compose exec -T dev cargo test --locked --lib compatibility_tests
docker compose exec -T dev cargo test --locked --test terraform_plan
```

On PowerShell, use `source="${PWD}"` for the bind mount.
`prepare` downloads checksum-verified linux_amd64 archives and initializes exact
provider selections, using committed locks read-only when present.
Each preparation recreates its dedicated case directory, removing obsolete HCL
and prior local state. Downloads remain in the separate checksum-verified binary
and provider caches.
`capture` uses the same binary and initialized provider for plan, show and schema.
Only the top-level timestamp is removed. Inputs are copied from existing offline
fixtures with required Core/Provider versions substituted and committed
`compatibility/inputs` HCL added. Binary plans stay under ignored
`output/terraform-compatibility`.
Each committed `capture.json` binds the normalized prepared inputs, Terraform
archive, lock, plan and extracted schema with SHA-256. `check` re-reads the declared
source HCL, nested modules, additional HCL and local-state fixture, so source changes
require recapture. Capture also rejects prepared inputs that changed after preparation.

Capture uses no network, empty local managed state and public dummy credentials.
There is no apply or cloud account. The language case reads the existing synthetic
local-state file; its sensitive value is an intentionally public placeholder.
Review JSON before committing; this capture is not an arbitrary-plan sanitizer.

## Comparisons and boundaries

`aws5` and `aws6` use identical HCL and state at each Core version, varying only
the provider. Each named case varies only Core across versions. `overrides` is
a separate AWS 6-only configuration; AWS 5 has no such deployment override.
`language` uses the built-in provider. Additional Core versions run `aws5` and
`language`; AWS 6 and override cases retain the two original Core versions.

The Node check compares planned values, changes, configuration, checks and
extracted schema contracts across Core versions, excluding volatile metadata.
Rust compares identity, action, reference provenance, completeness, checks and
coverage; SVG and HTML are compared for equivalent drawing inputs.
Changed-path metadata is excluded from semantic node comparisons: provider schemas
and Core versions may expose different optional/unknown attributes without changing
the diagram. The path analyzer is checked separately against the supplied metadata.
The override case verifies equal resolved regions, relationships, coverage and
SVG/HTML across both exact Core versions. Negative tests apply to both real
captures; no intervening or later Core/Provider versions are implicitly allowed.

Observed differences are asserted before normalization: 1.16.6 / AWS 6.0.0
adds ALB identity metadata with an unknown ARN, and 1.16.6's built-in
`terraform_data` adds unused `store: null`. The committed JSON retains both.
The [identity contract](https://developer.hashicorp.com/terraform/plugin/framework/resources/identity)
and [`store` reference](https://developer.hashicorp.com/terraform/language/resources/terraform-data)
explain those additions. This matrix does not implement identity-based import or
the new write-only store functionality.

The language case also broadens remote-state `relevant_attributes` from
`outputs.network_id` to `outputs` on 1.13.5, 1.14.9, 1.15.9 and 1.16.6. The check asserts that precise
difference; Planorama retains the emitted provenance. Language-case metadata is
therefore not a byte-identical rendering contract across versions.

Captured Core/Provider combinations preserve ECS `jsonencode` dependencies
only at `container_definitions.references`; nested environment provenance is not
serialized. The captures omit dynamic ingress expressions while retaining the two planned
ingress rules. Neither upgrade recovers that lost provenance. Generic references
remain conservative. Provider 6 adds default deployment-region attributes; explicit
override semantics are compared separately with the three-type schema contract
and global IAM Role negative control.

Cases cover child modules, count/for_each, managed/data, create/read/no-op actions,
known/unknown values, sensitive input, lifecycle precondition and check status.
Existing synthetic tests cover delete, replacement, move, import and deposed
objects; this empty-state capture does not certify those real transitions.
Ephemeral resources, write-only arguments and action invocations are unverified.
Missing information cannot be reconstructed from JSON.

Unknown minor fields are ignored, unsupported format majors are rejected, absent
optional completeness flags stay unknown and unknown actions remain `Other`.
See the official [JSON contract](https://developer.hashicorp.com/terraform/internals/json-format)
and [v1 compatibility promises](https://developer.hashicorp.com/terraform/language/v1-compatibility-promises).

Version-sensitive production paths are `plan/providers.rs` (static region
evidence), `plan/evidence.rs` and `cli/provider_plan.rs` (saved-plan evidence).
The ordinary JSON parser gates on format major, not Core version. The evidence
allowlists are intentional; this comparison does not establish a general 1.9.8
defect or validate newer internal binary-plan archive formats.

## CI

Ordinary Rust tests use committed JSON without Terraform execution.
**Terraform compatibility** checks fixture contracts on relevant PRs; manual
capture uses fixed tools with external networking disabled.
**Compatibility output** runs only on pushes to `release`. It builds the checked-out
CLI once and processes the matrix's committed JSON with that same binary, without
Terraform downloads or capture. SVG and HTML are compared across equivalent inputs;
language metadata differences are checked explicitly. Generation metadata comes from
the same binary and settings and remains part of the comparison.
The `compatibility-output-<SHA>` artifact contains outputs and `report.json` with
the checkout revision, case, input version, format and failures. A failure fails CI.
This workflow does not create tags, publish releases or upload distribution binaries.

Local output comparison after building the CLI:

```sh
docker compose exec -T dev node scripts/compatibility-output.cjs
```
