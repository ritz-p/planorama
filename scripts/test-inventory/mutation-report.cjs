const fs = require('node:fs');

const results = new Map(Object.entries({
  CaughtMutant:'killed', MissedMutant:'survived', Timeout:'timeout',
  Unviable:'unviable', Success:'not-evaluated', Failure:'inconclusive',
}));

function measurements(report, revision, suite, evidence) {
  if (!/^[a-f0-9]{40}$/.test(revision) || !suite || !evidence) throw Error('Supply the tested full commit SHA, test selection and durable evidence reference');
  const baseline = report.outcomes.find(o => o.scenario === 'Baseline');
  if (baseline?.summary !== 'Success') throw Error('A successful baseline is required');
  const mutants = report.outcomes.filter(o => o.scenario !== 'Baseline');
  if (!mutants.length || mutants.length !== report.total_mutants) throw Error('Incomplete mutation report');
  return mutants.map(o => {
    const m = o.scenario.Mutant;
    const result = results.get(o.summary);
    if (!m?.file || !m.span?.start || typeof m.replacement !== 'string' || !result) throw Error('Unknown cargo-mutants outcome');
    const behavior = m.function?.function_name || m.file;
    const location = [m.file,m.span.start.line,m.span.start.column].join(':');
    return [behavior, location+' '+m.genre+' → '+JSON.stringify(m.replacement),suite,result,revision,evidence];
  });
}

if (require.main === module) {
  try {
    const [report, revision, suite, evidence] = process.argv.slice(2);
    for (const row of measurements(JSON.parse(fs.readFileSync(report)),revision,suite,evidence)) {
      if (row.some(value => /[\t\r\n]/.test(value))) throw Error('TSV field contains a tab or newline');
      console.log(row.join('\t'));
    }
  } catch (e) { console.error(e.message); process.exitCode=1; }
}
module.exports = {measurements};
