# Spacing budget calibration

Defaults are 20 sweeps, `clamp(path_count, 60, 240)` attempts and
`clamp(initial_axis_size / 4, 640, 4096)` pixels of cumulative growth on each
axis. Minimum search and growth allowances increase by at least ten times.
Successful transactions rebuild demand; no-improvement sweeps stop. The
[routing policy](../../docs/routing.md) describes the immutable geometry reference
and whole-transaction validation.

`budget_calibration` compares the old numerical limits (2/6/64px), defaults, and
doubled limits on the same unspaced geometry using the new refinement algorithm.
This isolates budget sizing; the first row does not reproduce the old scheduler.
Fixtures are `terraform-large`, seven separate seven-lane corridors, and a
protected-junction case with no movable lanes. Records include sweeps, attempts,
accepted expansions, reconsiderations, dimensions, parallel deficit,
minimum/preferred border deficits, refinement time and every edge's length,
bends, offsets, reversals, backtracking and S-jogs.

The dense fixture validates seven expansions, eight sweeps and 756px cumulative
growth separately in both axes. Ten lanes accept more than 128px of necessary
endpoint-distance increase while retaining two bends per path; residual deficits
remain informational. Container fixtures check coherent container, scope and band
dimensions beyond 64px. Another fixture rejects a corridor, expands another gap,
and accepts the unchanged old corridor key on reconsideration. Protected geometry
exits after one sweep without attempts.

The manual **Spacing budget calibration** workflow runs calibration and uploads
`spacing-calibration.json` plus old-budget/default/doubled SVGs in the
`spacing-calibration-<SHA>` artifact. It is excluded from the standard PR job;
focused routing correctness and layout regressions remain required. Three
large-fixture comparisons use separate [post-merge/nightly coverage](../../docs/ci.md#deferred-routing-regressions-253).
See [dispatch and revision selection](../../docs/ci.md#on-demand-spacing-calibration).
Runtime is observational, without timing assertions.

```sh
docker compose exec -T -e PLANORAMA_SPACING_REPORT_DIR=output/spacing-calibration dev cargo test --locked --bin planorama budget_calibration -- --ignored --nocapture
```

The raw-polyline dense DAG regression changes from 394 to 396 crossings while
retaining 1px overlap, 87 bends and 71,184px length. Crossings can increase while
clearance improves; the pass does not optimize all metrics simultaneously.

## Local calibration

Measured 2026-10-14 with Rust 1.85.1 in Docker Desktop Linux, optimized test
profile, one observation per candidate while other validation was running.
Times cover final refinement only; compilation, initial placement/routing and
SVG composition are excluded. Border deficits in this internal calibration use
quarter-pixel × px; divide by four for the public report's px² convention.

| Fixture / budget | Sweeps / attempts | Accepted / expansions | Width × height | Parallel deficit | Minimum / preferred border deficit | Refinement ms |
| --- | ---: | ---: | --- | ---: | ---: | ---: |
| terraform-large / old limits | 2 / 2 | 2 / 0 | 4560 × 2407 | 31987 | 53 / 481 | 7656.9 |
| terraform-large / defaults | 13 / 47 | 12 / 0 | 4560 × 2407 | 13427 | 12 / 155 | 17294.0 |
| terraform-large / doubled | 13 / 47 | 12 / 0 | 4560 × 2407 | 13427 | 12 / 155 | 16512.3 |
| dense / old limits | 2 / 2 | 2 / 0 | 9800 × 3000 | 1823400 | 27684 / 185458 | 1836.3 |
| dense / defaults | 8 / 7 | 7 / 7 | 10556 × 3000 | 0 | 0 / 0 | 5766.6 |
| dense / doubled | 8 / 7 | 7 / 7 | 10556 × 3000 | 0 | 0 / 0 | 5878.3 |
| constrained / all candidates | 1 / 0 | 0 / 0 | 1100 × 2200 | 196000 | 4424 / 26726 | 0.08–0.11 |

The defaults reduce the large fixture's parallel deficit by about 58% and its
minimum border deficit by about 77% relative to the small-limit control, at about
2.3 times the refinement time in this observation. The large fixture needs no
accepted growth; the dense fixture demonstrates the additional cumulative room.
Doubling the candidate budget produced identical geometry and residual deficits,
so the smaller candidate was selected. This is calibration on representative
fixtures, not a claim of globally optimal budgets.

Large-fixture total route length changes from 40,106 to 39,178px, bends from 83
to 81, reversals from 6 to 4 and backtracking from 609 to 145px. Both candidates
retain two offset-shaped routes and zero S-jogs. Dense routes grow from 88,200
to 93,492px through necessary endpoint separation, with 98 bends and no offsets,
reversals or S-jogs in all candidates. Constrained geometry stays identical.
The full and enlarged large/dense SVG comparisons were inspected locally;
residual crowded runs remain in the large diagram and are recorded as deficits.

## CI calibration

[GitHub Actions run 38539978228](https://github.com/ritz-p/planorama/actions/runs/38539978228)
measured the same candidates on 2026-10-14 with the pinned Rust 1.85.1 toolchain
on `ubuntu-latest`. All sweeps, attempts, accepted changes, expansions, dimensions,
deficits and per-edge shape/length measurements match the local results above.
The run's `spacing-calibration` artifact contains the full JSON and nine SVGs.

| Fixture | Old-limit refinement ms | Default refinement ms | Doubled refinement ms |
| --- | ---: | ---: | ---: |
| terraform-large | 7798.1 | 20109.5 | 20098.2 |
| dense corridors | 2411.9 | 7941.0 | 7949.1 |
| constrained | 0.110 | 0.097 | 0.094 |

The selected defaults cost about 2.6 times the large-fixture refinement time and
3.3 times the dense-fixture time relative to the small-limit control in this CI
observation. They retain the measured clearance improvements without consuming
all available work. Doubling the limits adds no improvement on these fixtures,
confirming the smaller candidate as the default; constrained input still exits
immediately. Timings are single observations, not performance guarantees.
