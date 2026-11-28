const {spawnSync} = require('node:child_process');

function isAncestor(before, after) {
  return spawnSync('git', ['merge-base', '--is-ancestor', before, after], {stdio: 'ignore'}).status === 0;
}

function mutationHistory(rows, precedes = isAncestor) {
  const groups = new Map();
  for (const [i, row] of rows.entries()) {
    const key = JSON.stringify([row.behavior, row.mutation]);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push({...row, line: i + 2});
  }
  const counts = {missed: 0, improved: 0, always: 0, uncertain: 0};
  const histories = [];
  for (const records of groups.values()) {
    const revisions = [...new Set(records.map(row => row.revision))];
    const predecessors = new Map(revisions.map(rev => [rev, new Set()]));
    for (const a of revisions) for (const b of revisions) {
      if (a !== b && a !== 'unrecorded' && b !== 'unrecorded' && precedes(a, b)) predecessors.get(b).add(a);
    }
    const ordered = !revisions.includes('unrecorded') && revisions.every((rev, i) =>
      revisions.slice(i + 1).every(other => predecessors.get(rev).has(other) || predecessors.get(other).has(rev)));
    if (ordered) records.sort((a,b) => predecessors.get(a.revision).size - predecessors.get(b.revision).size);
    const latest = ordered ? revisions.find(rev => predecessors.get(rev).size === revisions.length - 1) : undefined;
    const last = latest ? records.filter(row => row.revision === latest) : [];
    let status = 'uncertain';
    if (records.every(row => row.result === 'killed')) status = 'always';
    else if (last.length && last.every(row => row.result === 'survived')) status = 'missed';
    else if (last.length && last.every(row => row.result === 'killed') &&
      records.some(row => row.revision !== latest && row.result === 'survived')) status = 'improved';
    counts[status]++;
    histories.push({behavior: records[0].behavior, mutation: records[0].mutation, status, ordered, records});
  }
  return {counts, measurements: rows.length, histories};
}

module.exports = {mutationHistory};
