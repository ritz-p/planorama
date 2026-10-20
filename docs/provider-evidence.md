# AWS provider evidence

The [JSON compatibility matrix](terraform-compatibility.md) does not expand this
saved-plan archive contract. Account evidence and resource-region evidence have
independent version allowlists.

Plan JSON alone never proves an AWS account. Terraform omits dynamic provider
blocks from that representation, including roles that could change the final
account. JSON-only invocations retain provider bindings, regions, resource
identities and state separation while leaving account scope unknown.

## Input contract

```sh
planorama plan.json --provider-plan saved.plan -o diagram.svg
planorama --state app=app.json --state shared=shared.json \
  --state-provider-plan app=app.plan --state-provider-plan shared=shared.plan \
  -o architecture.svg
```

Evidence is optional per state. Missing evidence leaves that state's accounts
unknown. Duplicate evidence flags, unknown state IDs, mixing positional/named
forms, stdin evidence and output paths that overwrite evidence are rejected.
JSON input can still use stdin. Each evidence file is matched to its own JSON;
equal provider keys in different states do not share evidence.

The verified pairs are explicit; other combinations are input errors:

| Terraform Core | hashicorp/aws | Archive | Authentication schema |
| --- | --- | --- | --- |
| 1.9.8 | 5.100.0 | tfplan format 3, embedded configuration and lock | AWS 5.100.0 |
| 1.16.6 | 5.100.0 | tfplan format 3, embedded configuration and lock | AWS 5.100.0 |

AWS 6.0.0 remains unsupported for saved-plan evidence even though it is in the
separate JSON/region compatibility matrix.

The exact Terraform version used to create and export the plan must be on `PATH`.
Initialize that plan's working directory and fixed provider version yourself
before invoking Planorama. Its `terraform version -json` check compares the
binary version with the input JSON and the installed dependency selection with
the verified pair. The embedded lock must select that same AWS version; the
archive's format and creating Core version must also agree with the JSON.
Planorama does not download tools or run `init`, `plan`, `apply`, refresh or
credential lookup. It runs `terraform show -json` against a private temporary
copy of the supplied saved plan, then compares the complete parsed JSON to the
input (JSON whitespace and object-key order do not matter). A mismatch or failed
show is an input error. `TF_CLI_ARGS*` and `TF_LOG*` are removed for both commands;
Terraform's update checkpoint is disabled.
Terraform stderr is suppressed because it can include configuration values.

## Completeness and supported configuration

The archive's [embedded snapshot](https://github.com/hashicorp/terraform/blob/v1.9.8/internal/plans/planfile/config_snapshot.go)
provides the files Terraform used, rather than a potentially stale working tree.
Its module manifest must exactly cover the plan's module tree, and every module
must have snapshot files. Missing/duplicate modules, duplicate archive entries,
duplicate providers, malformed configuration and unsupported override files are
input errors. Both `.tf` and `.tf.json` files are read; external configuration
directories or hand-written completeness declarations are not accepted.
Snapshot entries are limited to 64 MiB each and 128 MiB total uncompressed.

Providers are matched by their exact local name, alias and declaration module to
the state-local configuration key. Resource/module bindings come from the matched
plan, including aliases passed into child modules. Aliases never become account
identities. Removed/deposed resources without a current binding remain unknown.

A supported provider must have literal values throughout its complete block,
including region, endpoints, profile, tags and all authentication options.
Attribute types and nested block names are checked against the captured
[AWS 5.100.0 provider schema](../src/plan/evidence/aws-5.100.0.json).
Literal booleans and numbers are also accepted for string attributes, matching
Terraform's primitive conversion (for example `skip_metadata_api_check = true`).
Unknown fields, nested blocks, nulls, functions and expressions that require
evaluation leave account scope unknown. HCL role blocks must use block syntax;
JSON configuration accepts objects or arrays of objects for those blocks.
Only literal role targets in `assume_role` and
`assume_role_with_web_identity` blocks establish an account. Their expressions must agree with the
matched JSON. Any dynamic block in the provider, variable/template expression,
unsupported role representation, invalid role ARN, missing declaration or
conflicting role accounts leaves the account unknown. Even a statically empty
dynamic block is unsupported. Multiple roles are accepted only if all point to
the same account; this avoids assuming chain ordering or provider evaluation.
Only 12-digit account IDs in the commercial `aws` partition are supported.

This is structural verification of a trusted Terraform artifact, not a signature
or proof of origin. Deliberately fabricated/repacked archives and a substituted
Terraform/provider installation are outside the trust boundary. Use the saved plan produced
by the same trusted planning process as the JSON. A digest or supplied HCL alone
would not prove that configuration was used to produce the plan.

## Privacy and validation

Only the validated account ID and conflict status enter the model. Role ARNs,
session names, external IDs, credentials and unrelated values are not retained in
the model, rendered, or included in evidence error messages. Saved plans and JSON
can themselves contain secrets; the temporary plan copy is removed on completion.
Account IDs are intentional deployment metadata visible in SVG/HTML. Region,
account, provider binding and Terraform state remain independent identities.

The development Docker image defaults to Terraform 1.9.8. The evidence harness
installs checksum-pinned 1.9.8 and 1.16.6 binaries into separate directories in
`output/provider-plan/bin`, with separate initialized work directories and pinned
provider locks. It does not change the image's default executable. Run:

```sh
docker compose exec -T dev cargo test --locked --lib plan::evidence
docker compose exec -T dev cargo build --locked --bin planorama
docker compose exec -T dev node tests/support/provider_plan.cjs prepare
docker run --rm --network none --volumes-from "$(docker compose ps -q dev)" \
  planorama-dev node tests/support/provider_plan.cjs verify
```

The preparation step downloads binaries and initializes the pinned provider.
Verification runs without external networking and generates real saved plans
with fabricated credentials and a local STS fixture. Both Core versions must
produce the captured provider schema. Checks cover static roles, omitted dynamic
roles, other variable references, incompatible binaries, JSON mismatches,
JSON-only fallback, state isolation and suppressed logging/CLI arguments.
It performs no apply. `sh tests/support/verify_provider_plan.sh` is a convenience
wrapper for both phases inside the existing container; CI uses the networkless
verification command above. Unit and CLI tests cover malformed snapshots, archive
headers, lock/version mismatches, unknown authentication fields, binding and
privacy failures.

## Adding a verified pair

Inspect the target Core's `internal/plans/planproto/planfile.proto` and
`internal/plans/planfile` implementation independently of the AWS authentication
schema. The examined [1.9.8 protobuf](https://github.com/hashicorp/terraform/blob/v1.9.8/internal/plans/planproto/planfile.proto)
and [1.16.6 protobuf](https://github.com/hashicorp/terraform/blob/v1.16.6/internal/plans/planproto/planfile.proto)
store format at field 1 and Core version at field 14. Unknown top-level fields
are skipped by wire type; malformed or duplicate version fields are rejected.
Terraform itself must also successfully read the entire archive.

Review the provider's authentication implementation and capture its actual
`terraform providers schema -json` block, retaining attribute types and nested
block structure. A new provider version needs its own reviewed contract; do not
reuse a region-version allowlist. Extend `VERIFIED_PAIRS`, the real-plan harness,
checksum/lock inputs, positive and negative cases, and this table together.
Run the Docker checks, regenerate SVGs and verify final-commit Layout quality.
