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

Inventoried types have Basic support unless a documented reference or endpoint
contract below provides Partial or Dedicated support. Account, Zone, load balancer,
Worker, Workers script, Pages project, R2 bucket, KV namespace, D1 database, Access
application, Queue and cloudflared tunnel are spatial managed resources. Other types
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
CNAMEs and proxy settings do not prove traffic. Verified
Zone membership can carry its known account label to the DNS helper without
creating AWS scope or a physical containment edge. Provider aliases alone do not
prove or disprove account membership; the resource IDs establish correspondence.

A managed DNS record with `type = "CNAME"` can show `Configured Pages CNAME`
when its `content` explicitly references a Pages Project's `subdomain`, both
known values agree, and the project is the unique current endpoint in the DNS
record's verified Zone account. The record's `proxied` flag must be known and
non-sensitive; both true and false describe configured DNS, not observed traffic.
Masked content/subdomains, missing Zone provenance, account disagreement,
duplicate endpoints and data-record consumers fail closed. A singleton Pages
data endpoint is eligible only with the same known account and output evidence.
Worker Route patterns and Custom Domain/Pages Domain hostnames are not CNAME
target identities. Matching those names, a shared Zone, proxying, or a literal
`pages.dev` suffix does not create a public-path relationship. This does not prove
domain activation, TLS readiness, DNS resolution or runtime reachability.

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
opaque configuration. In pinned v5.28.1 `action_parameters.overrides` contains
action/enabled/sensitivity settings and rule/category overrides, not Pool or
Worker endpoint identities. `origin` contains only host/port, while `host_header`,
SNI, URI and redirect targets contain strings/expressions rather than typed
origin identities. Load-balancer pool contracts are separate from Rulesets.

`execute` and redirect-list references retain their documented verification.
Origin routing, URL rewrites, redirects, cache/header settings and security
overrides do not gain Pool/Worker/origin edges. `ruleset`, `rulesets`, `rules` and
`phases` in skip configuration describe exclusions, not forwarding or protection;
they are not promoted to execute relationships. Other action-parameter
references remain generic Terraform provenance. These references do not establish
live traffic, effective protection or healthy origins.

Cloudflare v5 object/list attributes use Terraform's aggregate expression
references. Verification checks the evaluated typed fields, complete relevant
collections, referenced output identities and scope together. It does not apply
AWS nested-block/index rules to these attribute contracts. Managed configuration
references and data-source lookup outputs are treated separately.

## Imported resources and value-only recovery

Value-only Zone/Worker recovery is disabled; there is no opt-in switch. Imported
plans with literal `zone_id`, `script` or `service` values keep their resources
and scope labels, but equal values alone do not create verified relationships.
This applies to Route → Zone, Route → Worker, Custom Domain → Zone and Custom
Domain → Worker. Restore explicit Terraform expressions to obtain the documented
verified links. A shared hostname or account never creates a direct Zone → Worker
relationship, and named states are independent identity domains.

The pinned schema provides typed Zone IDs and Worker names, so a separately
labelled configured-value match is technically possible with complete evidence.
It is not equivalent to Terraform provenance: names are account scoped, Zone data
outputs require matching `id`/`zone_id`, legacy script `id` is a name rather than
a Worker UUID, and a plan may omit other endpoints. Unknown/sensitive identities,
duplicate current candidates, conflicting scope, deleted/deposed targets and
environment-qualified services must never fall back to unrestricted matching.

The current semantic contract records verified references, not a separate
value-match confidence. Enabling recovery requires a distinct provenance model
and SVG/HTML labels, an explicit opt-in, per-state typed identity indexes, and
native/Wasm tests covering the same ambiguity and masking boundaries as explicit
references. Index construction should be linear in resources plus candidate
relations; measure large imported fixtures before enabling it. No fallback scan
or default behavior change is introduced by this decision.

## Applications and storage

[Application sample](../examples/cloudflare-applications.svg) combines separately
managed Workers, versions, routes, Pages and storage resources.

Workers and Pages are edge applications, with no inferred VPC membership.
Separately managed versions, deployments, routes, custom domains and Pages
Domains retain their own lifecycle rows. Storage stays separately managed.

