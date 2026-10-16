const { mkdirSync, writeFileSync } = require('node:fs');
const { resolve, join } = require('node:path');
const { pathToFileURL } = require('node:url');
const { Driver } = require('../browser/webdriver.cjs');

async function main() {
  const root = resolve(__dirname, '../..');
  const output = join(root, 'output/multi-account-review');
  mkdirSync(output, { recursive: true });
  const driver = await Driver.start(1600, 1200);
  try {
    driver.get(pathToFileURL(join(root, 'examples/multi-account/diagram.svg')).href);
    const boxes = driver.execute_script(`
      const selectors = {
        corridors: '[data-state-id="application-a"] [data-terraform-address="module.network.aws_vpc.main"]',
        endpoints: '[data-state-id="application-a"] [data-terraform-address="module.network.aws_subnet.private_a"]',
        annotations: '[data-state-id="application-a"] [data-relationship-index]',
        account: '[data-state-id="application-a"] [data-account-panel]'
      };
      return Object.fromEntries(Object.entries(selectors).map(([key, selector]) => {
        const e=document.querySelector(selector) || (key === 'account' && document.querySelector('[data-state-id="application-a"]'));
        if (!e) throw new Error('Missing capture target: ' + key);
        const b=e.getBBox(), m=e.getCTM();
        const p=new DOMPoint(b.x,b.y).matrixTransform(m);
        return [key,[p.x-30,p.y-50,Math.max(b.width+60,700),Math.max(b.height+100,400)]];
      }));
    `);
    for (const [name, box] of Object.entries(boxes)) {
      driver.execute_script(`
        const svg=document.documentElement;
        svg.setAttribute('width','1500'); svg.setAttribute('height','1050');
        svg.setAttribute('viewBox',arguments[0].join(' '));
        svg.setAttribute('preserveAspectRatio','xMinYMin meet');
      `, box);
      writeFileSync(join(output, name + '.png'), driver.screenshot());
    }
  } finally { await driver.quit(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
