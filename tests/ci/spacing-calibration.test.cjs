const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const path = require('node:path');
const {test} = require('node:test');

const read = file => readFileSync(path.join(__dirname, '../..', file), 'utf8');
const standard = read('.github/workflows/cargo_test.yml');
const manual = read('.github/workflows/spacing_calibration.yml');

test('ordinary suites retain selectors and aggregate without calibration or artifact prerequisite', () => {
  assert.doesNotMatch(standard, /budget_calibration|spacing-calibration|upload-artifact|--ignored/);
  assert.match(standard, /suite: \[standard, large\]/);
  assert.match(standard, /node .github\/scripts\/test-suites.cjs "\$TEST_SUITE" compile/);
  assert.match(standard, /node .github\/scripts\/test-suites.cjs "\$TEST_SUITE" run/);
  assert.match(standard, /needs: \[test-suites\]/);
  assert.match(standard, /run: test "\$SUITES_RESULT" = success/);
});

test('calibration is opt-in, read-only and bound to a validated immutable source revision', () => {
  assert.match(manual, /workflow_dispatch:/);
  assert.doesNotMatch(manual, /^  (pull_request|push|pull_request_target|schedule):/m);
  assert.match(manual, /permissions:\n  contents: read/);
  assert.doesNotMatch(manual, /: write|secrets\.|git push|update-examples/);
  assert.match(manual, /SOURCE_SHA: \$\{\{ inputs.source_sha \|\| github.sha \}\}/);
  assert.match(manual, /\^\[0-9a-f\]\{40\}\$/);
  assert.match(manual, /ref: \$\{\{ env.SOURCE_SHA \}\}/);
  assert.match(manual, /persist-credentials: false/);
  assert.match(manual, /test "\$\(git rev-parse HEAD\)" = "\$SOURCE_SHA"/);
  assert.match(manual, /rustc --version --verbose/);
  assert.match(read('rust-toolchain.toml'), /channel = "1\.85\.1"/);
});

test('manual calibration preserves the ignored experiment and uploads complete revision-labelled output', () => {
  assert.match(manual, /cargo test --locked --lib budget_calibration --no-run/);
  assert.match(manual, /cargo test --locked --lib budget_calibration -- --ignored --nocapture/);
  assert.match(manual, /PLANORAMA_SPACING_REPORT_DIR: output\/spacing-calibration/);
  assert.match(manual, /name: spacing-calibration-\$\{\{ env.SOURCE_SHA \}\}/);
  assert.match(manual, /path: output\/spacing-calibration/);
  assert.match(manual, /if-no-files-found: error/);
  assert.match(read('tests/unit/layout/routing_shared/spacing_calibration.rs'), /#\[ignore[^\n]*\]\s*fn budget_calibration/);
  assert.doesNotMatch(read('.github/workflows/update_examples.yml'), /spacing_calibration\.yml/);
});
