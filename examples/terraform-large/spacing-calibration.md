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
docker compose exec -T -e PLANORAMA_SPACING_REPORT_DIR=output/spacing-calibration dev cargo test --locked --lib budget_calibration -- --ignored --nocapture
```


The default budget is deliberately bounded: doubling it did not improve the
representative calibration fixtures. Recalibrate when workloads or refinement
change; do not infer optimality or a runtime guarantee from this choice.
