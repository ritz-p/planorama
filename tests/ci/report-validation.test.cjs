const assert = require('node:assert/strict');
const {test} = require('node:test');
const reportValidation = require('../../.github/scripts/report-validation.cjs');

const SHA = 'a'.repeat(40);
const NEW_SHA = 'b'.repeat(40);
const REPOSITORY = {id: 88, full_name: 'ritz-p/planorama'};
const DEFINITIONS = [
  ['cargo_fmt.yml', 'Cargo fmt', 'Formatting', ['Check formatting']],
  ['cargo_clippy.yml', 'Cargo clippy', 'Clippy', ['Run Clippy']],
  ['cargo_test.yml', 'Cargo test', 'Tests', ['Compile tests', 'Run tests']],
  ['cargo_msrv.yml', 'Cargo msrv', 'MSRV', ['Build MSRV image', 'Run MSRV tests']],
  ['workflow_tests.yml', 'Workflow tests', 'Workflow tests', ['Lint workflows', 'Test validation reporting']],
];

function setup(file = 'cargo_test.yml') {
  const [, name, status, commands] = DEFINITIONS.find(definition => definition[0] === file);
  const run = {
    id: 101, workflow_id: 22, name, path: `.github/workflows/${file}`,
    event: 'workflow_dispatch', display_title: `Validate PR #330 at ${SHA}`,
    head_sha: SHA, head_branch: 'feature/issue-330', run_number: 1, run_attempt: 1,
    repository: {...REPOSITORY}, head_repository: {...REPOSITORY}, status: 'completed', conclusion: 'success',
  };
  const job = {
    id: 201, name: `Dispatched ${status}`, status: 'completed', conclusion: 'success',
    steps: ['Checkout dispatched PR merge', ...commands].map(step => ({
      name: step, status: 'completed', conclusion: 'success',
    })),
  };
  const environment = {
    run, job, runs: [run], jobs: new Map([['101/1', [job]]]), statuses: [], calls: [], writes: [], messages: [],
    workflow: {id: 22, name, path: `.github/workflows/${file}`},
    pull: {
      number: 330, state: 'open',
      head: {sha: SHA, ref: 'feature/issue-330', repo: {...REPOSITORY}},
      base: {sha: 'c'.repeat(40), ref: 'main', repo: {...REPOSITORY}},
    },
    context: {
      eventName: 'workflow_run', repo: {owner: 'ritz-p', repo: 'planorama'}, serverUrl: 'https://github.com',
      payload: {repository: {...REPOSITORY}, workflow_run: structuredClone(run)},
    },
  };
  const response = (method, read) => async parameters => {
    environment.calls.push({method, parameters: structuredClone(parameters)});
    if (environment.onCall) await environment.onCall(method, parameters);
    return {data: structuredClone(read(parameters))};
  };
  const paginate = (items, parameters) => items.slice((parameters.page - 1) * parameters.per_page, parameters.page * parameters.per_page);
  const github = {rest: {
    actions: {
      getWorkflowRun: response('getWorkflowRun', parameters => {
        const found = environment.runs.find(candidate => candidate.id === parameters.run_id);
        assert.ok(found, `Unexpected run ${parameters.run_id}`);
        return found;
      }),
      getWorkflow: response('getWorkflow', parameters => {
        assert.equal(parameters.workflow_id, file);
        return environment.workflow;
      }),
      listWorkflowRuns: response('listWorkflowRuns', parameters => {
        const runs = (environment.historyRuns || environment.runs).filter(candidate => candidate.workflow_id === parameters.workflow_id
          && candidate.head_sha === parameters.head_sha && candidate.head_branch === parameters.branch
          && candidate.event === parameters.event);
        return {workflow_runs: paginate(runs, parameters), total_count: runs.length};
      }),
      listJobsForWorkflowRunAttempt: response('listJobsForWorkflowRunAttempt', parameters => ({
        jobs: paginate(environment.jobs.get(`${parameters.run_id}/${parameters.attempt_number}`) || [], parameters),
      })),
    },
    pulls: {get: response('getPull', () => environment.pull)},
    repos: {
      listCommitStatusesForRef: response('listCommitStatusesForRef', parameters => paginate(environment.statuses, parameters)),
      createCommitStatus: response('createCommitStatus', parameters => {
        const created = {...parameters, id: environment.writes.length + 1, creator: {login: 'github-actions[bot]'}};
        environment.writes.push(structuredClone(parameters));
        environment.statuses.unshift(created);
        return created;
      }),
    },
  }};
  environment.invoke = () => reportValidation({
    github, context: environment.context, core: {info: message => environment.messages.push(message)},
  });
  return environment;
}

for (const [file, , status] of DEFINITIONS) {
  test(`${status} publishes a successful merge validation on the exact generated SHA`, async () => {
    const environment = setup(file);
    const result = await environment.invoke();
    assert.equal(result.state, 'success');
    assert.deepEqual(environment.writes, [{
      owner: 'ritz-p', repo: 'planorama', sha: SHA, state: 'success', context: status,
      description: `${status} passed (attempt 1).`,
      target_url: 'https://github.com/ritz-p/planorama/actions/runs/101/attempts/1',
    }]);
  });
}

