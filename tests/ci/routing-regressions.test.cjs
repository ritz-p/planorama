const assert = require('node:assert/strict');
const {readFileSync, readdirSync} = require('node:fs');
const path = require('node:path');
const {test} = require('node:test');

const root = path.join(__dirname, '../..');
const read = file => readFileSync(path.join(root, file), 'utf8');

function rustFiles(directory) {
  return readdirSync(path.join(root, directory), {withFileTypes: true}).flatMap(entry => {
    const file = `${directory}/${entry.name}`;
    return entry.isDirectory() ? rustFiles(file) : file.endsWith('.rs') ? [file] : [];
  });
}

test('deferred filter selects only the three classified ignored regressions', () => {
  const selected = rustFiles('tests').flatMap(file => [...read(file).matchAll(
    /#\[test\]\s*(#\[ignore[^\n]*\]\s*)?fn (deferred_regression_\w+)\(/g,
  )].map(match => {
    assert.match(match[1] || '', /deferred regression:/, file);
    return match[2];
  }));
  assert.deepEqual(selected.sort(), [
    'deferred_regression_congested_container_routes_improve_over_shortest_paths',
    'deferred_regression_large_fixture_limits_each_edges_bends_and_valid_route_stretch',
    'deferred_regression_large_listener_locality_preserves_geometry_and_routes',
  ]);
});

test('deferred regressions run after merge and nightly without executing PR heads or manual benchmarks', () => {
  const workflow = read('.github/workflows/routing_regressions.yml');
  assert.match(workflow, /pull_request_target:\s+types: \[closed\]\s+branches: \[main\]/);
  assert.match(workflow, /schedule:\s+- cron:/);
  assert.match(workflow, /workflow_dispatch:/);
  assert.match(workflow, /github.event.pull_request.merged == true/);
  assert.match(workflow, /contents: read/);
  assert.match(workflow, /persist-credentials: false/);
  assert.doesNotMatch(workflow, /ref:|: write|continue-on-error|pull_request.head|--include-ignored/);
  assert.match(workflow, /cargo test --locked --all-features --bin planorama deferred_regression_ -- --ignored --test-threads=2 --nocapture/);
  assert.match(workflow, /set -o pipefail/);
  assert.match(workflow, /if: \$\{\{ !cancelled\(\) \}\}/);
  assert.match(workflow, /path: routing-regressions.log/);
  assert.doesNotMatch(read('.github/workflows/cargo_test.yml'), /routing_regressions|deferred_regression_|--ignored/);
});
