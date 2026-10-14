import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.action_chains import ActionChains
from selenium.webdriver.chrome.service import Service


ROOT = Path(__file__).resolve().parents[2]
BINARY = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')) / 'debug/planorama'


class ViewerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        options = webdriver.ChromeOptions()
        for flag in ['--headless', '--no-sandbox', '--disable-dev-shm-usage', '--window-size=1400,1000']:
            options.add_argument(flag)
        options.set_capability('goog:loggingPrefs', {'browser': 'ALL', 'performance': 'ALL'})
        cls.driver = webdriver.Chrome(service=Service('/usr/bin/chromedriver'), options=options)
        cls.driver.execute_cdp_cmd('Network.enable', {})
        cls.driver.execute_cdp_cmd('Network.emulateNetworkConditions', {
            'offline': True, 'latency': 0, 'downloadThroughput': 0, 'uploadThroughput': 0,
        })
        cls.temp = tempfile.TemporaryDirectory()

    @classmethod
    def tearDownClass(cls):
        cls.driver.quit()
        cls.temp.cleanup()

    def load(self, *args, input_plan=None, transform=None):
        output = Path(self.temp.name) / 'viewer.html'
        command = [str(BINARY), *args, '--format', 'html', '-o', str(output)]
        subprocess.run(command, input=json.dumps(input_plan) if input_plan else None,
                       text=True, check=True, capture_output=True, cwd=ROOT)
        if transform:
            output.write_text(transform(output.read_text()))
        self.driver.get(output.as_uri())
        self.assertIn('resources', self.find('#result-count').text)

    def tearDown(self):
        errors = [entry for entry in self.driver.get_log('browser') if entry['level'] == 'SEVERE']
        self.assertEqual(errors, [])
        requests = [json.loads(entry['message'])['message'] for entry in self.driver.get_log('performance')]
        remote = [r['params']['request']['url'] for r in requests
                  if r['method'] == 'Network.requestWillBeSent'
                  and r['params']['request']['url'].startswith(('http:', 'https:'))]
        self.assertEqual(remote, [])

    def find(self, selector):
        return self.driver.find_element(By.CSS_SELECTOR, selector)

    def view_box(self):
        return self.driver.execute_script("return document.querySelector('#viewport > svg').getAttribute('viewBox')")

    def search(self, text):
        field = self.find('#search')
        field.clear()
        field.send_keys(text)
        return self.driver.find_elements(By.CSS_SELECTOR, '#results button')

    def choose(self, text):
        matches = self.search(text)
        self.assertTrue(matches, text)
        matches[0].click()
        return self.find('#viewport [data-viewer-selected]').get_attribute('id')

    def test_maintained_multi_account_example_search_selection_and_fit(self):
        self.driver.get((ROOT / 'examples/multi-account/diagram.html').as_uri())
        before = self.view_box()
        matches = self.search('module.network.aws_vpc.main')
        self.assertEqual(len(matches), 3)
        selected = set()
        for state in ['application-a', 'application-b', 'shared-services']:
            selected.add(self.choose(state + ':module.network.aws_vpc.main'))
            self.assertEqual(self.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-state-id]').dataset.stateId"), state)
        self.assertEqual(len(selected), 3)
        self.choose('application-a:aws_s3_bucket.regional_archive')
        self.find('#zoom-in').click()
        self.assertNotEqual(before, self.view_box())
        self.find('#fit').click()
        self.assertEqual(before, self.view_box())
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-account-panel]')), 4)

    def test_offline_keyboard_search_zoom_pan_clear_and_fit(self):
        self.load('examples/plan.json')
        before = self.view_box()
        search = self.find('#search')
        search.send_keys(Keys.ARROW_DOWN)
        self.assertEqual(self.driver.switch_to.active_element.tag_name, 'button')
        self.driver.switch_to.active_element.send_keys(Keys.ENTER)
        self.assertTrue(self.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]'))
        focused = self.view_box()
        self.assertNotEqual(before, focused)
        self.find('#zoom-in').click()
        self.assertNotEqual(focused, self.view_box())
        self.driver.execute_script("document.getElementById('viewport').focus()")
        self.driver.switch_to.active_element.send_keys(Keys.ARROW_RIGHT, Keys.ESCAPE)
        self.assertFalse(self.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected], [data-viewer-edge], [data-viewer-related]'))
        self.find('#fit').click()
        self.assertEqual(before, self.view_box())
        viewport = self.find('#viewport')
        ActionChains(self.driver).move_to_element(viewport).click_and_hold().move_by_offset(90, 40).release().perform()
        self.assertNotEqual(before, self.view_box())

    def test_rendered_edge_semantics_and_bundles_survive_selection(self):
        for fixture in ['examples/bundling-plan.json', 'tests/fixtures/aws-relationships-plan.json']:
            self.load(fixture)
            script = "return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e => [e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end'),e.dataset.directionality,e.dataset.source,e.dataset.target])"
            before = self.driver.execute_script(script)
            edges = self.driver.find_elements(By.CSS_SELECTOR, 'path[data-edge-kind][data-source]')
            self.assertTrue(edges)
            for edge in edges:
                source = edge.get_attribute('data-source')
                self.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", source)
                direction = edge.get_attribute('data-directionality')
                self.assertEqual(edge.get_attribute('data-viewer-edge'), 'outgoing' if direction == 'directed' else 'undirected')
                self.assertIn('Outgoing rendered edges' if direction == 'directed' else 'Undirected rendered relationships', self.find('#selection').text)
                self.assertEqual(before, self.driver.execute_script(script))

    def test_component_membership_and_reference_navigation_are_separate(self):
        self.load('tests/fixtures/components-plan.json')
        self.choose('aws_lb.app')
        self.assertIn('Logical Component membership', self.find('#selection').text)
        self.find('#selection h3:last-of-type + ul button').click()
        self.assertIn('Component members', self.find('#selection').text)
        self.find('#selection button').click()
        self.assertTrue(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'))
        self.load('tests/fixtures/viewer-plan.json')
        reference = self.find('a[data-reference-attribute]')
        owner = self.driver.execute_script("return arguments[0].closest('[data-viewer-target]').id", reference)
        self.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", owner)
        self.assertIn('Uses / reference targets', self.find('#selection').text)
        target = self.driver.execute_script("return arguments[0].getAttribute('href').slice(1)", reference)
        self.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", reference)
        self.assertEqual(target, self.find('[data-viewer-selected]').get_attribute('id'))

    def test_named_states_and_cross_state_direction(self):
        self.load('--state', 'producer=tests/fixtures/multi/producer.json', '--state',
                  'consumer=tests/fixtures/multi/consumer.json', '--remote-state',
                  'consumer:data.terraform_remote_state.network=producer')
        edge = self.find('path[data-cross-state]')
        for endpoint, direction in [('source', 'outgoing'), ('target', 'incoming')]:
            self.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", edge.get_attribute('data-' + endpoint))
            self.assertEqual(edge.get_attribute('data-viewer-edge'), direction)
        self.assertTrue(self.search('producer'))
        self.assertTrue(all('producer:' in b.text for b in self.search('producer')))

    def test_lowered_resource_search_highlights_only_its_parallel_relationship(self):
        plan = json.loads((ROOT / 'tests/fixtures/relationship-scopes-plan.json').read_text())
        second = 'aws_route_table_association.second'
        plan['resource_changes'].append({'address': second, 'type': 'aws_route_table_association', 'change': {'actions': ['delete']}})
        config = dict(plan['configuration']['root_module']['resources'][-1], address=second)
        plan['configuration']['root_module']['resources'].append(config)
        self.load('-', input_plan=plan)
        self.choose(second)
        self.assertIn('Rendered relationships from this resource', self.find('#selection').text)
        paths = self.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')
        self.assertEqual(len(paths), 1)
        self.assertIn(second, paths[0].find_element(By.TAG_NAME, 'title').get_attribute('textContent'))
        before = self.driver.execute_script("return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e=>[e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end')])")
        entry = self.find('[data-relationship-entry="2"]')
        text = ''.join(n.get_attribute('textContent') for n in entry.find_elements(By.TAG_NAME, 'tspan'))
        self.assertIn('Resource scope: AWS Account: 111111111111 · Region: ap-northeast-1', text)
        self.assertIn('aws_route_table.private · AWS Account: 222222222222 · Region: us-east-1', text)
        number = entry.find_element(By.CSS_SELECTOR, '[data-relationship-select]')
        self.driver.execute_script('arguments[0].focus()', number)
        number.send_keys(Keys.ENTER)
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('id'), entry.get_attribute('id'))
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')), 1)
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-marker] [data-viewer-related]')), 2)
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '#selection button')), 2)
        self.find('#selection button').click()
        self.assertTrue(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'))
        self.find('#clear').click()
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')), 0)
        after = self.driver.execute_script("return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e=>[e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end')])")
        self.assertEqual(before, after)

    def test_number_selection_stays_within_its_state(self):
        self.load('--state', 'one=tests/fixtures/relationship-scopes-plan.json', '--state', 'two=tests/fixtures/relationship-scopes-plan.json')
        for state in ['one', 'two']:
            marker = self.find(f'[data-state-id="{state}"] [data-relationship-marker] a')
            self.driver.execute_script('arguments[0].focus()', marker)
            marker.send_keys(Keys.SPACE)
            self.assertEqual(self.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-state-id]').dataset.stateId"), state)
            selected_states = self.driver.execute_script("return Array.from(document.querySelectorAll('[data-viewer-related], [data-viewer-edge]')).map(e=>e.closest('[data-state-id]').dataset.stateId)")
            self.assertTrue(selected_states)
            self.assertEqual(set(selected_states), {state})

    def test_escaped_long_addresses_modes_and_individual_logical_targets(self):
        address = 'module.app["a.b"].aws_iam_role.item["</script><script>window.pwned=1</script>&\\\"' + 'x' * 100 + '"]'
        plan = {'format_version': '1.2', 'resource_changes': [
            {'address': address, 'type': 'aws_iam_role', 'change': {'actions': ['create'], 'after': {'secret': 'DO_NOT_EXPOSE'}}},
            {'address': 'aws_iam_role.item', 'type': 'aws_iam_role'},
            {'address': 'data.aws_iam_role.item', 'type': 'aws_iam_role', 'mode': 'data'},
        ]}
        self.load('-', input_plan=plan)
        self.assertIsNone(self.driver.execute_script('return window.pwned'))
        self.assertNotIn('DO_NOT_EXPOSE', self.driver.page_source)
        self.assertEqual(len(self.search('managed')), 2)
        self.assertEqual(len(self.search('data')), 1)
        selected = self.choose('module.app')
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'), address)
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('data-placement'), 'logical')
        self.driver.execute_script("const row=document.getElementById(arguments[0]); const group=document.createElementNS('http://www.w3.org/2000/svg','g'); row.parentNode.insertBefore(group,row); group.append(row); group.setAttribute('data-type-group','aws_iam_role');", selected)
        self.find('#clear').click()
        self.assertEqual(selected, self.choose('module.app'))
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]')), 1)

    def test_filtered_and_unresolved_targets_do_not_gain_navigation(self):
        self.load('tests/fixtures/viewer-plan.json', '--focus', 'aws_ecs_service.app', '--focus-depth', '0')
        resources = self.driver.execute_script("return JSON.parse(document.getElementById('viewer-data').textContent)")
        self.assertEqual(len(resources), len(self.driver.find_elements(By.CSS_SELECTOR, '#results button')))
        broken = self.driver.execute_script("return Array.from(document.querySelectorAll('#viewport a[href]')).filter(a=>!document.getElementById(a.getAttribute('href').slice(1))).length")
        self.assertEqual(broken, 0)

    def test_duplicate_state_addresses_deposed_objects_and_embedded_routes(self):
        self.load('--state', 'one=tests/fixtures/deposed-plan.json', '--state', 'two=tests/fixtures/deposed-plan.json')
        resources = self.driver.execute_script("return JSON.parse(document.getElementById('viewer-data').textContent)")
        first = next(r for r in resources if r['state'] == 'one' and r['deposed'])
        second = next(r for r in resources if r['state'] == 'two' and r['address'] == first['address'] and r['deposed'] == first['deposed'])
        self.assertNotEqual(first['id'], second['id'])
        self.assertEqual(first['id'], self.choose('one:' + first['address'] + ' ' + first['deposed']))
        self.assertEqual(second['id'], self.choose('two:' + second['qualified'] + ' ' + second['deposed']))
        self.load('tests/fixtures/routes-plan.json')
        route = self.find('[data-embedded-route]')
        self.assertEqual(route.get_attribute('id'), self.choose(route.get_attribute('data-terraform-address')))
        self.assertIn('Outgoing rendered edges', self.find('#selection').text)
        next_hop = self.find('[data-embedded-route] [data-relationship-select]')
        entry_id = self.driver.execute_script("return arguments[0].getAttribute('href').slice(1)", next_hop)
        self.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", next_hop)
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('id'), entry_id)
        paths = self.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')
        self.assertEqual(len(paths), 1)
        self.assertEqual(paths[0].get_attribute('data-directionality'), 'directed')
        self.assertTrue(paths[0].get_attribute('marker-end'))
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-marker] [data-viewer-related]')), 2)


if __name__ == '__main__':
    unittest.main()