Managed Custom Domains additionally verify `zone_id` against a unique current
Zone's `id` (or a singleton data Zone's matching `id`/`zone_id`). The domain and
Zone must have the same known, non-sensitive account. This independent
`Zone administration` relationship can coexist with `service` → Worker;
neither hostname matching nor literal zone IDs establish it. Missing, masked,
conflicting or ambiguous evidence fails closed without replacing the domain's
own account scope. The contract uses the pinned v5.28.1 inventory's `zone_id`
input; other schema shapes receive no inferred aliases or compatibility fallback.

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
  `bucket_name`, D1 `id`, Service `service` (Worker name), or Queue
  `queue_name`, or Hyperdrive `id`. Binding names and
  contents are not identities. Service endpoints accept a new Worker's `name`
  or a legacy script's matching `id`/`script_name`, never a Worker UUID.
  Service bindings currently require an absent/null `environment`: the endpoint
  resources do not identify named environments, including an explicit
  `production` qualifier. A supplied `entrypoint` must be a known nonempty string;
  it is not retained and does not prove that the target exports that handler.
  Unknown/sensitive qualifiers fail closed. Managed versions/scripts can refer
  to managed or singleton data endpoints, with the same name/account contract.
  In pinned v5.28.1 neither managed nor data `cloudflare_worker` /
  `cloudflare_workers_script` exposes an environment identity, and there is no
  separately registered Worker environment resource. Consequently even a known
  `production` qualifier cannot be equated to an unqualified Worker. A future
  extension requires provider-backed `(account, service, environment)` identity
  and explicit output provenance; entrypoint text alone cannot supply either.
  The entire mixed storage/service/queue collection must verify before its targets
  are split into Storage bindings, Service bindings, Hyperdrive bindings and Queue producer bindings
  rows. Repeated bindings retain one endpoint per role. Partial provenance never promotes
  the otherwise valid subset. The generated support matrix lists this combined
  binding contract; the sample includes multiple bindings to the same Worker.
  A legacy script binding to itself remains unsupported.
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

### External services and origins

Worker version and legacy script `type = "hyperdrive"` bindings verify an
explicit Hyperdrive configuration `id` in the same known account. Exactly one
current managed or singleton data endpoint must match; masked IDs, conflicting
accounts, ambiguous endpoints and incomplete mixed-binding provenance fail closed.
The separate Hyperdrive resource remains logical and retains its own lifecycle.
Its database name, host, user, password and Access credentials are not retained
for presentation. This link identifies the configured Hyperdrive object only.

Service Bindings similarly identify configured Workers, not arbitrary external
APIs. Worker source code and HTTP `fetch` calls are never parsed. Hyperdrive
`origin.host`, URL/IP values, database addresses and shared accounts do not prove
an AWS or other provider endpoint, even when Terraform expressions mention it.
No cross-provider or cross-state origin edge is inferred. Such an extension
requires an explicit typed endpoint contract with compatible scope and complete
provenance; hostname equality and named-state labels cannot substitute for it.

## Queues