test('reports an actual assertion failure instead of trusting the old success event', async () => {
  const environment = setup();
  environment.run.conclusion = 'failure';
  environment.job.conclusion = 'failure';
  environment.job.steps.at(-1).conclusion = 'failure';
  assert.equal((await environment.invoke()).state, 'failure');
  assert.equal(environment.writes[0].context, 'Tests');
});

test('a failed required step cannot be hidden by continue-on-error', async () => {
  const environment = setup();
  environment.job.steps.at(-1).conclusion = 'failure';
  assert.equal((await environment.invoke()).state, 'failure');
});

for (const [description, change] of [
  ['skipped validation job', environment => { environment.job.conclusion = 'skipped'; }],
  ['skipped merge checkout', environment => { environment.job.steps[0].conclusion = 'skipped'; }],
  ['skipped cargo command', environment => { environment.job.steps.at(-1).conclusion = 'skipped'; }],
  ['missing required step', environment => { environment.job.steps.pop(); }],
  ['duplicate required step', environment => { environment.job.steps.push(structuredClone(environment.job.steps[0])); }],
  ['unexpected job name', environment => { environment.job.name = 'Tests'; }],
]) {
  test(`${description} is an error even when the workflow conclusion is success`, async () => {
    const environment = setup();
    change(environment);
    assert.equal((await environment.invoke()).state, 'error');
  });
}

for (const conclusion of ['cancelled', 'timed_out', 'neutral', 'skipped', 'startup_failure', 'action_required']) {
  test(`${conclusion} is a visible non-success result`, async () => {
    const environment = setup();
    environment.run.conclusion = conclusion;
    assert.equal((await environment.invoke()).state, 'error');
    assert.match(environment.writes[0].description, /retry validation/);
  });
}

for (const status of ['queued', 'in_progress']) {
  test(`${status} publishes pending without inspecting stale completed jobs`, async () => {
    const environment = setup();
    environment.run.status = status;
    environment.run.conclusion = null;
    assert.equal((await environment.invoke()).state, 'pending');
    assert.ok(!environment.calls.some(call => call.method === 'listJobsForWorkflowRunAttempt'));
  });
}

for (const [description, change] of [
  ['generic manual run', environment => { environment.run.display_title = 'Cargo test'; }],
  ['different SHA in the inputs', environment => { environment.run.display_title = `Validate PR #330 at ${NEW_SHA}`; }],
  ['unsafe PR number', environment => { environment.run.display_title = `Validate PR #9007199254740992 at ${SHA}`; }],
  ['unknown workflow path', environment => { environment.run.path = '.github/workflows/unrelated.yml'; }],
  ['different registered workflow ID', environment => { environment.workflow.id = 23; }],
  ['fork run', environment => { environment.run.head_repository = {id: 99, full_name: 'someone/planorama'}; }],
  ['different origin repository', environment => { environment.run.repository.id = 99; }],
  ['ordinary PR workflow event', environment => { environment.context.payload.workflow_run.event = 'pull_request'; }],
  ['closed PR', environment => { environment.pull.state = 'closed'; }],
  ['advanced PR head', environment => { environment.pull.head.sha = NEW_SHA; }],
  ['different source branch', environment => { environment.pull.head.ref = 'feature/another-pr'; }],
  ['fork PR', environment => { environment.pull.head.repo = {id: 99, full_name: 'someone/planorama'}; }],
]) {
  test(`${description} cannot publish a PR status`, async () => {
    const environment = setup();
    change(environment);
    assert.equal(await environment.invoke(), null);
    assert.equal(environment.writes.length, 0);
  });
}

test('a late success event reports the newer eligible failing run', async () => {
  const environment = setup();
  environment.runs.push({...structuredClone(environment.run), id: 102, run_number: 2, conclusion: 'failure'});
  environment.jobs.set('102/1', [{...structuredClone(environment.job), conclusion: 'failure'}]);
  const result = await environment.invoke();
  assert.equal(result.state, 'failure');
  assert.match(result.target_url, /runs\/102\/attempts\/1$/);
  assert.ok(environment.calls.filter(call => call.method === 'listJobsForWorkflowRunAttempt')
    .every(call => call.parameters.run_id === 102));
});

