const WORKFLOWS = new Map([
  ['cargo_fmt.yml', {name: 'Cargo fmt', status: 'Formatting', steps: ['Check formatting']}],
  ['cargo_clippy.yml', {name: 'Cargo clippy', status: 'Clippy', steps: ['Run Clippy']}],
  ['cargo_test.yml', {name: 'Cargo test', status: 'Tests', steps: ['Compile tests', 'Run tests']}],
  ['cargo_msrv.yml', {name: 'Cargo msrv', status: 'MSRV', steps: ['Build MSRV image', 'Run MSRV tests']}],
  ['workflow_tests.yml', {
    name: 'Workflow tests', status: 'Workflow tests', steps: ['Lint workflows', 'Test validation reporting'],
  }],
].map(([file, configuration]) => [`.github/workflows/${file}`, {file, ...configuration}]));

const positiveInteger = value => Number.isSafeInteger(value) && value > 0;
const workflowPath = run => typeof run.path === 'string' ? run.path.split('@', 1)[0] : '';
const newer = (candidate, previous) => candidate.run_number > previous.run_number
  || (candidate.run_number === previous.run_number && candidate.run_attempt > previous.run_attempt);

function binding(run) {
  const match = /^Validate PR #([1-9][0-9]*) at ([0-9a-fA-F]{40})$/.exec(run.display_title || '');
  if (!match || !positiveInteger(Number(match[1]))) return null;
  return {number: Number(match[1]), sha: match[2].toLowerCase()};
}

function version(run) {
  return JSON.stringify([
    run.id, run.run_number, run.run_attempt, run.status, run.conclusion,
    run.workflow_id, run.path, run.head_branch, run.head_sha, run.display_title,
    run.repository?.id, run.repository?.full_name, run.head_repository?.id, run.head_repository?.full_name,
  ]);
}

