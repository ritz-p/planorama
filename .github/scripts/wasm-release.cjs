async function requireLayoutQuality(github, repo, commit) {
  const runs = await github.paginate(github.rest.actions.listWorkflowRuns, {
    ...repo, workflow_id: 'layout_quality.yml', head_sha: commit, event: 'workflow_dispatch', status: 'success', per_page: 100,
  });
  for (const run of runs) {
    if (run.event !== 'workflow_dispatch' || run.head_sha !== commit || run.conclusion !== 'success') continue;
    const jobs = await github.paginate(github.rest.actions.listJobsForWorkflowRun, {...repo, run_id: run.id, per_page: 100});
    if (jobs.some(job => job.conclusion === 'success' && job.steps?.some(step => step.name === 'Compare final layouts' && step.conclusion === 'success'))) return;
  }
  throw new Error('Successful Layout quality comparison is required for the release commit');
}

module.exports = {requireLayoutQuality};