test('a newer unbound manual run cannot replace merge validation', async () => {
  const environment = setup();
  environment.runs.push({...structuredClone(environment.run), id: 102, run_number: 2, display_title: 'Cargo test'});
  const result = await environment.invoke();
  assert.equal(result.state, 'success');
  assert.match(result.target_url, /runs\/101\//);
});

test('a stale history listing cannot overwrite a newer published result', async () => {
  const environment = setup();
  environment.runs.push({...structuredClone(environment.run), id: 102, run_number: 2, conclusion: 'failure'});
  environment.jobs.set('102/1', [{...structuredClone(environment.job), conclusion: 'failure'}]);
  await environment.invoke();
  environment.historyRuns = [environment.run];
  const result = await environment.invoke();
  assert.equal(result.state, 'failure');
  assert.equal(result.published, false);
  assert.equal(environment.writes.length, 1);
});

test('finds the newest eligible run beyond the first history page', async () => {
  const environment = setup();
  for (let index = 0; index < 99; index++) {
    environment.runs.push({...structuredClone(environment.run),
      id: 1000 + index, run_number: index + 2, display_title: 'Cargo test'});
  }
  environment.runs.push({...structuredClone(environment.run), id: 2000, run_number: 101});
  environment.jobs.set('2000/1', [structuredClone(environment.job)]);
  const result = await environment.invoke();
  assert.match(result.target_url, /runs\/2000\//);
  assert.ok(environment.calls.some(call => call.method === 'listWorkflowRuns' && call.parameters.page === 2));
});

test('recomputes before publishing when a newer run starts during reporting', async () => {
  const environment = setup();
  let historyReads = 0;
  environment.onCall = method => {
    if (method === 'listWorkflowRuns' && ++historyReads === 2) {
      environment.runs.push({...structuredClone(environment.run),
        id: 102, run_number: 2, status: 'in_progress', conclusion: null});
    }
  };
  const result = await environment.invoke();
  assert.equal(result.state, 'pending');
  assert.match(result.target_url, /runs\/102\//);
  assert.equal(environment.writes.length, 1);
});

test('a rerun supersedes old attempt results and uses the exact new attempt jobs', async () => {
  const environment = setup();
  environment.job.conclusion = 'failure';
  environment.context.payload.workflow_run.conclusion = 'failure';
  environment.run.run_attempt = 2;
  environment.jobs.set('101/2', [{...structuredClone(environment.job), conclusion: 'success'}]);
  const result = await environment.invoke();
  assert.equal(result.state, 'success');
  assert.match(result.target_url, /attempts\/2$/);
  assert.ok(environment.calls.filter(call => call.method === 'listJobsForWorkflowRunAttempt')
    .every(call => call.parameters.attempt_number === 2));
});

test('a rerun that begins during reporting resets the status to pending', async () => {
  const environment = setup();
  let historyReads = 0;
  environment.onCall = method => {
    if (method === 'listWorkflowRuns' && ++historyReads === 2) {
      Object.assign(environment.run, {run_attempt: 2, status: 'queued', conclusion: null});
    }
  };
  const result = await environment.invoke();
  assert.equal(result.state, 'pending');
  assert.match(result.target_url, /attempts\/2$/);
  assert.equal(environment.writes.length, 1);
});

test('an older attempt API response cannot override a newer known attempt', async () => {
  const environment = setup();
  environment.historyRuns = [{...structuredClone(environment.run), run_attempt: 2, status: 'queued', conclusion: null}];
  await assert.rejects(environment.invoke(), /Validation kept changing/);
  assert.equal(environment.writes.length, 0);
});

test('a human push before publication prevents reporting on the new head', async () => {
  const environment = setup();
  let pullReads = 0;
  environment.onCall = method => {
    if (method === 'getPull' && ++pullReads === 2) environment.pull.head.sha = NEW_SHA;
  };
  assert.equal(await environment.invoke(), null);
  assert.equal(environment.writes.length, 0);
});

test('deduplicates the latest identical Actions status', async () => {
  const environment = setup();
  assert.equal((await environment.invoke()).published, true);
  assert.equal((await environment.invoke()).published, false);
  assert.equal(environment.writes.length, 1);
});

for (const suffix of ['', '/attempts/1']) {
  test(`replaces bootstrap pending status linked to the SVG workflow${suffix}`, async () => {
    const environment = setup();
    environment.runs.push({...structuredClone(environment.run),
      id: 900, workflow_id: 33, name: 'Update SVG examples', path: '.github/workflows/update_examples.yml',
      event: 'pull_request', display_title: 'Generate SVGs'});
    environment.statuses.push({
      context: 'Tests', state: 'pending', description: 'Waiting for validation to start',
      target_url: `https://github.com/ritz-p/planorama/actions/runs/900${suffix}`,
      creator: {login: 'github-actions[bot]'},
    });
    assert.equal((await environment.invoke()).state, 'success');
    assert.equal(environment.writes.length, 1);
  });
}

test('does not reuse identical status text published by a different identity', async () => {
  const environment = setup();
  await environment.invoke();
  environment.statuses[0].creator.login = 'another-publisher';
  assert.equal((await environment.invoke()).published, true);
  assert.equal(environment.writes.length, 2);
});

test('an API failure cannot turn unverified jobs into a successful status', async () => {
  const environment = setup();
  environment.onCall = method => {
    if (method === 'listJobsForWorkflowRunAttempt') throw new Error('GitHub API unavailable');
  };
  await assert.rejects(environment.invoke(), /GitHub API unavailable/);
  assert.equal(environment.writes.length, 0);
});
