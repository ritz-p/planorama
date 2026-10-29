# AWS managed resource inventory

The baseline is HashiCorp AWS Provider **6.70.0**, official release commit
[`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f),
checked on 2026-10-22. Its **1,742 managed types across 249 service packages**
are recorded in [the inventory](../tests/fixtures/aws-inventory/6.70.0.json).
The source is the official Provider registration, versioned resource documentation
and `terraform providers schema -json`, not the implementation's existing type list.
Data-source, list-resource, ephemeral-resource and action registrations do not
enter the managed denominator. The inventory records actual data-source names,
schema deprecation, service category/name and aliases sharing a registration factory.

The [on-demand support report](resource-support.md) evaluates runtime behavior and
links the pinned resource documentation. Its baseline notes distinguish existing
classifications from basic-only display; conditional relationship links remain
separate. No managed types are missing basic role, presentation and service
capabilities. Neutral presentation and unresolved scope are explicit fallbacks, not
claims of full relationship support. Counts here describe types; input coverage
continues to count state-qualified resource instances and lifecycle objects.

The [semantic review data](aws-semantic-audit.md)
track missing ownership, network, reference, scope and summary capabilities separately
from basic classification, including prioritized issues and deferred cases.

[DynamoDB configuration references](dynamodb.md) document the exact table, replica,
legacy global-table, streaming-destination and table-configuration contracts.

## Capability policy

[`basic_entries.rs`](../src/provider/aws/basic_entries.rs) is the exact-type runtime
table. The existing role, presentation, service classification and scope rules take
precedence. New policies, identities, associations and configuration controllers
remain distinct. Independent compute/service resources can be Spatial; other new
types default to Logical without inventing a verified owner. Basic capabilities
never create containers, child cards, association edges or communication paths.
Generic Terraform references and dependencies remain available.

The inventory retains the Provider's `names/data/names_data.hcl` labels. Runtime
labels correct its historical brand names against the [AWS documentation index](https://docs.aws.amazon.com/)
(for example, Amazon MQ rather than the Provider's AWS MQ). Resources use provider-aware labels on card or logical action-section headers.
No official artwork is required for basic support; see [service labels](icons.md).

Regional entries use static provider region evidence, subject to the existing
resource-override contract. A region attribute in the new Provider schema does
not expand the accepted override versions. Explicit unresolved overrides remain
unknown. New deleted/deposed objects do not borrow current provider region.
Global entries describe global service objects, independently of control-plane
endpoint regions. `ResourceRegionDisabled` alone is not proof of Global scope:
Roles Anywhere is regional, while hosted Direct Connect target scope stays unknown.
Cross-region StackSet targets, Cloud Control target types, multi-region S3 access
points, incident replication sets and unverified WAF scopes retain documented
unknown scope. Newly classified types do not inherit provider account ownership;
shared/cross-account target proof requires a dedicated rule.

Provider-source dispatch, explicit unresolved bindings, managed/data distinctions,
historical identity and selection rules remain shared by CLI, library and Wasm.
Future unregistered types retain existing fallback behavior. Basic support does
not inspect or retain policy bodies, credentials, service definitions or arbitrary
attribute values, and does not invent resources from names or literal identifiers.
Saved-plan provider evidence and dedicated relationship proofs keep their separately
pinned verification versions; this inventory is not a compatibility claim for them.

## Verified resource aliases

Provider dispatch reuses canonical capabilities without rewriting Terraform types,
addresses, state-qualified identities, modes, lifecycle objects or provenance.
Distinct alias/canonical instances remain distinct. Provider and reference evidence
checks still apply at both endpoints; literal IDs/ARNs do not create relationships.

The six inventory aliases were checked against the pinned 6.70.0
[registration](https://github.com/hashicorp/terraform-provider-aws/blob/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/internal/service/elbv2/service_package_gen.go),
full managed/data schemas (inventory SHA-256), and each canonical resource's
versioned documentation, which explicitly describes identical functionality.
Each managed pair shares its factory, region behavior and complete schema,
including the subnet, security-group, VPC, listener, forwarding, certificate and
attachment fields consumed by existing rules. No prefix or service-name inference
is used.

| Alias | Canonical | Managed factory | Data equivalence |
| --- | --- | --- | --- |
| `aws_alb` | `aws_lb` | `resourceLoadBalancer` | Same `dataSourceLoadBalancer` and schema |
| `aws_alb_listener` | `aws_lb_listener` | `resourceListener` | Same `dataSourceListener` and schema |
| `aws_alb_listener_certificate` | `aws_lb_listener_certificate` | `resourceListenerCertificate` | Neither registered |
| `aws_alb_listener_rule` | `aws_lb_listener_rule` | `resourceListenerRule` | Alias unregistered; canonical has a separate Framework data source |
| `aws_alb_target_group` | `aws_lb_target_group` | `resourceTargetGroup` | Same `dataSourceTargetGroup` and schema |
| `aws_alb_target_group_attachment` | `aws_lb_target_group_attachment` | `resourceTargetGroupAttachment` | Neither registered |

Only the three verified data aliases inherit data capabilities. Unregistered data
aliases retain their fallback classification. Listener certificate aliases share
the verified listener/certificate association contract described in [AWS](aws.md#elbv2-certificates-and-trust-stores-908).
The support matrix follows canonical rule source references for verified managed
aliases, while displaying the original type and separate managed/data capabilities.

After obtaining the pinned source and schema using the procedure below, run:

```sh
node tests/support/verify-aws-aliases.cjs SOURCE /tmp/aws-inventory-schema/schema.json
cargo test --locked --lib provider::aws::aliases
```

`SOURCE` is the extracted pinned Provider source directory. The verifier checks
the schema hash, full schema equality, shared factories and official alias notes.
Ordinary Rust tests use committed fixtures offline. The
[mixed fixture](../tests/fixtures/aws-aliases-plan.json),
[default SVG](../examples/aws-aliases.svg),
[service-name SVG](../examples/aws-aliases-service-names.svg) and
[offline HTML](../examples/aws-aliases.html) retain original alias names alongside
separate canonical instances. Regenerate the HTML with:

```sh
cargo run --locked -- tests/fixtures/aws-aliases-plan.json --format html -o examples/aws-aliases.html
```

## Reproduce and update

Inside the development container:

```sh
sh scripts/prepare-aws-inventory.sh
sh scripts/update-resource-support.sh
cargo test --locked --lib provider::aws::basic
```

The first command downloads the pinned official source into a temporary directory,
installs signed AWS Provider 6.70.0 through Terraform, captures its schema and
compares the regenerated inventory byte-for-byte. It needs network access and does
not configure credentials, call AWS or apply resources. The schema SHA-256 is
recorded in the inventory. Ordinary tests use the committed inventory offline.
`node scripts/aws-inventory.cjs SOURCE SCHEMA OUTPUT [VERSION COMMIT]` performs extraction separately;
missing registrations, schemas, resource docs or service names fail explicitly.
Missing alias documentation is accepted only for the same service and factory
with a documented registered counterpart.

For a new baseline, first follow the comparison procedure below. After review,
update both scripts' default pinned version/commit and copy the reviewed inventory
into `tests/fixtures/aws-inventory/VERSION.json`. Review added and removed exact
types against their documentation and actual schema;
update the runtime table with explicit role, presentation, scope and service labels
decisions. Preserve historical implementation types when a Provider removes them.
Do not infer runtime capabilities from prefixes or automatically replace reviewed
entries when updating the inventory. Extend unknown-scope reasons when necessary.
Update the inventory test's baseline/count and matrix input, then regenerate the
matrix and examples and run the required Layout quality check.

### Compare candidate baselines

Acquisition requires network access; comparison uses only local files. Run these
commands inside the development container. Verify the official release/tag's full
40-character commit before setting `NEW_VERSION` and `NEW_COMMIT`; neither the
extractor nor Terraform proves that a caller-supplied source commit belongs to a
version. Floating `latest` is not accepted. Explicit output directories must be
empty, keeping old/new schemas and Terraform lock files separate.

```sh
sh scripts/prepare-aws-inventory.sh 6.70.0 5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f /tmp/aws-before
sh scripts/prepare-aws-inventory.sh "$NEW_VERSION" "$NEW_COMMIT" /tmp/aws-after
sh scripts/update-resource-support.sh
node scripts/aws-inventory-diff.cjs \
  /tmp/aws-before/inventory.json /tmp/aws-after/inventory.json \
  /tmp/aws-before/schema.json /tmp/aws-after/schema.json \
  output/resource-support.md > /tmp/aws-inventory-diff.json
```

No credentials, AWS API calls or apply are needed. The no-argument prepare command
still checks the committed baseline byte-for-byte. Explicit-version acquisition
does not replace the committed inventory. Keep both original schema files: each
inventory's SHA-256 binds the comparator to its exact schema bytes. The JSON report
records both version/commit/hash tuples and the support matrix hash, with sorted
types and JSON Pointer change paths. Reusing these inputs reproduces the report.
Regenerate the full report from the checkout being compared; a filtered or stale report cannot establish its capabilities.

`managed.added`, `managed.removed` and `managed.changed` cover exact managed types.
Changed entries separate inventory metadata (service, region, factory, alias and
deprecation included) from schema structure. Structure includes resource schema
version, attribute/block additions and removals, types, nested attributes/blocks,
nesting modes, bounds and required/optional/computed/sensitive/deprecated flags.
Documentation prose is omitted; attributes named `description` are still compared.
`otherSchemas` reports data sources and other supplied schema sections separately;
they never increase the managed denominator. Only sections present in the supplied
official schema can be compared.

For example, a synthetic changed resource produces this abbreviated report:

```json
{
  "schema": {"status": "compared", "missing": []},
  "managed": {
    "added": ["aws_new_service"],
    "removed": [],
    "changed": [{
      "type": "aws_example",
      "metadata": [],
      "schema": {"status": "compared", "changes": [{
        "path": "/block/block_types/child/nesting_mode",
        "kind": "changed", "before": "list", "after": "set"
      }]}
    }]
  }
}
```

Use `-` for either unavailable schema file. The report then says
`"status": "not-compared"` and names the missing side(s); empty metadata changes
are **not** evidence of no schema changes. Invalid/missing inventory fields,
duplicate types, invalid aliases, schema hash or managed-set mismatches, incomplete
schemas and unsupported formats fail with a nonzero exit status. The current
reader supports legacy inventories without a format marker (or `formatVersion: 1`)
and Terraform schema `format_version: "1.0"` with versioned block schemas.
An unsupported future schema representation requires updating the reader explicitly.

Review every `review` entry against its attached runtime matrix row and linked
conditional rule sources. New types require a classification decision even if an
existing implementation row exists; absent rows are `null`. Changed types require
review of role, scope, service labels and dedicated relationships. Removed types retain their
historical runtime support. A similar name is not rename evidence: without explicit
registration/documentation proof, removal and addition remain separate. Alias
metadata likewise does not establish full schema or behavior equivalence.

The command never chooses runtime classifications or updates baselines. Schema
comparison does not prove runtime compatibility or dedicated relationship correctness;
retain the [real-plan compatibility checks](terraform-compatibility.md). After human
review, follow the baseline update steps above. Reports alone do not require SVG
regeneration or Layout quality; runtime changes that affect diagrams do.
Offline synthetic comparison tests run in the Resource support workflow:

```sh
node --test tests/support/aws-inventory.test.cjs tests/support/aws-inventory-diff.test.cjs
```

The small [fixture](../tests/fixtures/aws-basic-plan.json),
[default SVG](../examples/aws-basic.svg),
[service-name example](../examples/aws-basic-service-names.svg) and
[offline HTML](../examples/aws-basic.html) exercise new services, role/presentation
boundaries, scope, references, long addresses and foreign-provider fallback.
Regenerate HTML with:

```sh
cargo run --locked -- tests/fixtures/aws-basic-plan.json --format html -o examples/aws-basic.html
```