[Queue sample](../examples/cloudflare-queues.svg) shows mixed Worker/script
producer bindings, Worker consumers and HTTP pull/notification consumers.
Each configured consumer uses a separate Queue, following the service's
[single active consumer constraint](https://developers.cloudflare.com/queues/reference/how-queues-works/).
Managed Queues are spatial resources; consumers remain independent logical
resources with their own actions. Neither is contained in a Worker.

- Producer bindings use `type = queue` and `queue_name`, accepting only a
  Queue's `queue_name` output. Queue IDs cannot substitute for names.
- Consumer `queue_id` accepts `cloudflare_queue.id` or `queue_id`; both
  endpoint aliases must be known and equal. The recognized consumer types are
  `worker`, `http_pull` and `notification`.
- Consumer `script_name` links to a Worker only for `type = worker`, using
  the same Worker-name contract as Service bindings. HTTP pull, notification,
  unknown and unsupported types never acquire Worker associations.
- Worker consumers additionally expose `Configured dead letter Queue` through
  `dead_letter_queue` → Queue `queue_name`, independently of producer bindings.
  The consumer's `queue_id` must first prove the source Queue. The logical
  consumer row identifies both the consumed Queue and its configured DLQ;
  self-targets and cycles among proven Queue/DLQ relations are rejected.
  HTTP-pull and notification DLQ contracts are not promoted by this support level.
- Every reference needs complete explicit Terraform provenance, known matching
  accounts and a unique current endpoint. Managed producers/consumers can use
  managed or singleton data Queue/Worker endpoints; data consumers do not gain
  managed semantics. Computed Queue producer/consumer summaries are not
  configuration evidence and never create resources or associations.

Queue names and IDs are separate account-scoped contracts. Queue jurisdiction
does not qualify the account/Queue ID API path or establish an AWS Region.
Notification destinations are not dedicated references in this support level.
Settings and payloads are not retained. Associations
describe configured producers/consumers, not message delivery or runtime
traffic guarantees.

## Access and Tunnel

Managed DNS CNAME records can identify a Tunnel through an explicit reference
to its `id`, with exact `<UUID>.cfargotunnel.com` content and `proxied = true`.
The DNS Zone must be explicitly verified and its known account must match the
unique current Tunnel endpoint. Known IDs, contents and proxy settings must be
non-sensitive. Literal CNAMEs, other suffixes (including FedRAMP variants),
unproxied records and unknown/ambiguous endpoints are not promoted.
`Configured Tunnel CNAME` describes DNS configuration only: ingress hostname
similarity does not establish reachability, connector health or Access protection.
The [Cloudflare routing contract](https://developers.cloudflare.com/tunnel/concepts/routing/)
defines the standard Tunnel subdomain and same-account restriction.

[Zero Trust sample](../examples/cloudflare-zero-trust.svg) shows an Access
application, identity provider, reusable policy, Access Groups, tunnel, configuration and
routing helpers. These remain independently managed resources. Access policies
are logical policy rows; a Tunnel virtual network is not an AWS VPC or a
physical container.

Applications and identity providers must have exactly one known administration
scope, account or zone. Ambiguous, unknown or sensitive ownership fails closed.
An application can reference an account identity provider in its known account;
a zone identity provider requires the same known zone. Reusable policy references
require a matching account, including account context obtained through a proven
Zone. The supported application fields are `allowed_idps` and `policies[].id`.
Identity-provider sets require complete known membership. Mixed inline/reusable
policy collections are not interpreted as verified policy associations.

Reusable managed Access Policies also verify `include`, `require` and `exclude`
Group conditions (`group.id`) with distinct labels. The pinned provider uses
account-only reusable Policies, whereas Groups can use account or zone APIs.
Only account-scoped Groups in the same known account are supported here; zone
Groups and ambiguous ownership fail closed. Singleton Group data endpoints must
have matching known `id` and `group_id`, and references must select `id`.
Each condition set needs fully known, non-sensitive membership and complete
explicit provenance. Literal non-Group conditions can coexist, but their payloads
are not retained. Unknown/sensitive set masks reject the whole set because set
positions cannot establish identity. Duplicate Group IDs also fail closed.
Inline application policies, nested Group-to-Group conditions and authorization
evaluation remain unsupported. The Zero Trust sample includes these three roles;
the generated support matrix lists the Policy fields and Group endpoint contract.

Tunnel configurations and routes verify `tunnel_id`; routes also verify
`virtual_network_id`. These require a unique current endpoint, matching known
account, exact `id` output and complete explicit Terraform provenance. Literal
UUID matches alone are insufficient. Access/application and Tunnel references
are configured administrative associations, not proof of authentication,
authorization decisions, ingress reachability or successful network traffic.
Hostnames, DNS names, URLs, IP addresses, CIDRs, identity configuration, secrets,
tokens and policy payloads do not create additional relationships and are not
retained for presentation. Other Zero Trust resource types retain basic support.

## Reproduction and evidence

The representative plans are deterministic synthetic inputs generated by
`scripts/cloudflare-fixtures.cjs`. Their
[provenance manifest](../tests/fixtures/cloudflare-provenance.json) records the
Core/provider/schema pin and hashes of each fixture. Known resource identifiers
and create actions are synthesized to exercise reference and lifecycle display;
these are not captured provider-generated plans or tested Cloudflare deployments.
Payload markers are artificial privacy canaries, not credentials or executable
Worker modules. The separately hand-authored foundation fixture is adversarial
input, including intentionally unsupported fields and identities.

The committed Terraform Core probe uses only built-in `terraform_data` resources
and no Cloudflare API. It verifies that nested object/list **attributes** produce
aggregate root references, including bare resource references. It does not
establish API behavior or turn these attributes into indexed Terraform blocks.

In the prepared Docker or WSL environment:

```sh
node scripts/cloudflare-fixtures.cjs
node scripts/cloudflare-fixtures.cjs --check
node --test tests/support/cloudflare-*.test.cjs
sh scripts/check-cloudflare-core-shapes.sh
cargo test --locked --lib provider::cloudflare
sh scripts/update-resource-support.sh cloudflare_
sh scripts/update-examples.sh
```

The Core probe requires Terraform 1.9.8. When intentionally changing its input,
run `sh scripts/check-cloudflare-core-shapes.sh --write` and review the small
captured expression-shape record. Provider pin updates also require regenerating
and reviewing the inventory, fixture provenance and generated Rust type catalog;
a schema match does not replace resource-specific reference tests.

The disposable `output/resource-support.md` reports all inventoried managed/data
modes with Basic, Partial or Dedicated contracts and explicit endpoint limits.
Unlisted types or unproven provider identities are Unknown. Data sources never
inherit managed binding/rule interpretation merely by sharing a type name.
All comparisons remain conditional on known scope, typed values, current unique
endpoints and complete explicit provenance.

Resource support CI checks fixture reproduction, pinned schema shapes and the
capability catalog. Browser Wasm CI also runs the Core probe and native/Wasm
SVG/HTML parity. Regression cases cover mixed providers, provider aliases, named
states, current/deposed instances, managed/data boundaries, unknown/sensitive
values and filtering. Layout quality and normal Tests remain independent checks.
No Cloudflare plan/state or live credentials are needed for these checks.

## Reference diagnostics

`--diagnostics` reports Cloudflare verification failures with resource address,
attribute and a value-free explanation. `--format html --html-semantic-diagnostics`
exposes the same findings in the existing searchable diagnostics panel. SVG/HTML
reference tooltips and SVG reference metadata carry the failure code; rejected
targets remain unlinked. HTML diagnostics retain the selected view and named-state,
managed/data and current/deposed identities.

| Code | Meaning |
| --- | --- |
| `cloudflare_sensitive_value` | A required identity, scope or collection value is sensitive |
| `cloudflare_unknown_value` | A required value is unknown until apply |
| `cloudflare_invalid_value` | A required typed value is absent/invalid, or its mask is malformed |
| `cloudflare_incomplete_collection` | Collection shape, unique membership or deployment percentages are invalid |
| `cloudflare_incomplete_provenance` | Complete, explicit Terraform endpoint provenance is unavailable |
| `cloudflare_inactive_endpoint` | A consumer/endpoint is deleted, removed or deposed |
| `cloudflare_unsupported_contract` | The type, mode, output target family or configuration contract is unsupported |
| `cloudflare_scope_unavailable` | Required administration scope is unavailable or ambiguous |
| `cloudflare_scope_mismatch` | Known endpoint administration scopes disagree |
| `cloudflare_ambiguous_endpoint` | More than one current endpoint matches the typed identity and scope |
| `cloudflare_output_mismatch` | A reference selects an unsupported endpoint output |
| `cloudflare_identity_mismatch` | Evaluated identities, aliases or R2 jurisdiction disagree |
| `cloudflare_owner_mismatch` | A deployment version lacks the same proven Worker as the deployment |

These describe failed supported contracts, not a complete audit of all Cloudflare
fields. Constants without Terraform references and unrecognized attributes do
not acquire dedicated findings. Existing generic reference-resolution failures
are retained instead of adding a redundant Cloudflare failure for that attribute.
One primary failure is retained per attempted contract: provenance is checked
before typed values, endpoint selection and ownership. When independent entries
fail within a collection or candidate stage, the table order breaks ties, making
the result independent of resource and collection enumeration order.

Unknown and sensitive masks retain their separate origins, including data restored
from prior state. Only fields read by the contract affect their diagnostics;
unrelated secret payloads and ignored computed aliases do not invalidate otherwise
verified references. No values, payloads or candidate identity strings are stored
in a failure. Diagnostics do not relax identity/scope/provenance requirements or
turn configuration references into runtime traffic.

## Provider-generated plan captures

[`cloudflare-captured`](../tests/fixtures/cloudflare-captured) supplements the
synthetic samples with actual Terraform **1.9.8** / official Provider **5.28.1**
`plan` and `show -json` output from dedicated artificial HCL:

| Case | Inputs and evidence | Presentation contract |
| --- | --- | --- |
| `create` | Empty state; Zone/DNS, Worker/version, KV/R2/D1 | Computed IDs remain unknown; Terraform provenance survives, while unverified membership/binding targets are withheld |
| `update` | Generated artificial local state; DNS TTL and KV title changes | Known Zone membership, Worker ownership and the complete storage binding collection are verified |
| `replace` | Same artificial state; explicit `-replace=cloudflare_workers_kv_namespace.cache` | Replacement keeps its action; the new unknown KV ID must not reuse the old ID for binding verification |
| `data` | Empty state; singleton Zone read from a loopback HTTP fixture and a deferred Zone lookup | The known lookup exists only in `prior_state` and is restored for references; the deferred lookup remains `read` with unknown identity |

[Create SVG](../examples/cloudflare-captured-create.svg) and
[update SVG](../examples/cloudflare-captured-update.svg) illustrate the different
evidence available before and after identities are known. These are **not live
Cloudflare deployments**. The seed state is generated from the create plan with
explicit artificial identities; it is not produced by `apply`. Replacement is
requested by Terraform CLI, not evidence of a Provider replacement modifier.
The HTTP fixture serves exactly one Zone response; it does not verify real API
behavior or authentication. No refresh, apply or remote cleanup is performed.

In prepared Docker or WSL on Linux x86_64, from the repository root:

```sh
node scripts/cloudflare-capture.cjs prepare
node scripts/cloudflare-capture.cjs capture
node scripts/cloudflare-capture.cjs check
node --test tests/support/cloudflare-capture.test.cjs
cargo test --locked --lib provider::cloudflare::captured
sh scripts/update-examples.sh
```

`prepare` downloads a checksum-pinned Core archive and initializes the official
Provider against the committed read-only lock. It recreates only the four case
directories under ignored `output/cloudflare-captured`; rerun it before recapture.
`capture` clears inherited Terraform/Cloudflare overrides, supplies a public dummy
token and points the Provider at a temporary loopback server. Managed planning
must make zero HTTP requests, and the data case must make exactly one expected GET.
For an external-network-isolated capture, after preparation use either:

```sh
unshare -Urn sh -c 'ip link set lo up && node scripts/cloudflare-capture.cjs capture'
# Or with the prepared development Docker image:
docker run --rm --network none --mount type=bind,source="$PWD",target=/workspace planorama-dev node scripts/cloudflare-capture.cjs capture
```

`capture.json` binds HCL, provider lock, generator, artificial state, selected
schema contracts and all four JSON plans with SHA-256. The top-level timestamp
is removed, and the unordered `relevant_attributes` collection is sorted without
dropping entries; other plan values and array orders are preserved. Schema
descriptions are omitted from the selected schema excerpt. The committed data
is from the dedicated artificial inputs, not a sanitizer for arbitrary user
plans. Binary plans and initialized
providers stay in `output/cloudflare-captured`; remove that directory to clean up.

Normal tests validate the committed captures without Terraform or API access.
The **Terraform compatibility** manual workflow recaptures with external
networking disabled. **Resource support** checks provenance and semantic tests;
**Browser Wasm** compares native/Wasm SVG/HTML, including changed-only rendering.
Use the same commands with recapture when changing HCL, the generator or pins.
