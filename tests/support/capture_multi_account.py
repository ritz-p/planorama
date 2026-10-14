from pathlib import Path
from selenium import webdriver
from selenium.webdriver.chrome.service import Service

root = Path(__file__).resolve().parents[2]
output = root / 'output/multi-account-review'
output.mkdir(parents=True, exist_ok=True)
options = webdriver.ChromeOptions()
for flag in ['--headless', '--no-sandbox', '--disable-dev-shm-usage', '--window-size=1600,1200']:
    options.add_argument(flag)
with webdriver.Chrome(service=Service('/usr/bin/chromedriver'), options=options) as driver:
    driver.get((root / 'examples/multi-account/diagram.svg').as_uri())
    boxes = driver.execute_script('''
        const selectors = {
          corridors: '[data-state-id="application-a"] [data-terraform-address="module.network.aws_vpc.main"]',
          endpoints: '[data-state-id="application-a"] [data-terraform-address="module.network.aws_subnet.private_a"]',
          annotations: '[data-state-id="application-a"] [data-relationship-index]',
          account: '[data-state-id="application-a"] [data-account-panel]'
        };
        return Object.fromEntries(Object.entries(selectors).map(([key, selector]) => {
          const e=document.querySelector(selector), b=e.getBBox(), m=e.getCTM();
          const p=new DOMPoint(b.x,b.y).matrixTransform(m);
          return [key,[p.x-30,p.y-50,Math.max(b.width+60,700),Math.max(b.height+100,400)]];
        }));
    ''')
    for name, box in boxes.items():
        driver.execute_script('''
          const svg=document.documentElement;
          svg.setAttribute('width','1500'); svg.setAttribute('height','1050');
          svg.setAttribute('viewBox',arguments[0].join(' '));
          svg.setAttribute('preserveAspectRatio','xMinYMin meet');
        ''', box)
        driver.save_screenshot(str(output / (name + '.png')))
