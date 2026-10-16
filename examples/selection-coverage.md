# Terraform coverage report

Selection: focus + required context.

Focus: aws&#95;instance&#46;app; neighbor depth: 0.

Input counts original Terraform resource instances. Selected counts distinct source instances represented after selection and projection, including logical rows, embedded Routes and lowered relationship provenance exactly once. Excluded = input - selected; excluded instances may be hidden by filters or projection. Group and component cards are not resources. Managed/data addresses and current/deposed instances remain distinct; states are counted separately.

Supported means the provider has a semantic classification for the resource type; unsupported means no such classification. Logical presentation does not imply unsupported.

Unknown account denominators include managed AWS instances. Unknown region denominators include classified, non-global managed AWS instances. Data sources and non-AWS resources are outside these scope denominators. Zero applicable instances means not applicable.

Findings count diagnostic records, not unique resources. Categories can overlap and must not be summed into a resource total or coverage percentage. Input findings include unassociated findings; selected findings only belong to represented instances. Analysis covers supported rules only; zero findings does not prove complete resolution. Cross-state resolution findings are not included in this analysis.

This aggregate report does not enumerate resource findings or excluded identities. Aggregate counts are not truncated. Interactive selected finding details are available through --html-semantic-diagnostics, with its own disclosed limits.

## Whole diagram

Input completeness: unknown. Analysis: supported rules only.

| Accounting | Input | Selected |
| --- | ---: | ---: |
| Resource instances | 3 | 3 |
| Supported classification | 3 | 3 |
| Unsupported classification | 0 | 0 |
| Unknown account / applicable | 3 / 3 | 3 / 3 |
| Unknown region / applicable | 3 / 3 | 3 / 3 |

Excluded resource instances: 0.

| Finding category | Input | Selected |
| --- | ---: | ---: |
| unresolved | 0 | 0 |
| ambiguous | 0 | 0 |
| dynamic | 0 | 0 |
| unavailable&#95;endpoint | 0 | 0 |
| classification | 0 | 0 |
| scope | 6 | 6 |
| drift | 0 | 0 |

Unassociated input findings: 0 (included only in input categories).

Direct selection: 1. Additional context: 2. Effective selected: 3.

Direct selection is the represented source-instance intersection before context closure. Additional context is effective selected minus direct selection. Metadata suppressed by projection is excluded, not additional context. Lowered relationship endpoints can be context outside a selected module. Reasons are recorded during selection; one instance can have several reasons, so reason counts overlap.

| Retention reason | Resource instances |
| --- | ---: |
| containment ancestor | 2 |
| direct selection | 1 |

## State: (single input)

Input completeness: unknown. Analysis: supported rules only.

| Accounting | Input | Selected |
| --- | ---: | ---: |
| Resource instances | 3 | 3 |
| Supported classification | 3 | 3 |
| Unsupported classification | 0 | 0 |
| Unknown account / applicable | 3 / 3 | 3 / 3 |
| Unknown region / applicable | 3 / 3 | 3 / 3 |

Excluded resource instances: 0.

| Finding category | Input | Selected |
| --- | ---: | ---: |
| unresolved | 0 | 0 |
| ambiguous | 0 | 0 |
| dynamic | 0 | 0 |
| unavailable&#95;endpoint | 0 | 0 |
| classification | 0 | 0 |
| scope | 6 | 6 |
| drift | 0 | 0 |

Unassociated input findings: 0 (included only in input categories).

Direct selection: 1. Additional context: 2. Effective selected: 3.

Direct selection is the represented source-instance intersection before context closure. Additional context is effective selected minus direct selection. Metadata suppressed by projection is excluded, not additional context. Lowered relationship endpoints can be context outside a selected module. Reasons are recorded during selection; one instance can have several reasons, so reason counts overlap.

| Retention reason | Resource instances |
| --- | ---: |
| containment ancestor | 2 |
| direct selection | 1 |

## Selected identity details

| State | Terraform identity | Mode | Lifecycle | Selection | Reasons |
| --- | --- | --- | --- | --- | --- |
| &#40;single input&#41; | aws&#95;instance&#46;app | managed | current | direct | direct selection |
| &#40;single input&#41; | aws&#95;subnet&#46;private | managed | current | context | containment ancestor |
| &#40;single input&#41; | aws&#95;vpc&#46;main | managed | current | context | containment ancestor |

Shown: 3. Omitted selected identities: 0. Limit: 10 across all states. No excluded identities or inferred fragment links are included.
