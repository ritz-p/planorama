# AWS provider evidence

Plan JSON alone never proves an AWS account. Terraform omits dynamic provider
blocks from that representation, including roles that could change the final
account. Existing JSON-only invocations keep their provider bindings, regions,
resource identities and state separation, but no longer receive account labels.

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

The initial supported archive format is Terraform **1.9.8** with **hashicorp/aws
5.100.0**, verified from the archive's dependency lock. Other versions are input
errors so unexamined provider authentication schemas cannot establish an account.
Terraform 1.9.8 must
be on `PATH`, with the saved plan's provider schemas already installed in the
working directory. Planorama does not run `init`, `plan`, `apply`, refresh or
credential lookup. It runs `terraform show -json` against a private temporary
copy of the supplied saved plan, then compares the complete parsed JSON to the
input (JSON whitespace and object-key order do not matter). A mismatch or failed
show is an input error. `TF_CLI_ARGS*` and `TF_LOG*` are removed for this command;
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

A supported provider must have only literal role targets in `assume_role` and
`assume_role_with_web_identity` blocks. Their role expressions must agree with the
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

The development Docker image includes Terraform 1.9.8. Run:

```sh
docker compose exec -T dev cargo test --locked --bin planorama plan::evidence
docker compose exec -T dev cargo build --locked --bin planorama
docker compose exec -T dev sh tests/support/verify_provider_plan.sh
```

The last check downloads the pinned AWS provider schema and generates synthetic
plans with fabricated credentials and a local STS fixture. It checks static roles,
omitted dynamic roles, mismatches, JSON-only fallback and repeated provider keys
across states. It performs no apply. Unit tests cover malformed/missing snapshots,
aliases, module bindings, unresolved expressions and JSON configuration syntax.
