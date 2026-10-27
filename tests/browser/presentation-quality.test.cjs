const assert = require('node:assert/strict');
const {test} = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const {spawnSync} = require('node:child_process');
const {Driver} = require('./webdriver.cjs');
const {inspectPresentation} = require('../../scripts/presentation-quality.cjs');

test('presentation geometry distinguishes legitimate decoration from collisions and child overflow', async () => {
  const driver = await Driver.start();
  try {
    const inspect = content => {
      driver.get('about:blank');
      driver.execute_script(`document.body.innerHTML = arguments[0];`, `<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600">
        <g data-state-id="named" transform="translate(20 30)"><g data-annotation-card="true">
        <rect width="500" height="400" fill="none" stroke="black"/>
        <g data-action-section="create" transform="translate(10 10)"><rect width="200" height="120" fill="none" stroke="black" stroke-width="2"/>
        <g data-terraform-address="long.resource"><text x="10" y="25" font-size="12">address</text>
        <rect data-service-badge="true" x="100" y="10" width="80" height="30"/>
        <text data-service-name="true" x="110" y="30" font-size="12">label</text>
        <line data-resource-underline="true" x1="10" x2="190" y1="25" y2="25" stroke="black"/>
        ${content}</g></g></g></g></svg>`);
      return driver.execute_script(`return (${inspectPresentation.toString()})();`);
    };
    assert.deepEqual(inspect('').findings, []);
    assert.deepEqual(inspect('<g data-hotspot-overlay="true"><rect width="5" height="5"/><text x="0" y="25">selection decoration</text></g>').findings, []);
    const overlap = inspect('<text x="15" y="25" font-size="12">collision</text>');
    assert(overlap.findings.some(f => f.category === 'overlap' && f.element.resource === 'long.resource'));
    assert(overlap.findings.every(f => f.element.state === 'named' && f.element.section === 'create'));
    assert(inspect('<text x="95" y="35" font-size="12">heading over badge</text>').findings.some(f => f.category === 'overlap' && f.peer.kind === 'badge'));
    const overflow = inspect('<text x="195" y="90" font-size="12">outside child</text>');
    assert(overflow.findings.some(f => f.category === 'content-overflow' && f.peer.bounds.width === 202));
    assert(inspect('<g data-import-badge="true"><rect x="190" y="60" width="30" height="20"/><text x="192" y="74" font-size="10">import</text></g>').findings.some(f => f.category === 'badge-frame-intersection'));
    assert(inspect('<g data-import-badge="true"><rect x="-5" y="-5" width="210" height="130"/></g>').findings.some(f => f.category === 'badge-frame-intersection'));
    assert(inspect('<g data-import-badge="true"><rect x="10" y="60" width="20" height="20"/><text x="12" y="74" font-size="10">too long</text></g>').findings.some(f => f.category === 'badge-text-overflow'));
    assert.deepEqual(inspect('<text font-size="12"><tspan x="10" y="65">line one</tspan><tspan x="10" y="85">line two</tspan></text>').findings, []);
    assert.deepEqual(inspect('<rect data-service-badge="true" data-presentation-padding="10 4" x="10" y="50" width="180" height="55"/><text data-service-name="true" x="20" y="70" font-size="12">A long service label</text><text data-service-name="true" x="20" y="90" font-size="12">continued on a new line</text>').findings, []);
    assert(inspect('<g data-presentation-header="0 0 200 50"><text x="10" y="95" font-size="12">below header</text></g>').findings.some(f => f.category === 'header-overflow'));
    assert(inspect('<g transform="translate(10 60) scale(2)"><rect data-service-badge="true" data-presentation-padding="10 4" width="70" height="25"/><text data-service-name="true" x="2" y="18" font-size="10">padding</text></g>').findings.some(f => f.category === 'badge-text-overflow'));
    assert.equal(overlap.limits.complete, true);
    driver.execute_script(`const group=document.querySelector('[data-terraform-address]'); group.innerHTML='<text x="10" y="25">dense</text>'.repeat(2100);`);
    const limited = driver.execute_script(`const r=(${inspectPresentation.toString()})(); return {limits:r.limits, findings:r.findings.length};`);
    assert.equal(limited.limits.complete, false);
    assert.equal(limited.limits.comparisons, limited.limits.pair_limit + 1);
    assert.equal(limited.findings, limited.limits.finding_limit);
  } finally { await driver.quit(); }
});

