# Offline pipeline benchmarks

Run `cargo test --release --locked benchmarks::pipeline -- --ignored --nocapture`
from the repository root. No Terraform, credentials, network or benchmark library
is required after dependencies have been fetched. Normal tests skip measurements.

The deterministic v1 generator creates 100, 500 and 1000 resources, in VPC/subnet
groups of 20. Sparse cases contain containment references. Dense cases also
reference up to four preceding instances within each group, exercising routing
without introducing random inputs. These are bounded-degree architecture graphs,
not an all-to-all worst case. Groups do not communicate across VPCs.

Set `PLANORAMA_BENCH_SIZES=100,500` to select scales and
`PLANORAMA_BENCH_REPEATS=3` for multiple measured runs (PowerShell:
`$env:PLANORAMA_BENCH_REPEATS='3'`). Run in release mode on an idle machine and
compare medians with identical compiler, hardware, power settings and input hashes.

Output includes generator version, revision/dirty state, compiler, OS, architecture,
available parallelism, graph/input/SVG sizes and a deterministic input fingerprint.
CSV rows separate parsing, semantics, layout/routing, SVG rendering and total
wall-clock milliseconds. Total excludes generation, warmup, reporting and object
destruction. A small untimed warmup precedes measurements. There are no timing
assertions in CI; machine-dependent thresholds would be flaky. The baseline below
is observational and does not promise absolute performance.

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
