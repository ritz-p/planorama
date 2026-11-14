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
Worker, Workers script, Pages project, R2 bucket, KV namespace, D1 database, Access
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

## Edge configuration

[Edge sample](../examples/cloudflare-edge.svg) combines DNS, custom WAF rules,
bulk and page-rule redirects, and load balancing. The load balancer is spatial;
pools, monitors, monitor groups, lists and rule configuration remain logical.

Dedicated references cover `default_pools`, `fallback_pool`, `country_pools`,
`region_pools`, `pop_pools`, pool `monitor`/`monitor_group`, and monitor-group
`members.monitor_id`. Collection values and the explicit reference set must
match completely, with unique current endpoints in the same known account.
Zone-scoped load balancers acquire account context through verified Zone
provenance. Monitor-group set masks must be entirely known; set positions are
not identities.

Rulesets have partial semantic support: their exclusive Account/Zone owner,
`execute` rule targets in the same scope, and redirect-list names in the same
account are verified. Redirect lists must have `kind = "redirect"`. Rule
expressions, redirect URLs, page-rule actions and pool origin addresses remain
opaque configuration. Dynamic rule pool overrides and other action-parameter
references remain generic Terraform provenance. These references do not establish
live traffic, effective protection or healthy origins.

Cloudflare v5 object/list attributes use Terraform's aggregate expression
references. Verification checks the evaluated typed fields, complete relevant
collections, referenced output identities and scope together. It does not apply
AWS nested-block/index rules to these attribute contracts. Managed configuration
references and data-source lookup outputs are treated separately.

## Applications and storage

[Application sample](../examples/cloudflare-applications.svg) combines separately
managed Workers, versions, routes, Pages and storage resources.

Workers and Pages are edge applications, with no inferred VPC membership.
Separately managed versions, deployments, routes, custom domains and Pages
Domains retain their own lifecycle rows. Storage stays separately managed.

Verified application references require explicit, complete Terraform provenance,
unique current identities and a known matching account:

- Worker versions identify a Worker by its UUID or name, according to the
  referenced output. A legacy Workers script's `id` is its `script_name`.
- Deployments identify their named Worker and versions. A version's proven
  Worker must be the same endpoint as the deployment's proven Worker, and
  known version percentages must total 100.
- Routes inherit account context only from a proven Zone. Custom domains use
  their account and explicit service reference. Patterns and hostnames do not
  establish ownership.
- Worker/script binding discriminators select KV `namespace_id`, R2
  `bucket_name`, or D1 `id`. Binding names and contents are not identities.
- Pages preview/production deployment configurations identify KV, R2 and D1
  through their respective typed maps. Pages Domains identify a project by
  its name or equivalent project `id`.
- R2 account, bucket name and jurisdiction must match; an unknown jurisdiction
  cannot be treated as the default. D1 `id`/`uuid`, R2 `id`/`name`, Pages
  `id`/`name`, and legacy script `id`/`script_name` must agree before aliases
  can be used.

This is partial support: unrelated bindings, deployment settings, script code,
secret values and database contents are neither interpreted nor retained.
D1 bindings require the configured `id`; a conflicting known `database_id`
is rejected. A computed unknown `database_id` on Worker versions does not
replace that explicit identity. Legacy script version ownership cannot be
inferred from a matching name alone. Verified references describe configured
relationships, not successful deployment or runtime requests.
