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

CI explicitly runs calibration in the standard Cargo test job and uploads
`spacing-calibration.json` plus old-budget/default/doubled SVGs in the
`spacing-calibration` artifact. Runtime is observational, without timing assertions.

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