test('rendered representative cards use the same reusable presentation checks', async () => {
  fs.mkdirSync('output', {recursive: true});
  const directory = fs.mkdtempSync(path.resolve('output/presentation-test-'));
  const driver = await Driver.start();
  const binary = process.env.LAYOUT_QUALITY_BINARY || 'target/debug/planorama';
  try {
    const longType = 'custom_' + 'complete_resource_type_'.repeat(9);
    const longPlan = path.join(directory, 'long.json');
    fs.writeFileSync(longPlan, JSON.stringify({format_version: '1.2', resource_changes: [
      {address: `${longType}.${'long_address_'.repeat(15)}`, type: longType, change: {actions: ['create']}},
      {address: 'data.aws_vpc.external', type: 'aws_vpc', mode: 'data'},
    ]}));
    const portsPlan = path.join(directory, 'ports.json');
    const routes = Array.from({length: 8}, (_, i) => ({address: `aws_route.route_${i}`, type: 'aws_route',
      change: {actions: ['create'], after: {route_table_id: 'rtb-example', nat_gateway_id: 'nat-example', destination_cidr_block: `10.${i}.0.0/16`}}}));
    fs.writeFileSync(portsPlan, JSON.stringify({format_version: '1.2', resource_changes: [
      {address: 'aws_nat_gateway.main', type: 'aws_nat_gateway', change: {actions: ['create'], after: {id: 'nat-example'}}},
      {address: 'aws_route_table.main', type: 'aws_route_table', change: {actions: ['create'], after: {id: 'rtb-example'}}},
      ...routes,
    ], configuration: {root_module: {resources: routes.map(route => ({address: route.address, type: route.type,
      expressions: {route_table_id: {references: ['aws_route_table.main.id']}, nat_gateway_id: {references: ['aws_nat_gateway.main.id']}}}))}}}));
    const cases = [
      [longPlan],
      [portsPlan],
      ['--state', 's=tests/fixtures/association-plan.json'],
      ['--state', 'W'.repeat(64) + '=tests/fixtures/association-plan.json'],
      ['--state', 'W'.repeat(64) + '=tests/fixtures/association-plan.json', '--state', 's=tests/fixtures/association-plan.json'],
      ['--state', 'producer=tests/fixtures/multi/producer.json', '--state', 'consumer=tests/fixtures/multi/consumer.json', '--remote-state', 'consumer:data.terraform_remote_state.network=producer'],
      ['tests/fixtures/service-labels-plan.json'],
      ['tests/fixtures/logical-actions-plan.json'],
      ['tests/fixtures/ecs-clusters-plan.json'],
      ['tests/fixtures/dynamodb-plan.json'],
      ['tests/fixtures/elasticache-configuration-plan.json'],
      ['tests/fixtures/eventbridge-configuration-plan.json'],
      ['tests/fixtures/msk-plan.json'],
      ['tests/fixtures/mq-plan.json'],
      ['tests/fixtures/docdb-plan.json'],
      ['tests/fixtures/neptune-plan.json'],
      ['tests/fixtures/redshift-serverless-plan.json'],
      ['tests/fixtures/apprunner-plan.json'],
      ['tests/fixtures/opensearch-serverless-plan.json'],
      ['tests/fixtures/networkfirewall-plan.json'],
      ['tests/fixtures/routes-plan.json'],
      ['tests/fixtures/components-plan.json'],
      ['tests/fixtures/relationship-index-plan.json'],
      ['tests/fixtures/aws-basic-plan.json'],
      ['--state', 'data=tests/fixtures/data-icons-plan.json', '--state', 'actions=tests/fixtures/logical-actions-plan.json'],
    ];
    for (const [index, args] of cases.entries()) {
      const file = path.join(directory, `${index}.svg`);
      const result = spawnSync(binary, [...args, '-o', file], {encoding: 'utf8', timeout: 180000});
      assert.equal(result.status, 0, result.stderr);
      driver.get(pathToFileURL(file).href);
      const report = driver.execute_script(`return (${inspectPresentation.toString()})();`);
      fs.writeFileSync(path.join(directory, `${index}.json`), JSON.stringify(report, null, 2));
      const headings = driver.execute_script(`
        const root = document.documentElement;
        return [...document.querySelectorAll('[data-state-heading]')].map(heading => {
          const state = heading.closest('[data-state-id]');
          const texts = [...heading.querySelectorAll('text')];
          const rect = heading.querySelector('rect').getBBox();
          const textWidth = Math.max(...texts.map(text => text.getBBox().width));
          const nested = state.querySelector(':scope > svg');
          const title = [...nested.querySelectorAll('text')].find(text => text.textContent === 'Terraform plan');
          const luminance = color => color.match(/[0-9.]+/g).slice(0,3).map(Number).map(v => v / 255)
            .map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
            .reduce((total,v,i) => total + v * [0.2126,0.7152,0.0722][i], 0);
          const contrast = (luminance(getComputedStyle(heading.querySelector('rect')).fill) + 0.05) /
            (luminance(getComputedStyle(texts[0]).fill) + 0.05);
          let pathOverlap = false;
          for (const path of document.querySelectorAll('[data-cross-state]')) {
            const length = path.getTotalLength();
            for (let d = 0; d <= length; d += 4) {
              const point = path.getPointAtLength(d);
              if (point.x >= rect.x && point.x <= rect.x + rect.width && point.y >= rect.y && point.y <= rect.y + rect.height) pathOverlap = true;
            }
          }
          return {complete: texts.map(text => text.textContent).join('') === 'State: ' + state.dataset.stateId,
            fontMatches: texts.every(text => getComputedStyle(text).fontSize === getComputedStyle(title).fontSize),
            fitted: rect.width >= textWidth + 32 && rect.width <= textWidth * 1.1 + 32,
            wrapped: state.dataset.stateId.length < 64 || texts.length > 1, contrast, pathOverlap,
            spaced: nested.y.baseVal.value > rect.y + rect.height,
            inside: rect.x > 0 && rect.x + rect.width < root.viewBox.baseVal.width &&
              nested.y.baseVal.value + nested.height.baseVal.value <= root.viewBox.baseVal.height};
        });
      `);
      assert.equal(headings.length, args.filter(arg => arg === '--state').length);
      for (const heading of headings) {
        assert(heading.complete && heading.fontMatches && heading.fitted && heading.wrapped && heading.spaced && heading.inside);
        assert(heading.contrast >= 4.5);
        assert.equal(heading.pathOverlap, false);
      }
      assert(report.counts.text > 0 && report.counts.frames > 0);
      assert.equal(report.limits.complete, true);
      assert.deepEqual(report.findings, [], `${args.join(' ')}: ${JSON.stringify(report.findings)}`);
    }
  } finally { await driver.quit(); }
});
