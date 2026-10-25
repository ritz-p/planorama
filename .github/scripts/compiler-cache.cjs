const {execFileSync} = require('node:child_process');
const {appendFileSync} = require('node:fs');

function shouldSave(record) {
  if (record.version !== '0.10.0' || !Number.isSafeInteger(record.stats?.cache_writes)
      || record.stats.cache_writes < 0 || !Number.isSafeInteger(record.stats.cache_write_errors)
      || record.stats.cache_write_errors < 0) throw new Error('Unexpected sccache statistics');
  return record.stats.cache_writes > 0 && record.stats.cache_write_errors === 0;
}

module.exports = {shouldSave};
if (require.main === module) {
  const record = JSON.parse(execFileSync('sccache', ['--show-stats', '--stats-format', 'json'], {encoding: 'utf8'}));
  appendFileSync(process.env.GITHUB_OUTPUT, `save=${shouldSave(record)}\n`);
}
