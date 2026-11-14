# Cloudflare plan support

Cloudflare support reads saved `terraform show -json` plans. It never contacts the
Cloudflare API. The canonical provider source is
`registry.terraform.io/cloudflare/cloudflare`; a resource prefix alone is not
provider evidence. Provider aliases, Terraform modes, named states and deposed
instances retain their independent identities.

## Pinned contract

The inventory uses Terraform Core **1.9.8** and official Cloudflare Provider
**5.28.1**, source commit
[`9b51a582458f5c5d46d8547ed2604fcba91fb861`](https://github.com/cloudflare/terraform-provider-cloudflare/tree/9b51a582458f5c5d46d8547ed2604fcba91fb861).
`tests/fixtures/cloudflare-inventory/5.28.1.json` records the schema SHA-256,
272 managed types, 468 data types, source registrations, documentation paths and
attribute shapes. Nested object attributes are distinct from Terraform blocks.

In the prepared Docker or WSL environment, with Terraform 1.9.8 on PATH:

```sh
sh scripts/prepare-cloudflare-inventory.sh
node scripts/update-cloudflare-types.cjs
cargo fmt --all
```

The first command fetches the pinned official source and provider binary, obtains
the schema without credentials or API requests, and compares the inventory.
When updating the pin, update the version/revision together in the acquisition
and inventory scripts, regenerate the inventory, and check changed contracts
before changing classifications or reference rules.

## Capability levels

- **Basic**: known type and mode, provider label, conservative Spatial/Logical
  placement and allowlisted Account/Zone administration labels.
- **Partial**: a documented subset of a type's references is verified.
- **Dedicated**: the documented relationship contracts have dedicated verification,
  presentation and regression tests. This never means every provider field is rendered.
- **Unknown**: absent from the pinned inventory, or provider identity is unproven.
  Generic Terraform provenance remains available without Cloudflare semantics.

Inventoried types have basic support unless a dedicated contract is described below. Account, Zone, load balancer,
Workers script, Pages project, R2 bucket, KV namespace, D1 database, Access
application and cloudflared tunnel are spatial managed resources. Other types
and data sources are logical. No administrative scope creates physical
containment, deployment Region, network traffic or AWS account identity.
Zone uses `account.id`; other scoped types use their own `account_id`/`zone_id`.
Only 32-digit hexadecimal administration IDs are recognized; other shapes,
unknown values and sensitive IDs remain unavailable. Tokens, credentials,
script bodies, DNS contents and arbitrary configuration values are not retained
for presentation. Known type classification is not proof of a network relationship.

## Zone and DNS

[Sample SVG](../examples/cloudflare-dns.svg) is generated from the sanitized
[plan fixture](../tests/fixtures/cloudflare-dns-plan.json).
Zone is an administrative resource, not a physical network container.
DNS records and Zone DNSSEC stay logical with independent action/lifecycle rows.
Their `zone_id` references navigate to exactly one current Cloudflare Zone when
explicit Terraform provenance and known, non-sensitive IDs agree. Zone account
references similarly verify `account.id` against a Cloudflare Account's `id`.
Singleton Zone data sources support `id` and `zone_id` outputs only when their
known IDs agree; plural lookup results are not endpoints. Duplicate current IDs
fail closed, while deposed instances remain visible without claiming ownership.

Zone, DNS record and Zone DNSSEC have dedicated support for these administration
contracts. Other DNS fields remain configuration: DNS names, contents, IPs,
CNAMEs and proxy settings do not identify an origin or prove traffic. Verified
Zone membership can carry its known account label to the DNS helper without
creating AWS scope or a physical containment edge. Provider aliases alone do not
prove or disprove account membership; the resource IDs establish correspondence.
