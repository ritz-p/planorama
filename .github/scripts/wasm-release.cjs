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

async function uploadAssets(github, repo, releaseId, assets) {
  const existing = await github.paginate(github.rest.repos.listReleaseAssets, {...repo, release_id: releaseId, per_page: 100});
  const matches = (remote, asset) => remote.size === asset.data.length && remote.digest === `sha256:${asset.digest}`;
  for (const asset of assets) {
    const remote = existing.find(remote => remote.name === asset.name);
    if (remote && !matches(remote, asset)) throw new Error(`Existing asset differs: ${asset.name}`);
  }
  for (const asset of assets) {
    if (existing.some(remote => remote.name === asset.name)) continue;
    const uploaded = (await github.rest.repos.uploadReleaseAsset({
      ...repo, release_id: releaseId, name: asset.name, data: asset.data,
      headers: {'content-type': 'application/octet-stream', 'content-length': asset.data.length},
    })).data;
    if (!matches(uploaded, asset)) throw new Error(`Upload verification failed: ${asset.name}`);
  }
}

module.exports = {requireLayoutQuality, uploadAssets};
