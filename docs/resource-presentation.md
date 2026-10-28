# Resource presentation policy

This defines shared presentation requirements; service-specific proofs are in
[AWS relationships](aws.md), [Route cards](routes.md) and [encryption](encryption.md). The [generated support matrix](resource-support.md)
records current defaults. Role, placement, deployment scope, network containment,
semantic membership and presentation ownership are independent.

The [purpose-dependent review](purpose-placement.md) covers SSM Parameter,
CloudWatch Alarm and the five IAM Identity types. Their defaults remain Logical;
optional independent presentation uses the shared [Spatial type option](configuration.md#explicit-spatial-types).

## Audit of the 42 AWS Logical types

The [original 42-type audit](https://github.com/ritz-p/planorama/blob/8fb4aef052f7d941b0a5da6b08a4fc610661a504/docs/resource-presentation.md#audit-of-the-42-aws-logical-types)
and [PR #552](https://github.com/ritz-p/planorama/pull/552) record defaults at
`ac700bb` and the reasons for proposed migrations. Those proposals are historical;
use the support matrix for current defaults and the service contracts above for
implemented ownership. The seven purpose-dependent types retain their separate
[comparison and decision](purpose-placement.md).

## Provider and selection contract

Provider interpretation must supply a unique owner identity and value-free proof
of the service-specific relationship. Exact, complete, explicit references and
required value agreement are prerequisites, not interchangeable with Terraform
dependency edges. A configured reference, semantic membership, network containment
and display ownership must remain distinguishable in the model. Ownership alone
adds no dependency, traffic, authorization or containment edge.

An owner identity includes state context and current/deposed identity. A local
identity is resolved only inside its own state graph. Cross-state links remain
links unless a separate ownership rule proves them. Current configuration never
establishes historical ownership of a deposed object. Neither layout nor renderers
may infer owners from resource types, names, ARN substrings or proximity.

The provider declares a candidate; the view resolves effective presentation after
filtering and existing context closure. Exactly one visible, eligible owner is
required. Missing, external, ambiguous, shared, self or cyclic ownership falls back
to an independent card (or the explicitly documented existing safe placement).
Do not duplicate shared resources under consumers. Nested owners need explicit
measurement support; until then reject such chains rather than losing children.
Do not restore a filtered parent or child merely to complete a display group.
Existing network-ancestor context closure is separate from presentation ownership.
Routes also retain their existing explicit `verified Route Table owner` context
closure; RDS members and new generic children do not acquire that exception.

Parent and child retain their own account, region/global and unknown scope.
Embedding requires compatible proven scope; equal unknown labels are not proof.
Existing RDS rules additionally allow a shared, nonconflicting provider configuration
with a known region when account identity is unavailable. This is configuration
compatibility, not newly inferred account evidence. Scope disagreement or unproven
compatibility requires fallback. Independent cards use only their own proven
Region/AWS Global panels and otherwise remain in unknown scope.

## Shared implementation boundary

Existing Route and RDS instance presentations reuse `view/routes.rs` card
measurements, `view/spatial.rs` projection and `svg/routes.rs` output. Route
ownership retains its specialized next-hop evidence; RDS `member_owner` is
provider-verified display ownership. Missing selected RDS parents already release
members to ordinary cards. `PresentationOwner` records a state-local Terraform
target, verified attribute and provider-supplied labels. `view/presentation.rs`
resolves Independent, Logical or Embedded after selection, rejecting ambiguous
identity, self/cyclic/nested owners and incompatible member scopes. Route-specific
proof remains in `RoutePresentation`; both paths share effective owner lookup.
Service contracts own their endpoint, scope and fallback requirements. Do not
copy another service's proof merely because both use this shared presentation
model. Network containment, configured references and embedding remain separate.

Measure headers, service labels, references, import/move badges and wrapped child
addresses before packing. Reserve parent height and actual child bounds/ports;
feed the same effective ownership into routing ancestry, masks, named-state offsets,
SVG metadata and HTML presentation/export indices. Avoid unrelated Route/RDS
geometry changes. A cluster/catalog is not a network Container merely because its
card contains children.

Each child keeps action, managed/data mode, current/deposed identity, DOM identity,
search/selection/history and focus/modules behavior. Count and coverage include
each selected Terraform resource once; grouping creates no resource. Parent-only
selection does not recreate excluded children; child-only selection retains a
visible fallback. Navigation and neighborhood/viewport exports must use actual
displayed ownership while preserving resource identities and relationship endpoints.

Moving a Logical row must preserve verified service-reference links on the new
card, including unavailable endpoints after selection. A link does not become a
drawn edge without a separately defined direction, meaning and provenance.
Never expose additional secret, policy, path or definition values to justify a
display change. Each service change needs focused identity/filter/scope regression
coverage and the rendering checks in [CONTRIBUTING](../CONTRIBUTING.md).
