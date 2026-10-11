# Logical architecture components

`Graph.components` contains provider-neutral `SyntheticComponent` entities with a stable ID, kind, label and exact source-resource addresses. These entities are distinct from Terraform `Node` objects: they do not have a Terraform action, provider identity, or `ResourceRole::Container`. Original nodes remain the source of all change information. Synthetic IDs occupy a separate logical namespace and do not replace Terraform addresses.

The supported grouping rule recognizes a managed AWS `aws_lb` with one or more `aws_lb_listener` resources linked through complete, uniquely resolved `load_balancer_arn` references. `aws_lb_listener_rule` resources can join through an equally unambiguous `listener_arn` reference to an included listener. Resource type, provider, mode, visibility, uniqueness, and deposed status are checked. Literal values, partial/dynamic references, conflicting candidates, and standalone load balancers do not create groups. A rule with an ambiguous listener remains ungrouped. Membership and component order are sorted deterministically.

Target groups remain independent because they can be shared; broad ALB/NLB, API Gateway and other provider grouping rules are not inferred. The label is intentionally “Load balancer” because no subtype is inferred from values. These are logical memberships, not network containment relationships, so a component can span network boundaries without changing resource placement.

Above the architecture, three independent vertical sections appear in order:
Logical Resources (cyan), Logical Components (purple), then Relationship Resources
(neutral). Empty sections reserve no space. Section colors identify their type;
Terraform cards and component member references retain the canonical action colors.
Section titles appear once. Inner resource cards start with their address;
component cards use their actual label without a classification prefix or Members
heading, and relationship index cards start with numbered entries.
Each component has an inner card ordered by synthetic identity. Headings and member
addresses wrap without truncation. Members are sorted by address and link to the
original spatial or logical card; missing or ambiguous targets use neutral,
non-clickable cards. Accessible titles preserve full addresses and operation/move/
deposed context. These annotations do not affect architecture placement or routing.

`data-entity-kind="synthetic"`, architecture identity/provenance and component kind
remain on each summary; `data-component-id` associates original resource cards with
their logical entity. Original cards retain imports, drift, replacement reasons
and move provenance. Synthetic entities do not increase resource/action counts or
create dependency edges. See [the component example](../examples/components.svg).
