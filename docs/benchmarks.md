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
| `placement`, `routing`, `routing_spacing` | Initial containment/placement, route search (including any replacement placement), and final spacing/refinement inside layout |
| `cross_state_routing` | Each cross-state endpoint-to-margin route; obstacles |
| `composition_serialization` | SVG composition and optional HTML metadata/viewer serialization; named-state count |
| `output_io` | Diagram and optional layout-report writes, including their status messages; diagram bytes |

Layout includes its child stages.
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

## Native and Wasm profiles

Build the native binary without default features and the Wasm bindings to compare
service-name rendering using the same committed inputs. Run from the repository
root in the development environment:

```sh
cargo build --locked --release --no-default-features --bin planorama
PLANORAMA_WASM_PROFILE=1 sh scripts/build-wasm-bindings.sh output/wasm-profile
node scripts/profile-pipeline.cjs native target/release/planorama typical svg
node scripts/profile-pipeline.cjs wasm output/wasm-profile typical svg
```

Choose `typical`, `dense`, `multi-state` or `multi-account` and `svg` or `html`.
Run each sample in a fresh process. JSON includes input hashes, engine, format,
Node version and architecture. Record the repository revision, compiler and machine
with comparisons. Both engines discard the diagram after measurement.

On Linux, native timing rows also report inclusive CPU milliseconds for the
calling thread from `/proc/thread-self/schedstat` and process peak resident KiB
through the end of each span from `VmHWM`, when available. The scheduler counter
can be coarse for short spans. The opt-in Wasm profile build records nested
spans for the same parser, semantic transform, placement and routing operations.
Load it with the profiling driver, which supplies the span callback. Standard
distributions omit that instrumentation. Both builds measure elapsed and Node process CPU between
existing progress callbacks. Selection includes semantic transformation, and
output includes final transfer/serialization and any named-state gutter routing.
Linear memory bytes are allocated Wasm capacity. Peak resident KiB includes
the Node runtime and compilation. Both memory measures are cumulative high-water
marks, so do not sum them or attribute a previous peak to a later stage.
Native totals include input and output I/O. Wasm totals start after bindings
initialization and file reads. Use repeated samples for performance decisions.

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
assertions in CI; machine-dependent thresholds would be flaky.

The separate [combined multi-account example](../examples/multi-account/README.md)
measures 250 inputs across three states, with cross-region/state paths and
large annotation indexes. Run its opt-in `benchmarks::multi_account::baseline`
test in release mode for per-state layout time, dimensions, crossings, shared
path lengths, bends and visible-border overlap. It complements the bounded-degree
scale generator above; ordinary CI does not run its expensive measurements.

The separate [spacing calibration](ci.md#on-demand-spacing-calibration)
runs through the manual CI workflow and compares old/default/doubled refinement budgets.
It records bounded-pass work and per-edge shape without shortest-route searches.

### Semantic input reference grouping

Run `cargo test --release --locked benchmarks::semantic_input -- --ignored --nocapture`
in the prepared Docker/WSL environment to measure base-graph construction separately
from parsing and layout. The benchmark uses the same sparse/dense generator at
40, 1,000 and 5,000 resources and the Terraform large fixture. Set
`PLANORAMA_BENCH_CASE=dense-1000` to select a case and
`PLANORAMA_BENCH_REPEATS=20` to control repetitions. An untimed warmup precedes each
case. Graph fingerprints include order, labels, completeness and provenance;
compare with the same compiler/build options. Formatting fingerprints and dropping
graphs happen outside the timed interval. There are no timing assertions.
