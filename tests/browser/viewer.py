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
from selenium.webdriver.support.ui import Select


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

    def test_copy_native_address_success_rejection_and_unavailable(self):
        address = 'module.demo["日本語"].test.item["a\\\"b"]'
        plan = {'format_version': '1.2', 'resource_changes': [
            {'address': address, 'type': 'test', 'deposed': 'old'}]}
        self.load('-', input_plan=plan)
        self.find('#results button').send_keys(Keys.ENTER)
        self.assertEqual(self.find('.copy-identity textarea').get_attribute('value'), address)
        self.assertIn('deposed: old', self.find('.copy-identity').text)
        before = self.view_box()
        for mode in ['success', 'rejected', 'unavailable']:
            self.driver.execute_script("""
                window.copied = null;
                const mode = arguments[0];
                Object.defineProperty(navigator, 'clipboard', {configurable: true, value:
                    mode === 'unavailable' ? undefined : {writeText: async text => {
                        if (mode === 'rejected') throw new Error('denied');
                        window.copied = text;
                    }}});
            """, mode)
            self.find('.copy-identity button').send_keys(Keys.ENTER)
            if mode == 'success':
                self.assertEqual(self.driver.execute_script('return window.copied'), address)
                self.assertIn('address copied', self.find('.copy-identity [role=status]').text)
            else:
                self.assertIn('manually', self.find('.copy-identity [role=status]').text)
                self.assertEqual(self.driver.execute_script('return document.activeElement.value'), address)
                self.assertEqual(self.driver.execute_script('return document.activeElement.selectionEnd'), len(address))
            self.assertEqual(before, self.view_box())
        self.find('#clear').click()
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '.copy-identity'), [])
        self.load('-', '--address-format', 'terraform', input_plan=plan)
        self.find('#results button').send_keys(Keys.ENTER)
        self.assertEqual(self.find('.copy-identity textarea').get_attribute('value'), address)
        self.driver.execute_script("Object.defineProperty(navigator, 'clipboard', {configurable: true, value: {writeText: () => new Promise((resolve, reject) => {window.rejectCopy = reject;})}})")
        self.find('.copy-identity button').click()
        self.find('#clear').click()
        self.driver.execute_script("window.rejectCopy(new Error('late rejection'))")
        self.assertEqual(self.find('#selection').text, '')
        self.assertEqual(self.driver.switch_to.active_element.get_attribute('id'), 'clear')

    def test_copy_targets_cover_resource_presentations_and_relationship_roles(self):
        for fixture in ['viewer', 'routes', 'association', 'deposed']:
            self.load('--state', 'one=tests/fixtures/' + fixture + '-plan.json',
                      '--state', 'two=tests/fixtures/' + fixture + '-plan.json')
            buttons = self.driver.find_elements(By.CSS_SELECTOR, '#results button')
            for result in buttons:
                result.click()
                selected = self.find('[data-viewer-selected]')
                native = selected.get_attribute('data-terraform-address') or selected.get_attribute('data-source-address')
                self.assertEqual(self.find('.copy-identity textarea').get_attribute('value'), native)
                self.assertIn('State: ' + selected.find_element(By.XPATH, './ancestor::*[@data-state-id]').get_attribute('data-state-id'),
                              self.find('.copy-identity').text)
            relationships = self.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-entry] [data-relationship-select]')
            if relationships:
                relationships[0].send_keys(Keys.ENTER)
                groups = self.driver.find_elements(By.CSS_SELECTOR, '.copy-identity')
                self.assertTrue(groups)
                self.assertTrue(all('Endpoint resource' in group.text or 'Provenance resource' in group.text for group in groups))

    def test_service_names_have_space_in_named_state_resource_rows(self):
        self.load('--state', 'services=tests/fixtures/data-icons-plan.json',
                  '--state', 'builtin=tests/fixtures/terraform-plan.json',
                  '--icon-mode', 'service-name')
        result = self.driver.execute_script("""
            const svg = document.querySelector('#viewport > svg');
            const viewport = document.querySelector('#viewport');
            svg.setAttribute('viewBox', `0 0 ${viewport.clientWidth} ${viewport.clientHeight}`);
            const labels = [...svg.querySelectorAll('[data-service-name]')];
            const overlaps = [];
            for (const label of labels) {
                const row = label.closest('[data-terraform-address]');
                const a = label.getBoundingClientRect();
                for (const other of row.querySelectorAll('text')) {
                    if (other === label) continue;
                    const b = other.getBoundingClientRect();
                    if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.1 &&
                        Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.1) {
                        overlaps.push([label.textContent, other.textContent]);
                    }
                }
            }
            return {count: labels.length, overlaps,
                    symbols: svg.querySelectorAll('symbol, image').length};
        """)
        self.assertGreater(result['count'], 10)
        self.assertEqual(result['overlaps'], [])
        self.assertEqual(result['symbols'], 0)

    def test_coverage_summary_is_absent_from_empty_and_single_resource_diagrams(self):
        for changes in [[], [{"address": "terraform_data.one", "type": "terraform_data"}]]:
            self.load('-', input_plan={"format_version": "1.2", "resource_changes": changes})
            self.assertFalse(self.driver.execute_script("return !!document.querySelector('[data-view-coverage]')"))
            self.assertNotIn('Diagram Coverage', self.find('svg').text)

    def test_hidden_relationship_legend_reclaims_space_and_keeps_annotations_clear(self):
        self.load('tests/fixtures/routes-plan.json')
        full_height, full_legend = self.driver.execute_script("return [document.querySelector('svg').viewBox.baseVal.height, +document.querySelector('#relationship-legend rect').getAttribute('height')]")
        self.load('tests/fixtures/routes-plan.json', '--hide-relationship-legend')
        compact = self.driver.execute_script("""
            const svg = document.querySelector('svg');
            const actions = document.querySelector('#relationship-actions');
            const bounds = actions.getBoundingClientRect();
            const annotations = [...svg.querySelectorAll('[data-annotation-section]')];
            const overlap = annotations.some(n => n.getBoundingClientRect().top < bounds.bottom);
            const outside = [...actions.querySelectorAll('text')].some(n => {
                const b = n.getBoundingClientRect();
                return b.left < bounds.left || b.right > bounds.right || b.top < bounds.top || b.bottom > bounds.bottom;
            });
            return {height: svg.viewBox.baseVal.height, legend: +actions.querySelector('rect').getAttribute('height'),
                    overlap, outside, entries: svg.querySelectorAll('[data-relationship-entry]').length,
                    explanations: svg.querySelectorAll('[data-legend-entry]').length};
        """)
        self.assertEqual(full_height - compact['height'], full_legend - compact['legend'])
        self.assertGreater(full_height, compact['height'])
        self.assertFalse(compact['overlap'])
        self.assertFalse(compact['outside'])
        self.assertGreater(compact['entries'], 0)
        self.assertEqual(compact['explanations'], 0)

    def test_semantic_diagnostics_navigate_all_resource_presentations_and_reset(self):
        for fixture, address in [('containment', 'aws_instance.app'),
                                 ('logical-regions', 'aws_iam_policy'),
                                 ('routes', 'aws_route.'),
                                 ('association', 'aws_route_table_association.private')]:
            self.load('--state', 'one=tests/fixtures/' + fixture + '-plan.json',
                      '--state', 'two=tests/fixtures/' + fixture + '-plan.json',
                      '--html-semantic-diagnostics')
            before = self.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)")
            self.find('#semantic-toggle').send_keys(Keys.ENTER)
            self.find('#semantic-search').send_keys('two ' + address)
            button = self.find('#semantic-list button')
            self.assertIn('two:', button.text)
            button.send_keys(Keys.ENTER)
            selected = self.find('[data-viewer-selected]')
            self.assertEqual(selected.find_element(By.XPATH, './ancestor::*[@data-state-id]').get_attribute('data-state-id'), 'two')
            self.assertEqual(self.find('#semantic-toggle').get_attribute('aria-expanded'), 'false')
            self.assertEqual(self.driver.switch_to.active_element.get_attribute('id'), selected.get_attribute('id'))
            self.find('#clear').click()
            self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]'), [])
            self.assertEqual(before, self.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)"))

    def test_semantic_filters_deposed_identity_bounds_unavailable_and_safe_text(self):
        hostile = 'test.item["</script><script>alert(1)</script>"]'
        plan = {'format_version': '1.2', 'resource_changes': [
            {'address': hostile, 'type': 'test'},
            {'address': hostile, 'type': 'test', 'deposed': 'old'},
            {'address': 'test.dynamic', 'type': 'test'}],
            'configuration': {'root_module': {'resources': [
                {'address': 'test.dynamic', 'expressions': {
                    'input': {'references': ['each.value']},
                    'missing': {'references': ['test.absent.id']}}}]}}}
        self.load('-', '--html-semantic-diagnostics', input_plan=plan)
        self.find('#semantic-toggle').click()
        self.find('#semantic-search').send_keys('old')
        self.find('#semantic-list button').click()
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('data-deposed-key'), 'old')
        self.find('#semantic-toggle').click()
        self.find('#semantic-search').clear()
        Select(self.find('#semantic-category')).select_by_value('dynamic')
        self.assertIn('dynamic_instance_selection', self.find('#semantic-list').text)
        self.assertNotIn('unsupported_classification', self.find('#semantic-list').text)
        Select(self.find('#semantic-category')).select_by_value('unresolved')
        self.assertIn('unresolved_reference', self.find('#semantic-list').text)
        self.find('#semantic-list button').click()
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'), 'test.dynamic')
        self.find('#semantic-toggle').click()
        self.find('#semantic-search').send_keys(Keys.ESCAPE)
        self.assertEqual(self.find('#semantic-toggle').get_attribute('aria-expanded'), 'false')

        def alter(document):
            start = document.index('<script id="viewer-semantic-diagnostics" type="application/json">')
            payload_start = document.index('>', start) + 1
            end = document.index('</script>', payload_start)
            payload = json.loads(document[payload_start:end])
            record = dict(payload['records'][0], address='missing.resource')
            payload.update(records=[record] * 110, total=115, omitted=5, records_complete=False)
            return document[:payload_start] + json.dumps(payload).replace('<', '\\u003c') + document[end:]

        self.load('-', '--html-semantic-diagnostics', input_plan=plan, transform=alter)
        self.find('#semantic-toggle').click()
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '#semantic-list li')), 100)
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '#semantic-list button'), [])
        self.assertIn('navigation unavailable', self.find('#semantic-list').text)
        self.assertIn('10 hidden by list limit', self.find('#semantic-status').text)
        self.assertIn('5 omitted', self.find('#semantic-status').text)
        self.load('examples/plan.json')
        self.assertFalse(self.find('#semantic-toggle').is_enabled())

    def test_ambiguous_semantic_reason_navigates_its_source_without_guessing_endpoint(self):
        plan = {'format_version': '1.2', 'resource_changes': [
            {'address': 'aws_vpc.a', 'type': 'aws_vpc'},
            {'address': 'aws_vpc.b', 'type': 'aws_vpc'},
            {'address': 'aws_subnet.ambiguous', 'type': 'aws_subnet'}],
            'configuration': {'root_module': {'resources': [
                {'address': 'aws_subnet.ambiguous', 'expressions': {
                    'vpc_id': {'references': ['aws_vpc.a.id', 'aws_vpc.b.id']}}}]}}}
        self.load('-', '--html-semantic-diagnostics', '--html-diagnostics', input_plan=plan)
        self.find('#semantic-toggle').click()
        Select(self.find('#semantic-category')).select_by_value('ambiguous')
        self.assertIn('ambiguous_containment_parent', self.find('#semantic-list').text)
        self.assertIn('Attribute: vpc_id', self.find('#semantic-list').text)
        self.find('#semantic-list button').click()
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'), 'aws_subnet.ambiguous')

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
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-account-panel]')), 0)

    def test_diagnostics_overlay_uses_report_regions_and_resets(self):
        self.load('--state', 'one=tests/fixtures/components-plan.json',
                  '--state', 'two=tests/fixtures/components-plan.json', '--html-diagnostics')
        data = self.driver.execute_script("return JSON.parse(document.getElementById('viewer-diagnostics').textContent)")
        self.assertGreater(data['total'], 0)
        self.assertLessEqual(len(data['hotspots']), 200)
        before = self.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)")
        toggle = self.find('#hotspot-toggle')
        toggle.send_keys(Keys.ENTER)
        rect = self.find('[data-hotspot-overlay] rect')
        self.assertEqual(float(rect.get_attribute('x')), data['hotspots'][0]['bounds'][0] - 6)
        self.assertEqual(float(rect.get_attribute('y')), data['hotspots'][0]['bounds'][1] - 6)
        rects = self.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay] rect')
        self.assertEqual(len(rects), len(data['hotspots']))
        for rect, hotspot in zip(rects, data['hotspots']):
            self.assertEqual(float(rect.get_attribute('y')), hotspot['bounds'][1] - 6)
        self.find('#hotspot-list button').click()
        self.assertTrue(self.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay]'))
        self.assertEqual(self.driver.switch_to.active_element.tag_name, 'button')
        self.find('#clear').click()
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay]'), [])
        self.assertEqual(before, self.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)"))
        self.choose('one:')
        self.assertEqual(toggle.get_attribute('aria-pressed'), 'false')
        self.load('examples/plan.json', '--changed-only', '--html-diagnostics')
        self.find('#hotspot-toggle').click()
        self.load('examples/plan.json')
        self.assertFalse(self.find('#hotspot-toggle').is_enabled())
        self.assertIn('unavailable', self.find('#hotspot-status').text)

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
        scope = ''.join(n.get_attribute('textContent') for n in self.driver.find_elements(By.CSS_SELECTOR, '[data-component-scope]'))
        self.assertIn('Multi-region', scope)
        for label in ['ap-northeast-1', 'us-east-1', 'Region: unknown']:
            self.assertIn(label, scope)
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-component-member]')), 3)
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

    def test_impact_dependency_depth_direction_keyboard_and_reset(self):
        plan = {'format_version': '1.2', 'resource_changes': [
            {'address': 'aws_instance.' + name, 'type': 'aws_instance', 'change': {'actions': [action]}}
            for name, action in [('a', 'create'), ('b', 'update'), ('c', 'delete')]],
            'configuration': {'root_module': {'resources': [
                {'address': 'aws_instance.' + name, 'expressions': {'input': {'references': ['aws_instance.' + dependency + '.id']}}}
                for name, dependency in [('b', 'a'), ('c', 'b')]]}}}
        self.load('-', input_plan=plan)
        self.choose('aws_instance.a')
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), [])
        Select(self.find('#impact-depth')).select_by_value('1')
        self.find('#impact-explore').send_keys(Keys.ENTER)
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 1)
        self.assertIn('aws_instance.b', self.find('#impact-results').text)
        Select(self.find('#impact-depth')).select_by_value('2')
        self.find('#impact-explore').click()
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop="2"]')), 1)
        self.assertIn('delete', self.find('#impact-results').text)
        self.choose('aws_instance.c')
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), [])
        Select(self.find('#impact-direction')).select_by_value('upstream')
        self.find('#impact-explore').click()
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 2)
        self.assertIn('Potentially related resources', self.find('#impact-status').text)
        self.search('aws_instance.b')
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), [])
        self.find('#impact-explore').click()
        self.find('#impact-explore').send_keys(Keys.ESCAPE)
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop], [data-impact-edge]'), [])
        self.load('-', '--focus', 'aws_instance.b', '--focus-depth', '0', input_plan=plan)
        self.choose('aws_instance.b')
        self.find('#impact-explore').click()
        self.assertEqual(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), [])
        self.assertIn('filtered/unavailable', self.find('#impact-status').text)
        plan_path = Path(self.temp.name) / 'chain.json'
        plan_path.write_text(json.dumps(plan))
        self.load('--state', 'one=' + str(plan_path), '--state', 'two=' + str(plan_path))
        self.choose('one:aws_instance.a')
        self.find('#impact-explore').click()
        related = self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')
        self.assertEqual(len(related), 2)
        for node in related:
            self.assertEqual(self.driver.execute_script("return arguments[0].closest('[data-state-id]').dataset.stateId", node), 'one')

    def test_impact_associations_and_cross_state_metadata(self):
        self.load('tests/fixtures/relationship-scopes-plan.json')
        edge = self.find('path[data-edge-kind="association"]')
        self.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", edge.get_attribute('data-source'))
        self.find('#impact-explore').click()
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 0)
        self.find('#impact-associations').click()
        self.find('#impact-explore').click()
        self.assertGreater(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 0)
        self.assertIn('non-causal', self.find('#impact').text)
        self.load('--state', 'producer=tests/fixtures/multi/producer.json', '--state',
                  'consumer=tests/fixtures/multi/consumer.json', '--remote-state',
                  'consumer:data.terraform_remote_state.network=producer')
        edge = self.find('path[data-cross-state]')
        target = edge.get_attribute('data-target')
        self.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", edge.get_attribute('data-source'))
        self.find('#impact-explore').click()
        self.assertTrue(self.driver.execute_script("return document.getElementById(arguments[0]).hasAttribute('data-impact-hop')", target))
        self.assertIn('consumer:', self.find('#impact-results').text)

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
        self.assertIn('Resource scope: Region: ap-northeast-1', text)
        self.assertIn('aws_route_table.private · Region: us-east-1', text)
        number = entry.find_element(By.CSS_SELECTOR, '[data-relationship-select]')
        self.driver.execute_script('arguments[0].focus()', number)
        number.send_keys(Keys.ENTER)
        self.assertEqual(self.find('[data-viewer-selected]').get_attribute('id'), entry.get_attribute('id'))
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')), 1)
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-marker] [data-viewer-related]')), 2)
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '#selection ul button')), 2)
        self.find('#selection button').click()
        self.assertTrue(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'))
        self.find('#clear').click()
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')), 0)
        after = self.driver.execute_script("return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e=>[e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end')])")
        self.assertEqual(before, after)

    def test_subscription_navigation_retains_provenance_and_endpoint_direction(self):
        self.check_messaging_navigation('sns-subscriptions', 'aws_sns_topic_subscription.jobs', 'subscription')

    def test_event_source_navigation_retains_provenance_and_endpoint_direction(self):
        self.check_messaging_navigation('event-sources', 'aws_lambda_event_source_mapping.jobs', 'eventsource')

    def test_event_target_navigation_retains_provenance_and_endpoint_direction(self):
        self.check_messaging_navigation('eventbridge-targets', 'aws_cloudwatch_event_target.jobs', 'eventtarget')

    def check_messaging_navigation(self, fixture, address, kind):
        self.load(f'tests/fixtures/{fixture}-plan.json')
        self.choose(address)
        paths = self.driver.find_elements(By.CSS_SELECTOR, f'path[data-viewer-edge][data-edge-kind="{kind}"]')
        self.assertEqual(len(paths), 1)
        self.assertEqual(paths[0].get_attribute('data-directionality'), 'directed')
        marker = self.find('[data-relationship-marker] a')
        self.driver.execute_script('arguments[0].dispatchEvent(new MouseEvent("click", {bubbles:true,cancelable:true}))', marker)
        entry = self.find('[data-viewer-selected]')
        self.assertTrue(entry.get_attribute('data-relationship-entry'))
        link = entry.find_element(By.CSS_SELECTOR, '[data-relationship-endpoint]')
        self.driver.execute_script('arguments[0].dispatchEvent(new MouseEvent("click", {bubbles:true,cancelable:true}))', link)
        self.assertTrue(self.find('[data-viewer-selected]').get_attribute('data-terraform-address'))

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

    def test_logical_action_sections_preserve_selection_and_cli_filtering(self):
        fixture = 'tests/fixtures/logical-actions-plan.json'
        self.load(fixture)
        sections = self.driver.find_elements(By.CSS_SELECTOR, '[data-resource-type="aws_iam_policy"] [data-action-section]')
        self.assertEqual([n.get_attribute('data-action-section') for n in sections],
                         ['create', 'update', 'delete', 'replace', 'read', 'unchanged', 'other'])
        self.assertIn('Create (2)', sections[0].get_attribute('textContent'))
        self.assertTrue(self.driver.execute_script("""
            return Array.from(document.querySelectorAll('[data-action-section]')).every(section => {
                const border = section.querySelector(':scope > rect').getBoundingClientRect();
                return Array.from(section.querySelectorAll('[data-placement="logical"]')).every(row => {
                    const bounds = row.getBoundingClientRect();
                    return bounds.left >= border.left && bounds.right <= border.right + 0.1 &&
                           bounds.top >= border.top && bounds.bottom <= border.bottom + 0.1;
                });
            });
        """))
        explanation = ''.join(n.get_attribute('textContent') for n in self.driver.find_elements(By.CSS_SELECTOR, '[data-role-explanation]'))
        self.assertIn('Role: Policy', explanation)
        self.assertIn('Role: Controller', explanation)
        self.choose('aws_iam_policy.update')
        self.assertEqual(self.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-action-section]').dataset.actionSection"), 'update')
        self.load(fixture, '--changed-only')
        self.assertFalse(self.driver.find_elements(By.CSS_SELECTOR, '[data-action-section="unchanged"]'))
        self.assertFalse(self.search('aws_iam_policy.unchanged'))
        self.load(fixture, '--focus', 'aws_iam_policy.create_a', '--focus-depth', '0')
        self.assertEqual(len(self.driver.find_elements(By.CSS_SELECTOR, '[data-action-section]')), 1)
        self.assertIn('Create (1)', self.find('[data-action-section]').get_attribute('textContent'))
        self.assertFalse(self.search('aws_iam_policy.create_b'))

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
