const {execFileSync} = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const binaries = process.argv.slice(2);
if (binaries.length !== 4) throw new Error('Expected standard, compact, wide and readable binary paths');
const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-preset-'));
function measure(binary, input) {
  const report = path.join(directory, 'report.json');
  execFileSync(binary, [input, '-o', path.join(directory, 'diagram.svg'), '--layout-diagnostics-json', report], {stdio: 'pipe'});
  const data = JSON.parse(fs.readFileSync(report));
  if (!data.limits.pair_metrics_complete) throw new Error('Incomplete pair metrics');
  const positions = new Map(data.resource_geometry.records.filter(r => r.bounds)
    .map(r => [JSON.stringify(r.identity), [r.bounds.x, r.bounds.y]]));
  return {positions, aspect: data.summary.viewport_coverage.aspect_ratio,
    whitespace: data.summary.viewport_coverage.whitespace_ratio,
    length: data.summary.raw_polyline.length, crossings: data.summary.visible_straight_sections.crossings};
}

try {
  for (const fixture of ['dense-architecture', 'spanning-dense']) {
    const file = 'tests/fixtures/' + fixture + '-plan.json';
    const plan = JSON.parse(fs.readFileSync(file));
    plan.resource_changes.push({address: 'aws_subnet.extra', type: 'aws_subnet'});
    plan.configuration.root_module.resources.push({address: 'aws_subnet.extra',
      expressions: {vpc_id: {references: ['aws_vpc.main.id']}}});
    const variant = path.join(directory, 'added.json');
    fs.writeFileSync(variant, JSON.stringify(plan));
    for (const [index, binary] of binaries.entries()) {
      const {positions, ...metrics} = measure(binary, file);
      const next = measure(binary, variant);
      let moved = 0, displacement = 0;
      for (const [key, [x, y]] of positions) {
        const retained = next.positions.get(key);
        if (!retained) throw new Error('Lost a measured resource');
        const delta = Math.abs(retained[0] - x) + Math.abs(retained[1] - y);
        if (delta) moved++;
        displacement += delta;
      }
      console.log(JSON.stringify({fixture, preset: ['standard','compact','wide','readable'][index],
        ...metrics, measured: positions.size, moved, displacement}));
    }
  }
} finally {
  fs.rmSync(directory, {recursive: true, force: true});
}
