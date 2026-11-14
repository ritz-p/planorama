# Resource support

Current defaults come from provider code; service labels and Spatial cards do not
prove dedicated relationships. Unknown types remain visible with unresolved
placement. Foreign providers do not acquire AWS rules from lookalike type names.

Generate only the needed type prefix inside the prepared Docker or WSL environment:

```sh
sh scripts/update-resource-support.sh aws_lambda_
```

The disposable report is output/resource-support.md. Omit the prefix only when a
full runtime catalog is needed, such as an inventory baseline comparison. Do not
commit it. The exporter evaluates classification, display, scope and service names;
source references identify conditional rules, not guaranteed edges. Extend its
literal collector in tests/unit/resource_support.rs when adding a new source form.

Use `sh scripts/update-resource-support.sh cloudflare_` for the pinned Cloudflare
inventory. Its section lists each managed/data mode separately, runtime role,
placement, service label, capability level, conditional attributes and verified
endpoint contracts. Basic, Partial and Dedicated describe the listed contracts;
they do not certify the entire provider API. Unknown types or unproven provider
identities receive no Cloudflare semantics. See [Cloudflare evidence and limits](cloudflare.md).

The [pinned inventory](aws-inventory.md) defines registered managed types; it does
not establish data-source support. [AWS contracts](aws.md) define evidence limits;
[semantic queries](aws-semantic-audit.md) expose type-specific review decisions.
