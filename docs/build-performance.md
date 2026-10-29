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

Outside Docker, run `sudo sh scripts/install-mold.sh`, then set
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$PWD/scripts/native-linker.sh"`
for Cargo. To disable mold in Compose, run
`docker compose exec -T -e CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=cc dev cargo test --locked`.
Changing the Cargo linker setting triggers rebuilding.
`PLANORAMA_LINKER=system` selects `cc` inside the wrapper for controlled
measurements; clean the benchmark target when changing this variable because
Cargo cannot detect changes inside a linker wrapper. For normal use, override
the Cargo linker variable to `cc`, which Cargo fingerprints.

Run `docker compose exec -T dev sh scripts/benchmark-linker.sh` from the repository
root. The script uses an isolated `/tmp/planorama-linker-benchmark-target`, cleans
it between linkers and at completion, disables incremental compilation and compiler
wrappers, and retains measurements under `output/linker-benchmark`.
The script copies tracked source into a temporary directory without Git metadata,
isolating it from working-tree edits and generated reports. Build provenance is
unavailable in that copy; the current build script still runs on every build.
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
