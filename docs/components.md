# Logical architecture components

`Graph.components` contains provider-neutral `SyntheticComponent` entities with a stable ID, kind, label and exact source-resource addresses. These entities are distinct from Terraform `Node` objects: they do not have a Terraform action, provider identity, or `ResourceRole::Container`. Original nodes remain the source of all change information. Synthetic IDs occupy a separate logical namespace and do not replace Terraform addresses.

The supported grouping rule recognizes a managed AWS `aws_lb` with one or more `aws_lb_listener` resources linked through complete, uniquely resolved `load_balancer_arn` references. `aws_lb_listener_rule` resources can join through an equally unambiguous `listener_arn` reference to an included listener. Resource type, provider, mode, visibility, uniqueness, and deposed status are checked. Literal values, partial/dynamic references, conflicting candidates, and standalone load balancers do not create groups. A rule with an ambiguous listener remains ungrouped. Membership and component order are sorted deterministically.

Target groups join only when all supported owning paths identify the same LB.
Supported paths are Listener `default_action.target_group_arn` and
`default_action.forward.target_group.arn`, plus the corresponding Rule `action`
paths through its Listener. Multiple listeners/rules for one LB add one member;
different LBs or an unresolved parent leave that target ungrouped. Explicit count
and for_each keys are supported; bare collections, dynamic selectors, deposed,
missing, foreign-provider or ambiguous resources do not establish ownership.
Every ownership hop also requires known, nonempty planned ARN values matching the
referenced resources: Listener → LB, Rule → Listener and action → Target Group.
All repeated action/forward target values must match the referenced ARN set.
Syntactic references alone cannot prove the result of a compound expression;
unknown, absent or different values leave targets ungrouped. Values are compared
transiently during ingestion; only the boolean proof is retained.

The proof is deliberately conservative within each input state. Every managed AWS
Listener/Rule must expose complete nested target paths and complete action-level
references. Missing configuration, flattened action blocks, literal/dynamic targets
or incomplete repeated blocks suppress target-group inference for that state,
including action shapes without target paths. Existing LB/Listener/Rule membership
is retained. Filtering never recomputes ownership from a subset of consumers.
Other attribute uses of a target disqualify it, except complete single-target
`aws_lb_target_group_attachment.target_group_arn` references. Attachments and
graph-only dependencies cannot establish ownership. State-local outputs do not
assert ownership in another state; no cross-state grouping is inferred.

The label is “Load balancer” because no subtype is inferred from values. Broader
ALB/NLB, API Gateway and other provider grouping rules are not inferred. Membership
is explanatory metadata: original target cards, resource counts, target-worker
associations, placement and routes remain unchanged. Component IDs stay anchored
to the LB when targets join or leave.

Above the architecture, Logical Resources (cyan) precede Logical Components
(purple), then Relationship Resources (neutral).
Empty resource/component/relationship sections reserve no space. Section colors identify their type.
Frames fit their child-card bounds plus header and padding. Cards use all available
horizontal space before wrapping, while the overall canvas retains its architecture width.
Logical Resources use neutral group cards with action-colored inner sections and row underlines;
component headings stay purple.
Section titles appear once. Logical-resource group headings identify their provider
source, resource type and mode. Component cards use their actual label without a
classification prefix or Members heading, and relationship index cards start
with numbered entries.
Each component has an inner card ordered by synthetic identity. Headings and member
addresses wrap without truncation. Members are sorted by address and appear as
neutral text rows without individual backgrounds, borders or card wrappers. One
underline below each complete wrapped item uses the original resource's canonical
action color. Members link to the original spatial card or logical-resource row;
missing or ambiguous targets use neutral underlines and non-clickable rows. Row measurements
include wrapping, scope labels, badge height, the underline and spacing before the next member.
Accessible titles preserve full qualified addresses and operation/move/deposed
context. These annotations do not affect architecture placement or routing.
Imported members also show their own `import` badge in a reserved row. The badge
follows the original member's metadata; the synthetic component and other members
do not acquire an import operation. Visible member text and accessible titles keep
the full composed operation label, including `import / ...` and state-removal order.

`data-entity-kind="synthetic"`, architecture identity/provenance and component kind
remain on each summary; `data-component-id` associates original resource elements
with their logical entity. Member links retain `data-source-address` and operation
attributes. Original resources retain imports, drift, replacement reasons
and move provenance. Synthetic entities do not increase resource/action counts or
create dependency edges. See [the component example](../examples/components.svg).

Component cards summarize the established deployment scopes of their visible,
unambiguous members and display each member's scope beside its address. A common
region is shown only when every member supports it. Multi-region and multi-account
summaries retain all distinct scope labels in sorted order; partial account evidence,
Global, unknown, not applicable and unavailable members remain explicit. Missing
region evidence never means Global. Provider configuration identities remain on
the original resources, even when their displayed scopes match. The summary does
not assign a deployment scope to the synthetic entity or create account/region
containment. CLI filtering shows a component only when all its members remain;
unavailable or ambiguous member identities never supply guessed scope or links.

Organizations also use synthetic cards for known indexed Roots. Their sole
source row identifies the originating Organization, not membership of the
Organization in the Root. They do not assign `data-component-id` membership to
the source card. Verified parent links target the derived Root card; filtering
its source removes the card and disables those links. See
[Organizations hierarchy](aws.md#organizations-hierarchy).
