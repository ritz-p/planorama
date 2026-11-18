const {execFileSync} = require('node:child_process');
const fs = require('node:fs');

const repo = process.env.GITHUB_REPOSITORY || 'ritz-p/planorama';
const count = Number(process.argv[2] || 12);
if (!Number.isInteger(count) || count < 1 || count > 50) throw new Error('Expected 1–50 samples');
function get(path) {
  return JSON.parse(execFileSync('gh', ['api', 'repos/' + repo + '/actions/' + path], {maxBuffer: 16 * 1024 * 1024}));
}
const seconds = (start, end) => (Date.parse(end) - Date.parse(start)) / 1000;
const workflows = ['cargo_test.yml', 'wasm.yml', 'cargo_clippy.yml', 'cargo_fmt.yml'];
const runs = workflows.map(file => get('workflows/' + file + '/runs?event=pull_request&per_page=100').workflow_runs);
const samples = [];
const visited = new Set();
const excluded = [];
for (const rust of runs[0]) {
  if (samples.length === count) break;
  const key = [rust.head_repository?.full_name, rust.head_branch, rust.head_sha].join(':');
  if (visited.has(key)) continue;
  visited.add(key);
  const group = runs.map(list => list.find(run => run.head_sha === rust.head_sha && run.head_branch === rust.head_branch
    && run.head_repository?.full_name === rust.head_repository?.full_name));
  if (!group[1] || group.some(run => run && (run.conclusion !== 'success' || run.run_attempt !== 1))) {
    excluded.push(rust.id);
    continue;
  }
  const jobs = [];
  for (const run of group.filter(Boolean)) {
    const response = get('runs/' + run.id + '/jobs?per_page=100');
    if (response.jobs.length !== response.total_count) throw new Error('Incomplete jobs for run ' + run.id);
    for (const job of response.jobs.filter(job => job.conclusion === 'success')) {
      jobs.push({name: job.name, run: run.id, started: job.started_at, finished: job.completed_at,
        waitFromWorkflowStart: seconds(run.run_started_at, job.started_at),
        execution: seconds(job.started_at, job.completed_at),
        steps: Object.fromEntries(job.steps.filter(step => step.conclusion === 'success')
          .map(step => [step.name, seconds(step.started_at, step.completed_at)]))});
    }
  }
  const names = jobs.map(job => job.name);
  if (!['Tests', 'Test suite (standard)', 'Test suite (large)', 'browser'].every(name => names.includes(name))) {
    excluded.push(rust.id);
    continue;
  }
  const start = group.filter(Boolean).map(run => run.run_started_at).sort()[0];
  const finish = jobs.map(job => job.finished).sort().at(-1);
  samples.push({sha: rust.head_sha, elapsed: seconds(start, finish), jobs});
}
if (!samples.length) throw new Error('No complete normal PR validations');
function quantile(values, p) {
  return [...values].sort((a, b) => a - b)[Math.ceil(values.length * p) - 1];
}
const elapsed = samples.map(sample => sample.elapsed);
const p50 = quantile(elapsed, 0.5), p95 = quantile(elapsed, 0.95);
const summary = samples.length + ' successful PR revisions: p50 ' + p50 + 's, p95 ' + p95 + 's (target 600s).';
console.error(summary);
if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary + '\n');
if (samples.length >= 3 && samples.slice(0, 3).every(sample => sample.elapsed > 600)) {
  console.error('::warning::The latest three complete PR validations exceeded 10 minutes.');
}
console.log(JSON.stringify({unit: 'seconds', p50, p95, excluded, samples}, null, 2));
