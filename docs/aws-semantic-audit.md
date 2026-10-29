# AWS semantic review data

The pinned [inventory](aws-inventory.md), [schema paths](aws-semantic-evidence.json)
and [review decisions](aws-semantic-decisions.json) are the machine-readable sources.
Query one exact managed type or official service package inside Docker:

```sh
node scripts/aws-semantic-audit.cjs aws_msk_cluster
node scripts/aws-semantic-audit.cjs kafka
node scripts/aws-semantic-audit.cjs --check
node --test tests/support/aws-semantic-audit.test.cjs
```

Queries return inventory metadata, candidate paths and decisions as JSON. There is
no checked-in Markdown register. Runtime defaults can be [generated separately](resource-support.md).

## Interpretation

Implemented refers only to the named capability; Partial preserves bounded rules;
Basic-only establishes no dedicated semantics; Deferred requires a concrete use
case and endpoint contract. P1 prioritizes network/service relationships, P2 bounded
configuration chains, P3 retains defaults pending that review. A closed issue does
not upgrade support. Review records describe their inspected commit; current
behavior is defined by [service contracts](aws.md) and implementation tests.

Schema candidates are configurable, non-sensitive paths, not verified endpoint
types. The conservative name scan can miss relationships; an empty list proves
nothing about completeness. Managed/data registrations, collection shapes, output
identities and lifecycle must be verified independently. A reference does not prove
ownership or containment. Credentials, arbitrary policy/payload interpretation and
runtime reachability remain excluded; safe summaries require explicit allowlists.

## Maintenance and verification

Update relevant decisions when a service contract changes. The offline check
validates pinned schema identity, exact managed coverage and decision references;
it does not assert runtime semantic completeness. After [acquiring the pinned
schema/source](aws-inventory.md#reproduce-and-update), refresh paths and verify
representative contracts:

```sh
node scripts/aws-semantic-audit.cjs --schema /tmp/aws-inventory-schema/schema.json
node tests/support/verify-aws-semantic-contracts.cjs /tmp/aws-inventory-schema/schema.json SOURCE
```

SOURCE is the acquired official source directory. The verifier checks configurable
paths, types and documentation without AWS calls. Keep investigation reports in
PRs/Issues; retain only current decisions and non-obvious contract evidence here.
