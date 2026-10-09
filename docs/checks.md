# Terraform check results

Planorama reads the top-level `checks` array described by [Terraform's JSON format](https://developer.hashicorp.com/terraform/internals/json-format#checks-representation). It preserves each checkable object's aggregate status and its instance statuses in `Graph.checks`. Resource preconditions and postconditions share this representation; the JSON does not distinguish individual conditions, so Planorama does not invent that distinction.

The SVG header counts **checkable objects**, not individual assertions or instances. Aggregate and instance results are never added together. `pass`, `fail`, `error`, and `unknown` have separate counts; unfamiliar strings count as `other` and remain available in the summary's SVG title. Missing or non-string statuses count as `unavailable`. Hover over the summary for instance statuses and any safely associated resource addresses. See [the check results example](../examples/checks.svg).

Resource associations require a resource-kind static address, matching resource type and mode, and an exact dynamic address belonging to that static address. The address must identify one non-deposed graph resource. No association is inferred from a static address alone, prior/moved addresses, or string prefixes. Associations use resource addresses rather than node indices and survive relationship lowering. Unmatched checks remain in the plan summary. Non-resource checks retain statuses without a resource association.

Problem messages, evaluated values, expression details, and unknown payload fields are never retained or rendered. Only status strings and verified resource addresses are retained. Non-array check collections and non-object entries are ignored; malformed instance collections do not discard an otherwise usable aggregate result. Results are sorted deterministically without discarding duplicates. Plans without usable check objects retain their existing SVG output.

Check results do not alter resource actions, plan status flags, or the CLI exit status. The summary reserves its own header row so it can coexist with plan status metadata without covering the diagram.
