# Build performance

## Native linker

Linux x86_64 Compose development and the two Cargo test CI suites use upstream
[mold 2.40.4](https://github.com/rui314/mold/releases/tag/v2.40.4), installed by
`scripts/install-mold.sh` with a pinned SHA-256. Rust 1.85.1 invokes GCC 12.2 in
Bookworm through `scripts/native-linker.sh`; GCC 12.1+ supports `-fuse-ld=mold`.
The wrapper uses `--no-fork` so linker CPU/RSS remain attributable to the build
process tree. It changes the linker, not Rust compilation options. The installer skips
other architectures. Native target selection is an environment setting, so a
plain checkout needs no additional tool and `.cargo/config.toml` stays unchanged.
Wasm uses `wasm-ld` with its import-memory and 4 GiB maximum. macOS, Windows,
cross compilation and published native releases are outside this rollout.

Install and disable commands are in [CONTRIBUTING](../CONTRIBUTING.md).
`PLANORAMA_LINKER=system` selects `cc` inside the wrapper for controlled
measurements; clean the benchmark target when changing this variable because
Cargo cannot detect changes inside a linker wrapper. For normal use, override
the Cargo linker variable to `cc`, which Cargo fingerprints.

Run `docker compose exec -T dev sh scripts/benchmark-linker.sh` from the repository
root. The script uses an isolated `/tmp/planorama-linker-benchmark-target`, cleans
it between linkers and at completion, disables incremental compilation and compiler
wrappers, and retains measurements under `output/linker-benchmark`.
The script copies tracked source into a temporary directory so edits and generated
reports cannot invalidate the empty build script during a warm run.
Cold means an empty target/profile with downloaded dependencies already available;
warm means an immediate unchanged build using that target. This does not measure
network or image construction time. Test compilation uses all targets/features;
CLI compilation uses the release profile and default features. Both linkers also
run the full ordinary Rust suite and render the same CLI fixture for byte comparison.

Each TSV records elapsed/user CPU/system CPU seconds and peak RSS in KiB using
GNU time. Per-driver invocations are recorded separately in `*-links/`; their sum
is work across potentially overlapping links, not time to subtract from wall time.
Cargo `--timings` HTML reports provide compilation-unit durations and concurrency.
Resource measurements cover child processes, with peak RSS the largest process,
not the sum of concurrently resident processes. Repeated runs are needed for
statistical claims; one local sample only establishes compatibility and indicative
cost on this machine.

### Local comparison, 2026-10-25

Source: `f218e9244b989e0adda80bc9b81d5f522cff85e8`, with only the linker tooling
changed. Docker Desktop/WSL2 Linux x86_64, 32 available CPUs, Rust 1.85.1,
GCC 12.2.0, GNU ld 2.40, mold 2.40.4. No sccache; incremental disabled.
The system pass preceded the mold pass; filesystem caches were not dropped.

| Build | Linker | Cold wall s | Cold CPU s (user + system) | Peak RSS KiB | Warm wall s |
| --- | --- | ---: | ---: | ---: | ---: |
| All test executables | GNU ld | 71.24 | 681.34 | 4,781,324 | 0.10 |
| All test executables | mold | 58.76 | 612.43 | 4,953,296 | 0.11 |
| Release CLI | GNU ld | 24.50 | 157.17 | 1,293,908 | 0.09 |
| Release CLI | mold | 23.31 | 154.46 | 1,285,656 | 0.08 |

| Build | Linker | Driver calls | Sum of link wall s | Link CPU s | Link peak RSS KiB | Other build CPU s |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Test | GNU ld | 27 | 18.90 | 18.57 | 618,268 | 662.77 |
| Test | mold | 27 | 5.20 | 35.65 | 1,071,852 | 576.78 |
| Release | GNU ld | 18 | 3.35 | 3.11 | 97,680 | 154.06 |
| Release | mold | 18 | 1.60 | 7.34 | 201,812 | 147.12 |

Other build CPU is total CPU minus driver CPU: compilation plus Cargo/build-script
overhead, not pure rustc code generation. Detailed compilation-unit timing is in
the generated Cargo HTML reports. Warm builds invoked no compiler or linker.
Test linking is substantially faster but uses more CPU and peak memory; total
build speed also depends on compilation variability. Release linking is a small
part of the total, so this result does not justify changing release publication.
The CLI ELF `.comment` identifies mold 2.40.4, and both linkers produced identical
SVG for `examples/plan.json`.