module.exports = async function reportValidation({github, context, core}) {
  const skip = reason => {
    core.info(reason);
    return null;
  };
  const event = context.payload.workflow_run;
  if (context.eventName !== 'workflow_run' || event?.event !== 'workflow_dispatch' || !positiveInteger(event.id)) {
    return skip('This event is not a dispatched validation run.');
  }
  const repository = context.payload.repository;
  const fullName = `${context.repo.owner}/${context.repo.repo}`;
  if (!positiveInteger(repository?.id) || repository.full_name !== fullName) {
    throw new Error('The event does not identify the trusted reporter repository.');
  }
  const sameRepository = candidate => candidate?.id === repository.id && candidate.full_name === fullName;
  const runsUrl = `${context.serverUrl || 'https://github.com'}/${fullName}/actions/runs/`;
  const repositoryParameters = context.repo;
  const getRun = async id => (await github.rest.actions.getWorkflowRun({
    ...repositoryParameters, run_id: id,
  })).data;
  const initial = await getRun(event.id);
  const path = workflowPath(initial);
  const configuration = WORKFLOWS.get(path);
  const expected = binding(initial);
  if (initial.id !== event.id || !configuration || !expected || initial.head_sha !== expected.sha
      || initial.event !== 'workflow_dispatch' || !sameRepository(initial.repository)
      || !sameRepository(initial.head_repository) || typeof initial.head_branch !== 'string' || !initial.head_branch) {
    return skip('The run is not an approved same-repository PR validation.');
  }
  const {data: workflow} = await github.rest.actions.getWorkflow({
    ...repositoryParameters, workflow_id: configuration.file,
  });
  if (!positiveInteger(workflow.id) || initial.workflow_id !== workflow.id
      || workflow.path !== path || workflow.name !== configuration.name) {
    return skip('The run does not belong to the approved validation workflow.');
  }
  const eligible = run => {
    const identity = binding(run);
    return positiveInteger(run.id) && positiveInteger(run.run_number) && positiveInteger(run.run_attempt)
      && run.workflow_id === workflow.id && workflowPath(run) === path
      && run.event === 'workflow_dispatch' && run.head_sha === expected.sha
      && run.head_branch === initial.head_branch && sameRepository(run.repository) && sameRepository(run.head_repository)
      && identity?.number === expected.number && identity.sha === expected.sha;
  };
  if (!eligible(initial)) return skip('The validation run has incomplete identity or attempt metadata.');
  const getPull = async () => (await github.rest.pulls.get({
    ...repositoryParameters, pull_number: expected.number,
  })).data;
  const currentPull = pull => pull.number === expected.number && pull.state === 'open'
    && sameRepository(pull.head?.repo) && sameRepository(pull.base?.repo)
    && pull.head.sha === expected.sha && pull.head.ref === initial.head_branch;
  if (!currentPull(await getPull())) return skip('The PR is closed, read-only, or has a newer head.');

  async function latestRun(seed) {
    let latest = seed;
    for (let page = 1; page <= 10; page++) {
      const {data} = await github.rest.actions.listWorkflowRuns({
        ...repositoryParameters, workflow_id: workflow.id, head_sha: expected.sha,
        branch: initial.head_branch, event: 'workflow_dispatch', per_page: 100, page,
      });
      if (!Array.isArray(data.workflow_runs)) throw new Error('Cannot inspect validation run history.');
      for (const run of data.workflow_runs) {
        if (eligible(run) && newer(run, latest)) {
          latest = run;
        }
      }
      if (data.workflow_runs.length < 100) return latest;
    }
    throw new Error('Validation run history is incomplete; retry the status reporter.');
  }

  async function jobsForAttempt(run) {
    const jobs = [];
    for (let page = 1; page <= 10; page++) {
      const {data} = await github.rest.actions.listJobsForWorkflowRunAttempt({
        ...repositoryParameters, run_id: run.id, attempt_number: run.run_attempt, per_page: 100, page,
      });
      if (!Array.isArray(data.jobs)) throw new Error('Cannot inspect validation jobs.');
      jobs.push(...data.jobs);
      if (data.jobs.length < 100) return jobs;
    }
    throw new Error('Validation job history is incomplete; retry the status reporter.');
  }

  async function resultFor(run) {
    const attempt = `attempt ${run.run_attempt}`;
    if (run.status !== 'completed') {
      return {state: 'pending', description: `${configuration.status} is ${run.status} (${attempt}).`};
    }
    if (!['success', 'failure'].includes(run.conclusion)) {
      return {state: 'error', description: `${configuration.status}: ${run.conclusion || 'no conclusion'} (${attempt}); retry validation.`};
    }
    const jobs = (await jobsForAttempt(run)).filter(job => job.name === `Dispatched ${configuration.status}`);
    if (jobs.length !== 1) {
      return {state: 'error', description: `${configuration.status}: expected validation job is missing or ambiguous; retry validation.`};
    }
    const job = jobs[0];
    const required = ['Checkout dispatched PR merge', ...configuration.steps];
    const steps = Array.isArray(job.steps) ? job.steps : [];
    if (job.conclusion === 'failure' || steps.some(step => required.includes(step.name) && step.conclusion === 'failure')) {
      return {state: 'failure', description: `${configuration.status} failed (${attempt}); see validation logs.`};
    }
    if (run.conclusion !== 'success' || job.status !== 'completed' || job.conclusion !== 'success'
        || required.some(name => {
          const matches = steps.filter(step => step.name === name);
          return matches.length !== 1 || matches[0].status !== 'completed' || matches[0].conclusion !== 'success';
        })) {
      return {state: 'error', description: `${configuration.status}: required validation did not complete (${attempt}); retry validation.`};
    }
    return {state: 'success', description: `${configuration.status} passed (${attempt}).`};
  }

  async function latestStatus() {
    for (let page = 1; page <= 10; page++) {
      const {data} = await github.rest.repos.listCommitStatusesForRef({
        ...repositoryParameters, ref: expected.sha, per_page: 100, page,
      });
      if (!Array.isArray(data)) throw new Error('Cannot inspect existing commit statuses.');
      const status = data.find(item => item.context?.toLowerCase() === configuration.status.toLowerCase());
      if (status) return status;
      if (data.length < 100) return null;
    }
    throw new Error('Commit status history is incomplete; retry the status reporter.');
  }

  async function publishedRun(status) {
    if (status?.creator?.login !== 'github-actions[bot]' || !status.target_url?.startsWith(runsUrl)) return null;
    const match = /^([1-9][0-9]*)\/attempts\/([1-9][0-9]*)$/.exec(status.target_url.slice(runsUrl.length));
    if (!match || !positiveInteger(Number(match[1])) || !positiveInteger(Number(match[2]))) return null;
    const run = await getRun(Number(match[1]));
    if (run.id !== Number(match[1]) || !eligible(run)) return null;
    if (run.run_attempt < Number(match[2])) {
      throw new Error('Previously published validation is newer than the run API response; retry the status reporter.');
    }
    return run;
  }

  let seed = initial;
  for (let attempt = 0; attempt < 3; attempt++) {
    const selected = await latestRun(seed);
    const run = await getRun(selected.id);
    if (run.id !== selected.id || !eligible(run)) return skip('The selected run no longer matches this validation.');
    if (run.run_attempt < selected.run_attempt) {
      seed = selected;
      continue;
    }
    const result = await resultFor(run);
    const status = {
      ...result, context: configuration.status,
      target_url: `${runsUrl}${run.id}/attempts/${run.run_attempt}`,
    };
    const previous = await latestStatus();
    const published = await publishedRun(previous);
    const newest = await latestRun(published && newer(published, run) ? published : run);
    const fresh = await getRun(newest.id);
    if (fresh.id !== newest.id || !eligible(fresh)) return skip('The latest run no longer matches this validation.');
    if (fresh.run_attempt < newest.run_attempt) {
      seed = newest;
      continue;
    }
    if (version(fresh) !== version(run)) {
      seed = fresh;
      continue;
    }
    if (!currentPull(await getPull())) return skip('The PR advanced or closed before status publication.');
    if (previous?.creator?.login === 'github-actions[bot]'
        && Object.entries(status).every(([key, value]) => previous[key] === value)) {
      core.info(`${configuration.status} already reports ${status.state} for ${expected.sha}.`);
      return {...status, sha: expected.sha, published: false};
    }
    await github.rest.repos.createCommitStatus({...repositoryParameters, sha: expected.sha, ...status});
    core.info(`${configuration.status}: ${status.state} for ${expected.sha}. ${status.target_url}`);
    return {...status, sha: expected.sha, published: true};
  }
  throw new Error('Validation kept changing while reporting; retry the status reporter after it settles.');
};
