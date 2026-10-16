# Terraform coverage report

Selection: no CLI filter.

Input counts original Terraform resource instances. Selected counts distinct source instances represented after selection and projection, including logical rows, embedded Routes and lowered relationship provenance exactly once. Excluded = input - selected; excluded instances may be hidden by filters or projection. Group and component cards are not resources. Managed/data addresses and current/deposed instances remain distinct; states are counted separately.

Supported means the provider has a semantic classification for the resource type; unsupported means no such classification. Logical presentation does not imply unsupported.

Unknown account denominators include managed AWS instances. Unknown region denominators include classified, non-global managed AWS instances. Data sources and non-AWS resources are outside these scope denominators. Zero applicable instances means not applicable.

Findings count diagnostic records, not unique resources. Categories can overlap and must not be summed into a resource total or coverage percentage. Input findings include unassociated findings; selected findings only belong to represented instances. Analysis covers supported rules only; zero findings does not prove complete resolution. Cross-state resolution findings are not included in this analysis.

This aggregate report does not enumerate resource findings or excluded identities. Aggregate counts are not truncated. Interactive selected finding details are available through --html-semantic-diagnostics, with its own disclosed limits.

## Whole diagram

Input completeness: unknown. Analysis: supported rules only.

| Accounting | Input | Selected |
| --- | ---: | ---: |
| Resource instances | 6 | 6 |
| Supported classification | 6 | 6 |
| Unsupported classification | 0 | 0 |
| Unknown account / applicable | 5 / 5 | 5 / 5 |
| Unknown region / applicable | 5 / 5 | 5 / 5 |

Excluded resource instances: 0.

| Finding category | Input | Selected |
| --- | ---: | ---: |
| unresolved | 0 | 0 |
| ambiguous | 0 | 0 |
| dynamic | 0 | 0 |
| unavailable&#95;endpoint | 0 | 0 |
| classification | 0 | 0 |
| scope | 10 | 10 |
| drift | 0 | 0 |

Unassociated input findings: 0 (included only in input categories).

## State: (single input)

Input completeness: unknown. Analysis: supported rules only.

| Accounting | Input | Selected |
| --- | ---: | ---: |
| Resource instances | 6 | 6 |
| Supported classification | 6 | 6 |
| Unsupported classification | 0 | 0 |
| Unknown account / applicable | 5 / 5 | 5 / 5 |
| Unknown region / applicable | 5 / 5 | 5 / 5 |

Excluded resource instances: 0.

| Finding category | Input | Selected |
| --- | ---: | ---: |
| unresolved | 0 | 0 |
| ambiguous | 0 | 0 |
| dynamic | 0 | 0 |
| unavailable&#95;endpoint | 0 | 0 |
| classification | 0 | 0 |
| scope | 10 | 10 |
| drift | 0 | 0 |

Unassociated input findings: 0 (included only in input categories).

