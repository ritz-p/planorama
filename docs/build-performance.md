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

## Compiler cache

Compose installs upstream [sccache 0.10.0](https://github.com/mozilla/sccache/releases/tag/v0.10.0)
using the SHA-256 in `scripts/install-sccache.sh`. Linux x86_64 uses the musl
binary with Rust 1.85.1; other Docker architectures fall back to direct rustc.
`RUSTC_WRAPPER` is explicit in Compose and the Cargo test jobs; ordinary host
Cargo, release publication and Clippy keep their existing environment.

Outside Docker, install with `sudo sh scripts/install-sccache.sh`, then opt in
with `RUSTC_WRAPPER="$PWD/scripts/rustc-cache.sh" SCCACHE_CACHE_SIZE=2G cargo build --locked`.
Without this environment setting the host development path is unchanged.

Cargo's target directory avoids invoking unchanged compilers at all. sccache
reuses eligible rustc outputs when Cargo does invoke the compiler. It does not
cache final native executable links, test harnesses, proc-macro links or the
combined rlib/cdylib engine. Build scripts still run under Cargo; compiling their
executables is not cacheable. Native/Wasm dependencies can be reused independently
by target/profile/features/compiler flags. The engine's final Wasm link still
runs through wasm-ld. Proc macros with undeclared filesystem inputs require care;
this project uses serde/wasm-bindgen derives with normal declared inputs.

CI disables incremental compilation in Cargo test and Browser Wasm jobs; incremental rustc
invocations are not cacheable. Local Compose retains Cargo's default incremental
behavior. sccache 0.10.0 refuses an explicit `CARGO_INCREMENTAL=1` environment;
the local wrapper bypasses sccache in that case. No external cache service,
credentials or new secrets are required.
GitHub cache scope continues to isolate fork/PR writes from the trusted base.
Keys contain the compiler/tool versions, target configuration, manifests/lock,
assets, wrappers and workflow configuration; sccache additionally checks compiler
arguments and input content. A missing cache is an ordinary full build.

The local named volume is mounted at `/var/cache/sccache`, bounded to 2 GiB.
`docker compose up -d --build dev` preserves it; `docker compose down -v` would
remove **all** Compose volumes and is not a selective cache reset. To reset only
compiler entries, stop builds and the server, then clear just this volume:

```sh
docker compose exec -T dev sccache --stop-server
docker compose exec -T dev sh -c 'find /var/cache/sccache -mindepth 1 -maxdepth 1 -exec rm -rf {} +'
```

Override `SCCACHE_CACHE_SIZE` when recreating a container to change its limit.
Use `RUSTC_WRAPPER=` for diagnosis or bypass; do not ignore compiler errors.
On GitHub, delete the affected `sccache-0.10.0-*` cache or bump its prefix to reset.
Stop the server before saving or removing its directory.

`sh scripts/benchmark-sccache.sh` runs in Docker with isolated fixed source,
target and compiler-cache directories. It records no-wrapper, empty-cache,
warm-cache/empty-target, unchanged-target, partial-source, dependency-feature,
incremental and Wasm cases. Dependency change means enabling serde_json's
`raw_value` feature without changing locked versions. Partial edits add whitespace
to `src/lib.rs`; they deliberately leave behavior unchanged. Server and Cargo
GNU-time TSVs are separate: add their CPU, take the greater peak RSS, and use
Cargo wall time. The foreground server includes its rustc children; counting only
the Cargo client would under-report compilation resources. Cache pack/unpack
timings include local gzip I/O, not GitHub network transfer or runner queue time.

### Compiler-cache comparison, 2026-10-25

Same Docker/Rust/GCC host as the linker comparison, mold `--no-fork`, source
`139914091652ab351c7b055db29ae37e5282549e`. Dependencies were downloaded before
measurement. Native uses all test targets/features; Wasm uses release, lib and
no default features. Empty target directories retain the same source path.

| Case | Wall s | CPU s (client + server) | Peak RSS KiB | Rust hits / misses | Non-cacheable calls |
| --- | ---: | ---: | ---: | ---: | ---: |
| Native, no wrapper / empty target | 60.14 | 625.31 | 4,659,748 | — | — |
| Native, empty compiler cache / empty target | 63.86 | 641.59 | 4,706,816 | 0 / 52 | 32 |
| Native, warm compiler cache / empty target | 54.95 | 550.21 | 4,873,840 | 52 / 0 | 32 |
| Native, unchanged warm target | 0.11 | 0.13 | 36,788 | 0 / 0 | 0 |
| Native, partial source / incremental off | 48.90 | 493.21 | 4,583,220 | 0 / 0 | 10 |
| Native, dependency feature changed | 52.02 | 524.14 | 4,723,944 | 0 / 1 | 11 |
| Native, initial incremental build / direct rustc | 58.82 | 808.60 | 3,938,800 | — | — |
| Native, partial source / incremental on | 4.53 | 37.79 | 1,228,908 | — | — |
| Wasm, no wrapper / empty target | 18.38 | 95.66 | 1,266,392 | — | — |
| Wasm, empty compiler cache / empty target | 18.93 | 95.54 | 1,304,904 | 0 / 19 | 18 |
| Wasm, warm compiler cache / empty target | 15.98 | 84.36 | 1,270,860 | 19 / 0 | 18 |

Native non-cacheable reasons were 27 `crate-type`, two `-o`, two missing-input
and one stdin probe calls. Wasm had 14 `crate-type`, two stdin and two missing-input
calls. Dependency build scripts probe unsupported compiler features (including
proc-macro2 span support): their three native/one Wasm failed probe compilations
appear in statistics even though Cargo succeeds. Cache errors/write errors were
zero. The dependency-feature edit produced one new miss, not a stale hit.
Cold/warm/no-wrapper Wasm bytes matched exactly.

The combined compiler cache occupied 73,832,345 bytes; a gzip archive was
73,583,938 bytes, taking 1.96 s to pack and 0.68 s to restore locally. This is
additional storage beside Cargo's target cache, which remains useful for build
scripts/proc macros and avoiding requests altogether. CI retains that cache;
sccache covers eligible misses and uploads only when new entries were written.
These local times do not establish a GitHub runner/network speedup. Local
incremental stays enabled because the partial-edit result is much faster than
turning it off merely to increase cacheability.

For planorama-site, the measured engine-only benefit is 2.40 s before cache/tool
transfer overhead, and the existing verified package and Cargo target caches
already avoid work. No additional engine wrapper is enabled there: external
Cargo config rejection, source/lock checks and manifest keys remain intact.
Its separate binding-tool installation path is evaluated with
`sh scripts/benchmark-binding-cache.sh`; package/tool hits continue to skip it.

That script uses Rust 1.85.1, wasm-bindgen-cli 0.2.100, `--locked --force`, all
three installed executables and GNU ld, matching site's existing tool install.
Registry data is warm; crates.io index checks are included in wall time.

| Binding install | Wall s | CPU s | Peak RSS KiB | Hits / misses (Rust + C/C++) |
| --- | ---: | ---: | ---: | ---: |
| No wrapper, empty target | 25.40 | 274.08 | 618,492 | — |
| Empty compiler cache, empty target | 29.53 | 294.90 | 605,268 | 0 / 169 |
| Warm compiler cache, empty target | 9.23 | 53.63 | 341,036 | 169 / 0 |
| Existing tool version check | <0.01 | <0.01 | 3,124 | Not invoked |

The warm run reused 152 Rust and 17 third-party C/C++ compilations; 53 calls were
non-cacheable (29 crate-type, 13 unknown-language probes, five missing-input,
five stdin and one `-o`). Final executable links still ran. The 94,096,418-byte
cache compressed to 93,742,950 bytes in 2.51 s and restored in 0.89 s. Installed
wasm-bindgen SHA-256 was identical in all three runs; generated JS/Wasm also
matched the preinstalled binding tool. The local 16.17 s warm improvement exceeds
the measured 3.40 s local archive overhead, supporting conditional adoption for
site's binding-tool miss path. Network/CI transfer time still needs observation;
engine/tool hits skip this cache entirely. Site's Docker image already caches
the installed tool in its image layer and does not gain another compiler cache.
