# Offline pipeline benchmarks

## CLI stage timings

Add `--timings` to a normal SVG or HTML invocation to report timings on stderr.
It works with stdin, named states, filtering, provider evidence and existing
diagnostics. Diagram bytes are unchanged. No timing report is collected unless
enabled: disabled spans perform only the collector check, without clock reads,
report allocations or additional graph traversal. There is no JSON destination.
Successful generation emits the report after output I/O; failed runs keep their
existing error and exit status without emitting a partial timing report. A timing
report write failure is returned as an I/O error.

Each row has a sequential `id`, a `parent` row ID (`0` means top level), a stage,
`inclusive_ms`, `exclusive_ms` and numeric workload counters. Rows follow start
order. Repeated input, parse, semantic and layout rows give per-state costs in
sorted state-ID order; projection may remove states before layout. Report rows
never include state names, resource addresses, input paths or plan values.
Provider evidence may invoke parsing internally; these rows belong to the
provider row rather than representing additional states.

| Stage | Measured work and counters |
| --- | --- |
| `input_io` | Reading each JSON input, including waiting for stdin |
| `provider_evidence` | Optional saved-plan loading, Terraform subprocess and evidence validation; an absent evidence source is a cheap no-op |
| `terraform_parse_resolve` | JSON decoding and Terraform reference resolution; input bytes |
| `cross_state_resolution` | Named-state relationship resolution; states |
| `semantic_transform` | Architecture lowering for each state; input resources and reference edges |
| `selection_projection` | Filtering and view projection; input nodes/edges or states |
| `layout_routing` | Placement, routing and spacing refinement for each retained state; selected nodes/edges |
| `cross_state_routing` | Each cross-state endpoint-to-margin route; obstacles |
| `composition_serialization` | SVG composition and optional HTML metadata/viewer serialization; named-state count |
| `output_io` | Diagram and optional layout-report writes, including their status messages; diagram bytes |

Layout includes its routing work; no separate routing estimate is added to it.
Named-state layout and cross-state routing occur inside composition: subtract
their durations to obtain composition's exclusive time. Inclusive rows must not
be summed. Exclusive rows are disjoint, but their sum is not the wall-clock total:
`total_ms` starts after argument parsing and generation-config resolution and ends after output I/O, includes
validation, diagnostic work, intermediate destruction and instrumentation, and
excludes timing-report formatting/writing and subsequent cleanup. Diagnostic
scene construction is charged to composition. Output I/O measures writes, not
durable storage synchronization. No extra routing passes or benchmark loops run.

Build once, then measure representative committed inputs directly:

```sh
docker compose exec -T dev cargo build --release --locked --bin planorama
docker compose exec -T dev target/release/planorama examples/plan.json --timings -o /tmp/small.svg
docker compose exec -T dev target/release/planorama examples/terraform-large/plan.json --timings --format html -o /tmp/large.html
docker compose exec -T dev target/release/planorama \
  --state application-a=examples/multi-account/application-a.json \
  --state application-b=examples/multi-account/application-b.json \
  --state shared-services=examples/multi-account/shared-services.json \
  --remote-state application-a:module.application.data.terraform_remote_state.shared=shared-services \
  --remote-state application-b:module.application.data.terraform_remote_state.shared=shared-services \
  --remote-state application-b:data.terraform_remote_state.peer=application-a \
  --timings -o /tmp/multi.svg
```

Use `--format html` to compare the named-state viewer. Keep input hashes, revision,
compiler, machine, output destination, diagnostic options and cache conditions
with results. Run on an otherwise idle machine; repeat manually if a distribution
is needed. These observations do not guarantee performance, and CI checks report
structure and byte equality rather than timing thresholds.

One smoke measurement on 2026-10-16 used Docker Desktop Linux, Rust 1.85.1,
release mode, `23a58b6` plus the #430 instrumentation, and the large HTML command
above (output `/tmp/issue-430-large.html`). The input SHA-256 was
`0767ce05f828ac12e8aadeab8f7b13cced41b07cb20db9d03f463d204cfe91a8`.
It reported 50 input resources, 40 selected nodes, 79 edges and 379,882 output
bytes: total 30,986.755 ms, layout/routing 30,968.665 ms, parsing 5.166 ms and
composition 3.427 ms. Other validation processes were active, so this single
sample only demonstrates identifying the dominant stage; it is not an idle-machine
baseline, an overhead comparison or a performance guarantee.

## Synthetic benchmark harness

Run `docker compose exec -T dev cargo test --release --locked benchmarks::pipeline -- --ignored --nocapture`
from the repository root. No Terraform, credentials, network or benchmark library
is required after dependencies have been fetched. Normal tests skip measurements.

The deterministic v1 generator creates 100, 500 and 1000 resources, in VPC/subnet
groups of 20. Sparse cases contain containment references. Dense cases also
reference up to four preceding instances within each group, exercising routing
without introducing random inputs. These are bounded-degree architecture graphs,
not an all-to-all worst case. Groups do not communicate across VPCs.

Pass `-e PLANORAMA_BENCH_SIZES=100,500 -e PLANORAMA_BENCH_REPEATS=3` to
`docker compose exec` to select scales and repeats. Run in release mode on an idle machine and
compare medians with identical compiler, hardware, power settings and input hashes.

Output includes generator version, revision/dirty state, compiler, OS, architecture,
available parallelism, graph/input/SVG sizes and a deterministic input fingerprint.
CSV rows separate parsing, semantics, layout/routing, SVG rendering and total
wall-clock milliseconds. Total excludes generation, warmup, reporting and object
destruction. A small untimed warmup precedes measurements. There are no timing
assertions in CI; machine-dependent thresholds would be flaky. The baseline below
is observational and does not promise absolute performance.

The separate [combined multi-account example](../examples/multi-account/README.md)
measures 250 inputs across three states, with cross-region/state paths and
large annotation indexes. Run its opt-in `benchmarks::multi_account::baseline`
test in release mode for per-state layout time, dimensions, crossings, shared
path lengths, bends and visible-border overlap. It complements the bounded-degree
scale generator above; ordinary CI does not run its expensive measurements.

The separate [spacing calibration](../examples/terraform-large/spacing-calibration.md)
runs through the manual CI workflow and compares old/default/doubled refinement budgets.
It records bounded-pass work and per-edge shape without shortest-route searches.

## Initial baseline (2026-10-09)

Revision `4389b6092a1f0290a9f4ee48ed77a929ba368e90` plus this benchmark-only
working change; Windows x86_64, AMD64 Family 25 Model 33 Stepping 2, 32 logical
processors, Rust 1.85.1 MSVC / LLVM 19.1.7, release profile, one run per case.
Input fingerprints are included so future generators cannot silently change work.

```csv
pattern,nodes,edges,run,input_bytes,input_fnv1a,svg_bytes,parse_ms,semantic_ms,layout_ms,render_ms,total_ms
sparse,100,95,1,20023,3ae0864f8b338abf,102255,6.087,0.119,0.151,0.965,7.321
dense,100,405,1,29758,5fa1ec6a34dbca0e,320042,24.338,0.183,233.569,2.798,260.888
sparse,500,475,1,99740,a65c3b9940c9a1e5,480664,130.636,0.622,0.717,4.823,136.798
dense,500,2025,1,148415,46836bfee1d95bda,1626509,554.939,0.907,3572.022,13.588,4141.456
sparse,1000,950,1,199386,6e3e84d5062a06bd,954152,507.082,1.309,1.629,9.369,519.389
dense,1000,4050,1,296736,72e483db8a20e79b,3274976,2126.122,1.844,10350.491,29.180,12507.637
```
