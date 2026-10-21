const assert = require('node:assert/strict');
const { mkdtempSync, rmSync, readFileSync, writeFileSync } = require('node:fs');
const { join, resolve } = require('node:path');
const { tmpdir } = require('node:os');
const { pathToFileURL } = require('node:url');
const { execFileSync } = require('node:child_process');
const { Driver, By, Keys, Select, wait } = require('./webdriver.cjs');

const ROOT = resolve(__dirname, '../..');
const BINARY = join(process.env.CARGO_TARGET_DIR || join(ROOT, 'target'), 'debug/planorama');
const len = value => value instanceof Set ? value.size : value.length;
const truth = value => Array.isArray(value) ? value.length > 0 : !!value;

class ViewerTests {
    constructor(driver, temp) { this.driver = driver; this.temp = temp; }
    assertEqual(actual, expected) { assert.deepStrictEqual(actual, expected); }
    assertNotEqual(actual, expected) { assert.notDeepStrictEqual(actual, expected); }
    assertTrue(value, message) { assert.ok(truth(value), message); }
    assertFalse(value) { assert.ok(!truth(value)); }
    assertIn(part, value) { assert.ok(value.includes(part), JSON.stringify({part, value})); }
    assertNotIn(part, value) { assert.ok(!value.includes(part), JSON.stringify({part, value})); }
    assertGreater(a, b) { assert.ok(a > b, a + ' > ' + b); }
    assertLessEqual(a, b) { assert.ok(a <= b, a + ' <= ' + b); }
    assertIsNone(value) { assert.equal(value, null); }
    find(selector) { return this.driver.find_element(By.CSS_SELECTOR, selector); }
    view_box() { return this.driver.execute_script("return document.querySelector('#viewport > svg').getAttribute('viewBox')"); }
    load(...args) {
        const options = typeof args.at(-1) === 'object' ? args.pop() : {};
        const output = join(this.temp, 'viewer.html');
        execFileSync(BINARY, [...args, '--format', 'html', '-o', output], {
            input: options.input_plan ? JSON.stringify(options.input_plan) : undefined,
            encoding: 'utf8', cwd: ROOT, timeout: 120000, maxBuffer: 16 * 1024 * 1024,
        });
        if (options.transform) writeFileSync(output, options.transform(readFileSync(output, 'utf8')));
        this.driver.get(pathToFileURL(output).href);
        wait(() => this.find('#result-count').text.includes('resources'), 'viewer resources');
    }
    check_logs() {
        const errors = this.driver.get_log('browser').filter(entry => entry.level === 'SEVERE');
        const requests = this.driver.get_log('performance').map(entry => JSON.parse(entry.message).message);
        const remote = requests.filter(entry => entry.method === 'Network.requestWillBeSent' && /^https?:/.test(entry.params.request.url)).map(entry => entry.params.request.url);
        assert.deepEqual(errors, []);
        assert.deepEqual(remote, []);
    }
    test_spatial_types_search_reference_navigation_and_exports() {
        this.load('tests/fixtures/identity-providers-plan.json',
            '--spatial-type', 'aws_iam_role',
            '--spatial-type', 'aws_iam_openid_connect_provider',
            '--spatial-type', 'aws_iam_saml_provider');
        const selected = this.choose('aws_iam_role.workload');
        const reference = this.find(`[id="${selected}"] a[href]`);
        const target = reference.get_attribute('href').split('#').at(-1);
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", reference);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), target);
        for (const kind of ['neighborhood', 'viewport']) {
            this.driver.execute_script(`
                document.getElementById('export-depth').value = '1';
                window.beforeExport = document.querySelector('#viewport').innerHTML;
                window.exportText = null;
                URL.createObjectURL = blob => { window.exportBlob = blob; return 'blob:export-test'; };
                HTMLAnchorElement.prototype.click = function () {};
            `);
            this.find('#export-' + kind).click();
            this.assertIn('download requested', this.find('#export-status').text);
            this.driver.execute_script('window.exportBlob.text().then(text => { window.exportText = text; })');
            wait(() => this.driver.execute_script('return !!window.exportText'), 'Spatial export');
            const result = this.driver.execute_script(`
                const doc = new DOMParser().parseFromString(window.exportText, 'image/svg+xml');
                return {
                    unchanged: window.beforeExport === document.querySelector('#viewport').innerHTML,
                    reference: doc.documentElement.textContent.includes('References: aws_iam_openid_connect_provider.workload'),
                    broken: [...doc.querySelectorAll('[href]')].filter(el => !doc.getElementById(el.getAttribute('href').slice(1))).length,
                    private: window.exportText.includes('PRIVATE_'),
                    invalid: doc.querySelectorAll('parsererror').length
                };
            `);
            this.assertTrue(result.unchanged);
            this.assertTrue(result.reference);
            this.assertEqual(result.broken, 0);
            this.assertEqual(result.invalid, 0);
            this.assertFalse(result.private);
        }
        this.load('tests/fixtures/identity-providers-plan.json', '--focus', 'aws_iam_role.workload', '--focus-depth', '0', '--spatial-type', 'aws_iam_role', '--spatial-type', 'aws_iam_openid_connect_provider');
        this.assertIn('Unavailable', this.find('#viewport').text);
        this.assertNotIn('PRIVATE_', this.find('#viewport').text);
    }
    test_neighborhood_export_preserves_presentations_and_removes_unrelated_resources() {
        for (const fixture of ['viewer', 'routes', 'association', 'deposed', 'aurora', 'efs', 'kms', 'secrets-manager', 'private-ca', 'acm', 'certificate-cards', 'glue', 'athena', 'organizations']) {
            this.load('--state', 'one=tests/fixtures/' + fixture + '-plan.json', '--state', 'two=tests/fixtures/' + fixture + '-plan.json');
            const candidates = this.driver.execute_script(`return [...document.querySelectorAll('#results button')].map(b => b.textContent)`);
            for (const address of candidates.slice(0, fixture === 'private-ca' ? 6 : 4)) {
                const result = this.driver.find_elements(By.CSS_SELECTOR, '#results button').find(b => b.text === address);
                result.click();
                this.driver.execute_script(`
                    document.getElementById('export-depth').value = '0';
                    window.beforeExport = document.querySelector('#viewport').innerHTML;
                    window.exportText = null; window.exportBlob = null;
                    URL.createObjectURL = blob => { window.exportBlob = blob; return 'blob:export-test'; };
                    HTMLAnchorElement.prototype.click = function () {};
                `);
                this.find('#export-neighborhood').send_keys(Keys.ENTER);
                this.assertIn('download requested', this.find('#export-status').text);
                this.driver.execute_script('window.exportBlob.text().then(text => { window.exportText = text; })');
                wait(() => this.driver.execute_script('return !!window.exportText'), 'neighborhood serialization');
                const checks = this.driver.execute_script(`
                    const doc = new DOMParser().parseFromString(window.exportText, 'image/svg+xml');
                    const meta = JSON.parse(doc.querySelector('metadata').textContent);
                    const ids = [...doc.querySelectorAll('[id]')].map(el => el.id);
                    const paths = [...doc.querySelectorAll('path[data-edge-kind]')];
                    const live = document.querySelector('#viewport');
                    return {
                        unchanged: live.innerHTML === window.beforeExport,
                        duplicates: ids.length !== new Set(ids).size,
                        selected: meta.resources.includes(document.querySelector('[data-viewer-selected]').id),
                        foreignState: [...doc.querySelectorAll('[data-state-id]')].some(el => el.dataset.stateId !== document.querySelector('[data-viewer-selected]').closest('[data-state-id]').dataset.stateId),
                        broken: [...doc.querySelectorAll('[href]')].filter(el => !doc.getElementById(el.getAttribute('href').slice(1))).length,
                        invalid: doc.querySelectorAll('parsererror, script, [data-viewer-selected], [data-hotspot-overlay]').length,
                        count: meta.resources.length,
                        targets: meta.resources.every(id => !!doc.getElementById(id)),
                        resources: [...doc.querySelectorAll('[data-terraform-address][id], [data-source-identity][id]')].every(el => meta.resources.includes(el.id)),
                        masks: [...doc.querySelectorAll('[data-occluder]')].every(el => meta.resources.includes(el.dataset.occluder)),
                        context: JSON.parse(document.getElementById('viewer-data').textContent).filter(r => meta.resources.includes(r.id)).every(r => r.context.every(id => meta.resources.includes(id))),
                        paths: paths.every(path => [...live.querySelectorAll('path[data-edge-kind]')].some(original => original.getAttribute('d') === path.getAttribute('d') && original.dataset.source === path.dataset.source && original.dataset.target === path.dataset.target)),
                        summary: doc.documentElement.textContent.includes('Original layout, no repacking')
                    };
                `);
                this.assertTrue(checks.unchanged); this.assertFalse(checks.duplicates); this.assertTrue(checks.selected);
                this.assertFalse(checks.foreignState); this.assertEqual(checks.broken, 0); this.assertEqual(checks.invalid, 0);
                this.assertTrue(checks.targets); this.assertTrue(checks.resources); this.assertTrue(checks.masks); this.assertTrue(checks.context);
                this.assertTrue(checks.paths); this.assertTrue(checks.summary);
            }
        }
    }
    test_neighborhood_cross_state_scope_filtering_and_cancellation() {
        this.load('--state', 'producer=tests/fixtures/multi/producer.json', '--state', 'consumer=tests/fixtures/multi/consumer.json', '--remote-state', 'consumer:data.terraform_remote_state.network=producer');
        this.driver.execute_script(`
            const path = document.querySelector('path[data-cross-state]');
            document.getElementById(path.dataset.source).dispatchEvent(new MouseEvent('click', {bubbles:true}));
            URL.createObjectURL = blob => { window.exportBlob = blob; return 'blob:export-test'; };
            HTMLAnchorElement.prototype.click = function () {};
        `);
        this.find('#export-neighborhood').click();
        this.assertIn('download requested', this.find('#export-status').text);
        this.driver.execute_script('window.exportBlob.text().then(text => { window.exportText = text; })');
        wait(() => this.driver.execute_script('return !!window.exportText'), 'cross-state export');
        const checks = this.driver.execute_script(`
            const doc = new DOMParser().parseFromString(window.exportText, 'image/svg+xml');
            const meta = JSON.parse(doc.querySelector('metadata').textContent);
            return {states: doc.querySelectorAll('[data-state-id]').length, paths: doc.querySelectorAll('[data-cross-state]').length,
                remote: meta.relationships.some(edge => edge.provenance.some(p => p.kind === 'remote-state')),
                dangling: [...doc.querySelectorAll('[href]')].some(el => !doc.getElementById(el.getAttribute('href').slice(1)))};
        `);
        this.assertEqual(checks.states, 2); this.assertGreater(checks.paths, 0); this.assertTrue(checks.remote); this.assertFalse(checks.dangling);
        const standalone = join(this.temp, 'neighborhood.svg');
        writeFileSync(standalone, this.driver.execute_script('return window.exportText'));
        this.driver.get(pathToFileURL(standalone).href);
        this.assertEqual(this.driver.execute_script('return document.documentElement.localName'), 'svg');
        this.load('tests/fixtures/viewer-plan.json', '--focus', 'aws_ecs_service.app', '--focus-depth', '0');
        this.find('#export-neighborhood').click();
        this.assertIn('Select a resource', this.find('#export-status').text);
        this.find('#results button').click();
        this.driver.execute_script(`
            URL.createObjectURL = blob => { window.exportBlob = blob; return 'blob:export-test'; };
            HTMLAnchorElement.prototype.click = function () {};
        `);
        this.find('#export-neighborhood').click();
        this.assertIn('download requested', this.find('#export-status').text);
        this.driver.execute_script('window.exportBlob.text().then(text => { window.exportText = text; })');
        wait(() => this.driver.execute_script('return !!window.exportText'), 'filtered export');
        this.assertTrue(this.driver.execute_script(`
            const doc = new DOMParser().parseFromString(window.exportText, 'image/svg+xml');
            const source = new Set(JSON.parse(document.getElementById('viewer-data').textContent).map(r => r.id));
            return JSON.parse(doc.querySelector('metadata').textContent).resources.every(id => source.has(id));
        `));
        this.driver.execute_script(`
            const selected = document.querySelector('[data-viewer-selected]');
            selected.getBBox = () => { throw new Error('Geometry unavailable'); };
            for (const el of selected.querySelectorAll('*')) if (el.getBBox) el.getBBox = selected.getBBox;
            window.exportBlob = null;
        `);
        this.find('#export-neighborhood').click();
        this.assertIn('Export cancelled', this.find('#export-status').text);
        this.assertIsNone(this.driver.execute_script('return window.exportBlob'));
    }
    test_viewport_export_is_standalone_and_preserves_live_view() {
        for (const fixture of ['viewer', 'routes', 'association', 'deposed', 'efs']) {
            this.load('--state', 'one=tests/fixtures/' + fixture + '-plan.json', '--state', 'two=tests/fixtures/' + fixture + '-plan.json', '--html-diagnostics');
            this.find('#results button').send_keys(Keys.ENTER);
            this.find('#zoom-in').click();
            this.driver.execute_script(`
                window.beforeExport = document.querySelector('#viewport').innerHTML;
                URL.createObjectURL = blob => { window.exportBlob = blob; return 'blob:export-test'; };
                HTMLAnchorElement.prototype.click = function () { window.exportName = this.download; };
            `);
            this.find('#export-viewport').send_keys(Keys.ENTER);
            this.assertIn('download requested', this.find('#export-status').text);
            this.driver.execute_script('window.exportBlob.text().then(text => { window.exportText = text; })');
            wait(() => this.driver.execute_script('return !!window.exportText'), 'export serialization');
            const result = this.driver.execute_script(`
                const exported = new DOMParser().parseFromString(window.exportText, 'image/svg+xml');
                const live = document.querySelector('#viewport > svg');
                return {
                    unchanged: window.beforeExport === document.querySelector('#viewport').innerHTML,
                    viewBox: exported.documentElement.getAttribute('viewBox') === live.getAttribute('viewBox'),
                    invalid: exported.querySelectorAll('parsererror, script, foreignObject, [data-viewer-selected], [data-impact-hop], [data-hotspot-overlay]').length,
                    dangling: [...exported.querySelectorAll('[href]')].filter(el => !exported.getElementById(el.getAttribute('href').slice(1))).length,
                    paths: [...exported.querySelectorAll('path[data-edge-kind]')].map(el => [el.getAttribute('d'), el.getAttribute('stroke'), el.getAttribute('marker-end')]),
                    originalPaths: [...live.querySelectorAll('path[data-edge-kind]')].map(el => [el.getAttribute('d'), el.getAttribute('stroke'), el.getAttribute('marker-end')]),
                    name: window.exportName,
                    imageRoles: [exported.documentElement, ...exported.querySelectorAll('svg svg')].every(el => el.getAttribute('role') === 'img'),
                    imageNames: [exported.documentElement, ...exported.querySelectorAll('svg svg')].map(el => el.getAttribute('aria-labelledby') || el.getAttribute('aria-label'))
                };
            `);
            this.assertTrue(result.unchanged); this.assertTrue(result.viewBox);
            this.assertEqual(result.invalid, 0); this.assertEqual(result.dangling, 0);
            this.assertEqual(result.paths, result.originalPaths);
            this.assertEqual(result.name, 'planorama-viewport.svg');
            this.assertTrue(result.imageRoles);
            this.assertEqual(result.imageNames, ['Current viewport; content outside this viewBox is intentionally cropped. Selection and diagnostic overlays are omitted.', 'State: one', 'State: two']);
            const exportedFile = join(this.temp, 'export.svg');
            writeFileSync(exportedFile, this.driver.execute_script('return window.exportText'));
            this.driver.get(pathToFileURL(exportedFile).href);
            this.assertEqual(this.driver.execute_script('return document.documentElement.localName'), 'svg');
        }
        this.load('tests/fixtures/viewer-plan.json');
        this.driver.execute_script('URL.createObjectURL = undefined');
        this.find('#export-viewport').click();
        this.assertIn('unsupported', this.find('#export-status').text);
        this.driver.execute_script("URL.createObjectURL = () => { throw new Error('download denied'); }");
        this.find('#export-viewport').click();
        this.assertIn('download denied', this.find('#export-status').text);
        this.load('-', {input_plan: {format_version: '1.2', resource_changes: [{address: 'test.item["url(foo)"]', type: 'test'}]}});
        this.driver.execute_script(`
            URL.createObjectURL = () => 'blob:export-test';
            HTMLAnchorElement.prototype.click = function () {};
        `);
        this.find('#export-viewport').click();
        this.assertIn('download requested', this.find('#export-status').text);
        this.driver.execute_script(`
            const svg = document.querySelector('#viewport > svg');
            const clone = svg.cloneNode.bind(svg);
            svg.cloneNode = deep => { const copy = clone(deep); copy.setAttribute('filter', 'URL(https://invalid.example/filter)'); return copy; };
        `);
        this.find('#export-viewport').click();
        this.assertIn('Unavailable SVG definition', this.find('#export-status').text);
    }
    test_copy_native_address_success_rejection_and_unavailable() {
        let address = 'module.demo["日本語"].test.item["a\\\"b"]';
        let plan = {'format_version': '1.2', 'resource_changes': [ {'address': address, 'type': 'test', 'deposed': 'old'}]};
        this.load('-', { input_plan: plan });
        this.find('#results button').send_keys(Keys.ENTER);
        this.assertEqual(this.find('.copy-identity textarea').get_attribute('value'), address);
        this.assertIn('deposed: old', this.find('.copy-identity').text);
        let before = this.view_box();
        for (const mode of ['success', 'rejected', 'unavailable']) {
            this.driver.execute_script(`
                window.copied = null;
                const mode = arguments[0];
                Object.defineProperty(navigator, 'clipboard', {configurable: true, value:
                    mode === 'unavailable' ? undefined : {writeText: async text => {
                        if (mode === 'rejected') throw new Error('denied');
                        window.copied = text;
                    }}});
            `, mode);
            this.find('.copy-identity button').send_keys(Keys.ENTER);
            if (mode == 'success') {
                this.assertEqual(this.driver.execute_script('return window.copied'), address);
                this.assertIn('address copied', this.find('.copy-identity [role=status]').text);
            }
            else {
                this.assertIn('manually', this.find('.copy-identity [role=status]').text);
                this.assertEqual(this.driver.execute_script('return document.activeElement.value'), address);
                this.assertEqual(this.driver.execute_script('return document.activeElement.selectionEnd'), len(address));
            }
            this.assertEqual(before, this.view_box());
        }
        this.find('#clear').click();
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '.copy-identity'), []);
        this.load('-', '--address-format', 'terraform', { input_plan: plan });
        this.find('#results button').send_keys(Keys.ENTER);
        this.assertEqual(this.find('.copy-identity textarea').get_attribute('value'), address);
        this.driver.execute_script("Object.defineProperty(navigator, 'clipboard', {configurable: true, value: {writeText: () => new Promise((resolve, reject) => {window.rejectCopy = reject;})}})");
        this.find('.copy-identity button').click();
        this.find('#clear').click();
        this.driver.execute_script("window.rejectCopy(new Error('late rejection'))");
        this.assertEqual(this.find('#selection').text, '');
        this.assertEqual(this.driver.switch_to.active_element.get_attribute('id'), 'clear');
    }
    test_copy_targets_cover_resource_presentations_and_relationship_roles() {
        for (const fixture of ['viewer', 'routes', 'association', 'deposed']) {
            this.load('--state', 'one=tests/fixtures/' + fixture + '-plan.json', '--state', 'two=tests/fixtures/' + fixture + '-plan.json');
            let buttons = this.driver.find_elements(By.CSS_SELECTOR, '#results button');
            for (const result of buttons) {
                result.click();
                let selected = this.find('[data-viewer-selected]');
                let native = selected.get_attribute('data-terraform-address') || selected.get_attribute('data-source-address');
                this.assertEqual(this.find('.copy-identity textarea').get_attribute('value'), native);
                this.assertIn('State: ' + selected.find_element(By.XPATH, './ancestor::*[@data-state-id]').get_attribute('data-state-id'), this.find('.copy-identity').text);
            }
            let relationships = this.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-entry] [data-relationship-select]');
            if (relationships.length) {
                relationships[0].send_keys(Keys.ENTER);
                let groups = this.driver.find_elements(By.CSS_SELECTOR, '.copy-identity');
                this.assertTrue(groups);
                this.assertTrue(groups.every(group => group.text.includes('Endpoint resource') || group.text.includes('Provenance resource')));
                let names = [];
                for (const group of groups) {
                    let name = group.find_element(By.TAG_NAME, 'button').accessible_name;
                    this.assertIn(group.find_element(By.TAG_NAME, 'textarea').get_attribute('value'), name);
                    this.assertIn(group.find_element(By.TAG_NAME, 'p').text, name);
                    names.push(name);
                }
                this.assertEqual(len(names), len(new Set(names)));
            }
        }
    }
    test_search_facets_combine_reset_and_preserve_selection_and_geometry() {
        let actions = [['create', ['create']], ['update', ['update']], ['delete', ['delete']], ['replace', ['delete', 'create']], ['read', ['read']], ['unchanged', ['no-op']], ['other', ['unexpected']]];
        let plan = {'format_version': '1.2', 'resource_changes': actions.map(([name, action]) => ({address: 'test.' + name, type: 'test', provider_name: 'example.com/a/test', change: {actions: action}}))};
        plan.resource_changes.push(...[ {'address': 'test.create', 'type': 'test', 'provider_name': 'example.com/b/test', 'deposed': 'old', 'change': {'actions': ['create']}}, {'address': 'data.test.lookup', 'type': 'test', 'mode': 'data', 'provider_name': 'example.com/b/test', 'change': {'actions': ['read']}}]);
        let path = join(this.temp, 'facets.json');
        writeFileSync(path, JSON.stringify(plan));
        this.load('--state', 'one=' + String(path), '--state', 'two=' + String(path));
        let geometry = "return [...document.querySelectorAll('#viewport svg *')].map(e=>['d','x','y','width','height','transform'].map(a=>e.getAttribute(a)))";
        let before = this.driver.execute_script(geometry);
        this.assertEqual(this.find('#result-count').text, '18 of 18 resources');
        Select(this.find('#search-state')).select_by_visible_text('two');
        for (const [action, _] of actions) {
            Select(this.find('#search-action')).select_by_value(action);
            let expected = ['create', 'read'].includes(action) ? 2 : 1;
            this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '#results button')), expected);
        }
        Select(this.find('#search-action')).select_by_value('create');
        Select(this.find('#search-type')).select_by_visible_text('test · example.com/b/test');
        this.find('#search').send_keys('old', Keys.ARROW_DOWN);
        this.driver.switch_to.active_element.send_keys(Keys.ENTER);
        let selected = this.find('[data-viewer-selected]').get_attribute('id');
        this.assertIn('two:', this.find('#selection h2').text);
        this.assertIn('deposed: old', this.find('#selection h2').text);
        let box = this.view_box();
        Select(this.find('#search-action')).select_by_value('read');
        this.assertIn('0 of 18', this.find('#result-count').text);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), selected);
        this.assertEqual(this.view_box(), box);
        this.find('#search-reset').send_keys(Keys.ENTER);
        this.assertEqual(this.find('#result-count').text, '18 of 18 resources');
        this.assertEqual(this.driver.switch_to.active_element.get_attribute('id'), 'search');
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), selected);
        this.assertEqual(this.driver.execute_script(geometry), before);
        this.find('#search').send_keys('data.test.lookup');
        Select(this.find('#search-state')).select_by_visible_text('one');
        this.find('#search').send_keys(Keys.ENTER);
        this.assertIn('data', this.find('#selection h2').text);
    }
    test_search_facets_include_lowered_resources_and_never_restore_cli_exclusions() {
        this.load('tests/fixtures/association-plan.json');
        Select(this.find('#search-type')).select_by_visible_text('aws_route_table_association · registry.terraform.io/hashicorp/aws');
        this.assertTrue(this.driver.find_elements(By.CSS_SELECTOR, '#results button'));
        this.find('#search').send_keys(Keys.ENTER);
        this.assertIn('Rendered relationships from this resource', this.find('#selection').text);
        let plan = {'format_version': '1.2', 'resource_changes': Array.from({length: 105}, (_, i) => ({address: 'aws_iam_role.item' + i, type: 'aws_iam_role', change: {actions: i === 0 ? ['create'] : ['no-op']}}))};
        this.load('-', { input_plan: plan });
        this.assertEqual(this.find('#result-count').text, '105 of 105 resources');
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '#results button')), 105);
        Select(this.find('#search-action')).select_by_value('unchanged');
        this.assertEqual(this.find('#result-count').text, '104 of 105 resources');
        this.find('#search').send_keys(Keys.ARROW_DOWN);
        this.driver.switch_to.active_element.send_keys(Keys.END, Keys.ENTER);
        this.assertIn('unchanged', this.find('#selection').text.toLowerCase());
        for (const flags of [['--changed-only',], ['--focus', 'aws_iam_role.item0', '--focus-depth', '0']]) {
            this.load('-', ...flags, { input_plan: plan });
            this.find('#search-reset').click();
            this.assertEqual(this.find('#result-count').text, '1 of 1 resources');
            Select(this.find('#search-action')).select_by_value('unchanged');
            this.assertIn('0 of 1', this.find('#result-count').text);
        }
    }
    test_selection_history_restores_resources_relationships_provenance_and_viewports() {
        this.load('tests/fixtures/association-plan.json', '--html-diagnostics', '--html-semantic-diagnostics');
        this.assertFalse(this.find('#selection-back').is_enabled());
        this.assertFalse(this.find('#selection-forward').is_enabled());
        let visits = [];
        for (const address of ['aws_subnet.private', 'aws_route_table.private']) {
            let selected = this.choose(address);
            this.find('#zoom-in').click();
            this.find('[data-viewer-selected]').send_keys(Keys.ARROW_RIGHT);
            visits.push([selected, this.view_box()]);
        }
        this.find('[data-relationship-entry] [data-relationship-select]').send_keys(Keys.ENTER);
        this.find('#zoom-out').click();
        visits.push([this.find('[data-viewer-selected]').get_attribute('id'), this.view_box()]);
        this.find('#selection ul button').send_keys(Keys.ENTER);
        visits.push([this.find('[data-viewer-selected]').get_attribute('id'), this.view_box()]);
        this.find('#impact-associations').click();
        this.find('#impact-explore').click();
        this.assertTrue(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'));
        for (const [selected, box] of visits.slice(0, -1).reverse()) {
            this.find('#selection-back').send_keys(Keys.ENTER);
            this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), selected);
            this.assertEqual(this.driver.switch_to.active_element.get_attribute('id'), selected);
            this.assertEqual(this.view_box(), box);
            this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop], [data-impact-edge]'));
        }
        this.assertFalse(this.find('#selection-back').is_enabled());
        for (const [selected, box] of visits.slice(1)) {
            this.find('#selection-forward').send_keys(Keys.ENTER);
            this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), selected);
            this.assertEqual(this.view_box(), box);
        }
        this.assertFalse(this.find('#selection-forward').is_enabled());
        this.choose('aws_route_table_association.private');
        let provenance = this.find('[data-viewer-selected]').get_attribute('id');
        this.find('#selection-back').click();
        this.find('#hotspot-toggle').click();
        this.assertTrue(this.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay]'));
        this.find('#selection-forward').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), provenance);
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay]'));
        this.find('#selection-back').click();
        this.find('#semantic-toggle').click();
        this.find('#selection-forward').click();
        this.assertEqual(this.find('#semantic-toggle').get_attribute('aria-expanded'), 'false');
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), provenance);
    }
    test_selection_history_branching_bounds_missing_targets_and_keyboard_reset() {
        this.load('--state', 'one=tests/fixtures/deposed-plan.json', '--state', 'two=tests/fixtures/deposed-plan.json');
        let resources = this.driver.execute_script("return JSON.parse(document.getElementById('viewer-data').textContent)");
        let old = resources.find(r => r['state'] == 'one' && r['deposed']);
        let current = resources.find(r => r['state'] == 'one' && !r['deposed'] && r['address'] == old['address']);
        let other = resources.find(r => r['state'] == 'two' && r['deposed'] == old['deposed'] && r['address'] == old['address']);
        const click_id = (identifier) => {
            this.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", identifier);
        }
        for (const resource of [old, current, other]) {
            click_id(resource['id']);
        }
        this.assertIn('3 of 3', this.find('#history-status').text);
        click_id(other['id']);
        this.find('#zoom-in').click();
        this.assertIn('3 of 3', this.find('#history-status').text);
        this.find('#selection-back').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), current['id']);
        click_id(current['id']);
        this.assertIn('2 of 3', this.find('#history-status').text);
        this.assertTrue(this.find('#selection-forward').is_enabled());
        this.find('#selection-back').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), old['id']);
        this.find('#search-reset').click();
        this.find('#fit').click();
        this.assertTrue(this.find('#selection-forward').is_enabled());
        click_id(other['id']);
        this.assertIn('2 of 2', this.find('#history-status').text);
        this.assertFalse(this.find('#selection-forward').is_enabled());
        let prevented = this.driver.execute_script(`
            const event = new KeyboardEvent('keydown', {key:'ArrowLeft', altKey:true, bubbles:true, cancelable:true});
            document.querySelector('[data-viewer-selected]').dispatchEvent(event);
            return event.defaultPrevented;
        `);
        this.assertFalse(prevented);
        this.driver.execute_script("document.getElementById(arguments[0]).remove()", old['id']);
        this.find('#selection-back').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), other['id']);
        this.assertFalse(this.find('#selection-back').is_enabled());
        this.find('#clear').click();
        this.assertEqual(this.find('#history-status').text, 'No selection history');
        this.driver.execute_script(`
            for (let i=0; i<105; i++) document.getElementById(arguments[i%2]).dispatchEvent(new MouseEvent('click', {bubbles:true}));
        `, current['id'], other['id']);
        this.assertEqual(this.find('#history-status').text, 'Selection 100 of 100');
        this.find('#search').send_keys(Keys.ESCAPE);
        this.assertFalse(this.find('#selection-back').is_enabled());
        this.assertFalse(this.find('#selection-forward').is_enabled());
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]'));
    }
    test_service_names_have_space_in_named_state_resource_rows() {
        this.load('--state', 'services=tests/fixtures/data-icons-plan.json', '--state', 'builtin=tests/fixtures/terraform-plan.json', '--icon-mode', 'service-name');
        let result = this.driver.execute_script(`
            const svg = document.querySelector('#viewport > svg');
            const viewport = document.querySelector('#viewport');
            svg.setAttribute('viewBox', \`0 0 \${viewport.clientWidth} \${viewport.clientHeight}\`);
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
        `);
        this.assertGreater(result['count'], 10);
        this.assertEqual(result['overlaps'], []);
        this.assertEqual(result['symbols'], 0);
    }
    test_coverage_summary_is_absent_from_empty_and_single_resource_diagrams() {
        for (const changes of [[], [{"address": "terraform_data.one", "type": "terraform_data"}]]) {
            this.load('-', {input_plan: {"format_version": "1.2", "resource_changes": changes}});
            this.assertFalse(this.driver.execute_script("return !!document.querySelector('[data-view-coverage]')"));
            this.assertNotIn('Diagram Coverage', this.find('svg').text);
        }
    }
    test_hidden_relationship_legend_reclaims_space_and_keeps_annotations_clear() {
        this.load('tests/fixtures/routes-plan.json');
        let [full_height, full_legend] = this.driver.execute_script("return [document.querySelector('svg').viewBox.baseVal.height, +document.querySelector('#relationship-legend rect').getAttribute('height')]");
        this.load('tests/fixtures/routes-plan.json', '--hide-relationship-legend');
        let compact = this.driver.execute_script(`
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
        `);
        this.assertEqual(full_height - compact['height'], full_legend - compact['legend']);
        this.assertGreater(full_height, compact['height']);
        this.assertFalse(compact['overlap']);
        this.assertFalse(compact['outside']);
        this.assertGreater(compact['entries'], 0);
        this.assertEqual(compact['explanations'], 0);
    }
    test_semantic_diagnostics_navigate_all_resource_presentations_and_reset() {
        for (const [fixture, address] of [['containment', 'aws_instance.app'], ['logical-regions', 'aws_iam_policy'], ['routes', 'aws_route.'], ['association', 'aws_route_table_association.private']]) {
            this.load('--state', 'one=tests/fixtures/' + fixture + '-plan.json', '--state', 'two=tests/fixtures/' + fixture + '-plan.json', '--html-semantic-diagnostics');
            let before = this.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)");
            this.find('#semantic-toggle').send_keys(Keys.ENTER);
            this.find('#semantic-search').send_keys('two ' + address);
            let button = this.find('#semantic-list button');
            this.assertIn('two:', button.text);
            button.send_keys(Keys.ENTER);
            let selected = this.find('[data-viewer-selected]');
            this.assertEqual(selected.find_element(By.XPATH, './ancestor::*[@data-state-id]').get_attribute('data-state-id'), 'two');
            this.assertEqual(this.find('#semantic-toggle').get_attribute('aria-expanded'), 'false');
            this.assertEqual(this.driver.switch_to.active_element.get_attribute('id'), selected.get_attribute('id'));
            this.find('#clear').click();
            this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]'), []);
            this.assertEqual(before, this.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)"));
        }
    }
    test_semantic_filters_deposed_identity_bounds_unavailable_and_safe_text() {
        let hostile = 'test.item["</script><script>alert(1)</script>"]';
        let plan = {'format_version': '1.2', 'resource_changes': [ {'address': hostile, 'type': 'test'}, {'address': hostile, 'type': 'test', 'deposed': 'old'}, {'address': 'test.dynamic', 'type': 'test'}], 'configuration': {'root_module': {'resources': [ {'address': 'test.dynamic', 'expressions': { 'input': {'references': ['each.value']}, 'missing': {'references': ['test.absent.id']}}}]}}};
        this.load('-', '--html-semantic-diagnostics', { input_plan: plan });
        this.find('#semantic-toggle').click();
        this.find('#semantic-search').send_keys('old');
        this.find('#semantic-list button').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-deposed-key'), 'old');
        this.find('#semantic-toggle').click();
        this.find('#semantic-search').clear();
        Select(this.find('#semantic-category')).select_by_value('dynamic');
        this.assertIn('dynamic_instance_selection', this.find('#semantic-list').text);
        this.assertNotIn('unsupported_classification', this.find('#semantic-list').text);
        Select(this.find('#semantic-category')).select_by_value('unresolved');
        this.assertIn('unresolved_reference', this.find('#semantic-list').text);
        this.find('#semantic-list button').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'), 'test.dynamic');
        this.find('#semantic-toggle').click();
        this.find('#semantic-search').send_keys(Keys.ESCAPE);
        this.assertEqual(this.find('#semantic-toggle').get_attribute('aria-expanded'), 'false');
        function alter(document) {
            let start = document.indexOf('<script id="viewer-semantic-diagnostics" type="application/json">');
            let payload_start = document.indexOf('>', start) + 1;
            let end = document.indexOf('</script>', payload_start);
            let payload = JSON.parse(document.slice(payload_start, end));
            let record = {...payload.records[0], address: 'missing.resource'};
            Object.assign(payload, {records: Array(110).fill(record), total: 115, omitted: 5, records_complete: false});
            return document.slice(0, payload_start) + JSON.stringify(payload).replaceAll('<', '\\u003c') + document.slice(end);
        }
        this.load('-', '--html-semantic-diagnostics', { input_plan: plan, transform: alter });
        this.find('#semantic-toggle').click();
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '#semantic-list li')), 100);
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '#semantic-list button'), []);
        this.assertIn('navigation unavailable', this.find('#semantic-list').text);
        this.assertIn('10 hidden by list limit', this.find('#semantic-status').text);
        this.assertIn('5 omitted', this.find('#semantic-status').text);
        this.load('examples/plan.json');
        this.assertFalse(this.find('#semantic-toggle').is_enabled());
    }
    test_ambiguous_semantic_reason_navigates_its_source_without_guessing_endpoint() {
        let plan = {'format_version': '1.2', 'resource_changes': [ {'address': 'aws_vpc.a', 'type': 'aws_vpc'}, {'address': 'aws_vpc.b', 'type': 'aws_vpc'}, {'address': 'aws_subnet.ambiguous', 'type': 'aws_subnet'}], 'configuration': {'root_module': {'resources': [ {'address': 'aws_subnet.ambiguous', 'expressions': { 'vpc_id': {'references': ['aws_vpc.a.id', 'aws_vpc.b.id']}}}]}}};
        this.load('-', '--html-semantic-diagnostics', '--html-diagnostics', { input_plan: plan });
        this.find('#semantic-toggle').click();
        Select(this.find('#semantic-category')).select_by_value('ambiguous');
        this.assertIn('ambiguous_containment_parent', this.find('#semantic-list').text);
        this.assertIn('Attribute: vpc_id', this.find('#semantic-list').text);
        this.find('#semantic-list button').click();
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'), 'aws_subnet.ambiguous');
    }
    search(text) {
        let field = this.find('#search');
        field.clear();
        field.send_keys(text);
        return this.driver.find_elements(By.CSS_SELECTOR, '#results button');
    }
    choose(text) {
        let matches = this.search(text);
        this.assertTrue(matches, text);
        matches[0].click();
        return this.find('#viewport [data-viewer-selected]').get_attribute('id');
    }
    test_maintained_multi_account_example_search_selection_and_fit() {
        this.driver.get(pathToFileURL(join(ROOT, 'examples/multi-account/diagram.html')).href);
        let before = this.view_box();
        let matches = this.search('module.network.aws_vpc.main');
        this.assertEqual(len(matches), 3);
        let selected = new Set();
        for (const state of ['application-a', 'application-b', 'shared-services']) {
            selected.add(this.choose(state + ':module.network.aws_vpc.main'));
            this.assertEqual(this.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-state-id]').dataset.stateId"), state);
        }
        this.assertEqual(len(selected), 3);
        this.choose('application-a:aws_s3_bucket.regional_archive');
        this.find('#zoom-in').click();
        this.assertNotEqual(before, this.view_box());
        this.find('#fit').click();
        this.assertEqual(before, this.view_box());
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-account-panel]')), 0);
    }
    test_diagnostics_overlay_uses_report_regions_and_resets() {
        this.load('--state', 'one=tests/fixtures/components-plan.json', '--state', 'two=tests/fixtures/components-plan.json', '--html-diagnostics');
        let data = this.driver.execute_script("return JSON.parse(document.getElementById('viewer-diagnostics').textContent)");
        this.assertGreater(data['total'], 0);
        this.assertLessEqual(len(data['hotspots']), 200);
        let before = this.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)");
        let toggle = this.find('#hotspot-toggle');
        toggle.send_keys(Keys.ENTER);
        let rect = this.find('[data-hotspot-overlay] rect');
        this.assertEqual(Number(rect.get_attribute('x')), data['hotspots'][0]['bounds'][0] - 6);
        this.assertEqual(Number(rect.get_attribute('y')), data['hotspots'][0]['bounds'][1] - 6);
        let rects = this.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay] rect');
        this.assertEqual(len(rects), len(data['hotspots']));
        for (const [rect, hotspot] of rects.map((rect, i) => [rect, data.hotspots[i]])) {
            this.assertEqual(Number(rect.get_attribute('y')), hotspot['bounds'][1] - 6);
        }
        this.find('#hotspot-list button').click();
        this.assertTrue(this.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay]'));
        this.assertEqual(this.driver.switch_to.active_element.tag_name, 'button');
        this.find('#clear').click();
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-hotspot-overlay]'), []);
        this.assertEqual(before, this.driver.execute_script("return [...document.querySelectorAll('path[data-edge-kind]')].map(p=>p.outerHTML)"));
        this.choose('one:');
        this.assertEqual(toggle.get_attribute('aria-pressed'), 'false');
        this.load('examples/plan.json', '--changed-only', '--html-diagnostics');
        this.find('#hotspot-toggle').click();
        this.load('examples/plan.json');
        this.assertFalse(this.find('#hotspot-toggle').is_enabled());
        this.assertIn('unavailable', this.find('#hotspot-status').text);
    }
    test_offline_keyboard_search_zoom_pan_clear_and_fit() {
        this.load('examples/plan.json');
        let before = this.view_box();
        let search = this.find('#search');
        search.send_keys(Keys.ARROW_DOWN);
        this.assertEqual(this.driver.switch_to.active_element.tag_name, 'button');
        this.driver.switch_to.active_element.send_keys(Keys.ENTER);
        this.assertTrue(this.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]'));
        let focused = this.view_box();
        this.assertNotEqual(before, focused);
        this.find('#zoom-in').click();
        this.assertNotEqual(focused, this.view_box());
        this.driver.execute_script("document.getElementById('viewport').focus()");
        this.driver.switch_to.active_element.send_keys(Keys.ARROW_RIGHT, Keys.ESCAPE);
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected], [data-viewer-edge], [data-viewer-related]'));
        this.find('#fit').click();
        this.assertEqual(before, this.view_box());
        let viewport = this.find('#viewport');
        this.driver.drag(viewport, 90, 40);
        this.assertNotEqual(before, this.view_box());
    }
    test_rendered_edge_semantics_and_bundles_survive_selection() {
        for (const fixture of ['examples/bundling-plan.json', 'tests/fixtures/aws-relationships-plan.json']) {
            this.load(fixture);
            let script = "return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e => [e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end'),e.dataset.directionality,e.dataset.source,e.dataset.target])";
            let before = this.driver.execute_script(script);
            let edges = this.driver.find_elements(By.CSS_SELECTOR, 'path[data-edge-kind][data-source]');
            this.assertTrue(edges);
            for (const edge of edges) {
                let source = edge.get_attribute('data-source');
                this.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", source);
                let direction = edge.get_attribute('data-directionality');
                this.assertEqual(edge.get_attribute('data-viewer-edge'), direction === 'directed' ? 'outgoing' : 'undirected');
                this.assertIn(direction === 'directed' ? 'Outgoing rendered edges' : 'Undirected rendered relationships', this.find('#selection').text);
                this.assertEqual(before, this.driver.execute_script(script));
            }
        }
    }
    test_component_membership_and_reference_navigation_are_separate() {
        this.load('tests/fixtures/components-plan.json');
        let scope = this.driver.find_elements(By.CSS_SELECTOR, '[data-component-scope]').map(n => n.get_attribute('textContent')).join('');
        this.assertIn('Multi-region', scope);
        for (const label of ['ap-northeast-1', 'us-east-1', 'Region: unknown']) {
            this.assertIn(label, scope);
        }
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-component-member]')), 3);
        this.choose('aws_lb.app');
        this.assertIn('Logical Component membership', this.find('#selection').text);
        this.find('#selection h3:last-of-type + ul button').click();
        this.assertIn('Component members', this.find('#selection').text);
        this.find('#selection button').click();
        this.assertTrue(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'));
        this.load('tests/fixtures/viewer-plan.json');
        let reference = this.find('a[data-reference-attribute]');
        let owner = this.driver.execute_script("return arguments[0].closest('[data-viewer-target]').id", reference);
        this.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", owner);
        this.assertIn('Uses / reference targets', this.find('#selection').text);
        let target = this.driver.execute_script("return arguments[0].getAttribute('href').slice(1)", reference);
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", reference);
        this.assertEqual(target, this.find('[data-viewer-selected]').get_attribute('id'));
    }
    test_named_states_and_cross_state_direction() {
        this.load('--state', 'producer=tests/fixtures/multi/producer.json', '--state', 'consumer=tests/fixtures/multi/consumer.json', '--remote-state', 'consumer:data.terraform_remote_state.network=producer');
        let edge = this.find('path[data-cross-state]');
        for (const [endpoint, direction] of [['source', 'outgoing'], ['target', 'incoming']]) {
            this.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", edge.get_attribute('data-' + endpoint));
            this.assertEqual(edge.get_attribute('data-viewer-edge'), direction);
        }
        this.assertTrue(this.search('producer'));
        this.assertTrue(this.search('producer').every(b => b.text.includes('producer:')));
    }
    test_impact_dependency_depth_direction_keyboard_and_reset() {
        let plan = {'format_version': '1.2', 'resource_changes': [['a', 'create'], ['b', 'update'], ['c', 'delete']].map(([name, action]) => ({address: 'aws_instance.' + name, type: 'aws_instance', change: {actions: [action]}})), 'configuration': {'root_module': {'resources': [['b', 'a'], ['c', 'b']].map(([name, dependency]) => ({address: 'aws_instance.' + name, expressions: {input: {references: ['aws_instance.' + dependency + '.id']}}}))}}};
        this.load('-', { input_plan: plan });
        this.choose('aws_instance.a');
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), []);
        Select(this.find('#impact-depth')).select_by_value('1');
        this.find('#impact-explore').send_keys(Keys.ENTER);
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 1);
        this.assertIn('aws_instance.b', this.find('#impact-results').text);
        Select(this.find('#impact-depth')).select_by_value('2');
        this.find('#impact-explore').click();
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop="2"]')), 1);
        this.assertIn('delete', this.find('#impact-results').text);
        this.choose('aws_instance.c');
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), []);
        Select(this.find('#impact-direction')).select_by_value('upstream');
        this.find('#impact-explore').click();
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 2);
        this.assertIn('Potentially related resources', this.find('#impact-status').text);
        this.search('aws_instance.b');
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), []);
        this.find('#impact-explore').click();
        this.find('#impact-explore').send_keys(Keys.ESCAPE);
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop], [data-impact-edge]'), []);
        this.load('-', '--focus', 'aws_instance.b', '--focus-depth', '0', { input_plan: plan });
        this.choose('aws_instance.b');
        this.find('#impact-explore').click();
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]'), []);
        this.assertIn('filtered/unavailable', this.find('#impact-status').text);
        let plan_path = join(this.temp, 'chain.json');
        writeFileSync(plan_path, JSON.stringify(plan));
        this.load('--state', 'one=' + String(plan_path), '--state', 'two=' + String(plan_path));
        this.choose('one:aws_instance.a');
        this.find('#impact-explore').click();
        let related = this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]');
        this.assertEqual(len(related), 2);
        for (const node of related) {
            this.assertEqual(this.driver.execute_script("return arguments[0].closest('[data-state-id]').dataset.stateId", node), 'one');
        }
    }
    test_impact_associations_and_cross_state_metadata() {
        this.load('tests/fixtures/relationship-scopes-plan.json');
        let edge = this.find('path[data-edge-kind="association"]');
        this.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", edge.get_attribute('data-source'));
        this.find('#impact-explore').click();
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 0);
        this.find('#impact-associations').click();
        this.find('#impact-explore').click();
        this.assertGreater(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-impact-hop]')), 0);
        this.assertIn('non-causal', this.find('#impact').text);
        this.load('--state', 'producer=tests/fixtures/multi/producer.json', '--state', 'consumer=tests/fixtures/multi/consumer.json', '--remote-state', 'consumer:data.terraform_remote_state.network=producer');
        edge = this.find('path[data-cross-state]');
        let target = edge.get_attribute('data-target');
        this.driver.execute_script("document.getElementById(arguments[0]).dispatchEvent(new MouseEvent('click', {bubbles:true}))", edge.get_attribute('data-source'));
        this.find('#impact-explore').click();
        this.assertTrue(this.driver.execute_script("return document.getElementById(arguments[0]).hasAttribute('data-impact-hop')", target));
        this.assertIn('consumer:', this.find('#impact-results').text);
    }
    test_lowered_resource_search_highlights_only_its_parallel_relationship() {
        let plan = JSON.parse(readFileSync(join(ROOT, 'tests/fixtures/relationship-scopes-plan.json'), 'utf8'));
        let second = 'aws_route_table_association.second';
        plan['resource_changes'].push({'address': second, 'type': 'aws_route_table_association', 'change': {'actions': ['delete']}});
        let config = {...plan.configuration.root_module.resources.at(-1), address: second};
        plan['configuration']['root_module']['resources'].push(config);
        this.load('-', { input_plan: plan });
        this.choose(second);
        this.assertIn('Rendered relationships from this resource', this.find('#selection').text);
        let paths = this.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]');
        this.assertEqual(len(paths), 1);
        this.assertIn(second, paths[0].find_element(By.TAG_NAME, 'title').get_attribute('textContent'));
        let before = this.driver.execute_script("return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e=>[e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end')])");
        let entry = this.find('[data-relationship-entry="2"]');
        let text = entry.find_elements(By.TAG_NAME, 'tspan').map(n => n.get_attribute('textContent')).join('');
        this.assertIn('Resource scope: Region: ap-northeast-1', text);
        this.assertIn('aws_route_table.private · Region: us-east-1', text);
        let number = entry.find_element(By.CSS_SELECTOR, '[data-relationship-select]');
        this.driver.execute_script('arguments[0].focus()', number);
        number.send_keys(Keys.ENTER);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), entry.get_attribute('id'));
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')), 1);
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-marker] [data-viewer-related]')), 2);
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '#selection ul button')), 2);
        this.find('#selection button').click();
        this.assertTrue(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'));
        this.find('#clear').click();
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]')), 0);
        let after = this.driver.execute_script("return Array.from(document.querySelectorAll('path[data-edge-kind]')).map(e=>[e.getAttribute('d'),e.getAttribute('stroke'),e.getAttribute('stroke-dasharray'),e.getAttribute('marker-end')])");
        this.assertEqual(before, after);
    }
    test_subscription_navigation_retains_provenance_and_endpoint_direction() {
        this.check_messaging_navigation('sns-subscriptions', 'aws_sns_topic_subscription.jobs', 'subscription');
    }
    test_event_source_navigation_retains_provenance_and_endpoint_direction() {
        this.check_messaging_navigation('event-sources', 'aws_lambda_event_source_mapping.jobs', 'eventsource');
    }
    test_event_target_navigation_retains_provenance_and_endpoint_direction() {
        this.check_messaging_navigation('eventbridge-targets', 'aws_cloudwatch_event_target.jobs', 'eventtarget');
    }
    check_messaging_navigation(fixture, address, kind) {
        this.load(`tests/fixtures/${fixture}-plan.json`);
        this.choose(address);
        let paths = this.driver.find_elements(By.CSS_SELECTOR, `path[data-viewer-edge][data-edge-kind="${kind}"]`);
        this.assertEqual(len(paths), 1);
        this.assertEqual(paths[0].get_attribute('data-directionality'), 'directed');
        let marker = this.find('[data-relationship-marker] a');
        this.driver.execute_script('arguments[0].dispatchEvent(new MouseEvent("click", {bubbles:true,cancelable:true}))', marker);
        let entry = this.find('[data-viewer-selected]');
        this.assertTrue(entry.get_attribute('data-relationship-entry'));
        let link = entry.find_element(By.CSS_SELECTOR, '[data-relationship-endpoint]');
        this.driver.execute_script('arguments[0].dispatchEvent(new MouseEvent("click", {bubbles:true,cancelable:true}))', link);
        this.assertTrue(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'));
    }
    test_number_selection_stays_within_its_state() {
        this.load('--state', 'one=tests/fixtures/relationship-scopes-plan.json', '--state', 'two=tests/fixtures/relationship-scopes-plan.json');
        for (const state of ['one', 'two']) {
            let marker = this.find(`[data-state-id="${state}"] [data-relationship-marker] a`);
            this.driver.execute_script('arguments[0].focus()', marker);
            marker.send_keys(Keys.SPACE);
            this.assertEqual(this.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-state-id]').dataset.stateId"), state);
            let selected_states = this.driver.execute_script("return Array.from(document.querySelectorAll('[data-viewer-related], [data-viewer-edge]')).map(e=>e.closest('[data-state-id]').dataset.stateId)");
            this.assertTrue(selected_states);
            this.assertEqual(new Set(selected_states), new Set([state]));
        }
    }
    test_logical_action_sections_preserve_selection_and_cli_filtering() {
        let fixture = 'tests/fixtures/logical-actions-plan.json';
        this.load(fixture);
        let sections = this.driver.find_elements(By.CSS_SELECTOR, '[data-resource-type="aws_iam_policy"] [data-action-section]');
        this.assertEqual(sections.map(n => n.get_attribute('data-action-section')), ['create', 'update', 'delete', 'replace', 'read', 'unchanged', 'other']);
        this.assertIn('Create (2)', sections[0].get_attribute('textContent'));
        this.assertTrue(this.driver.execute_script(`
            return Array.from(document.querySelectorAll('[data-action-section]')).every(section => {
                const border = section.querySelector(':scope > rect').getBoundingClientRect();
                return Array.from(section.querySelectorAll('[data-placement="logical"]')).every(row => {
                    const bounds = row.getBoundingClientRect();
                    return bounds.left >= border.left && bounds.right <= border.right + 0.1 &&
                           bounds.top >= border.top && bounds.bottom <= border.bottom + 0.1;
                });
            });
        `));
        let explanation = this.driver.find_elements(By.CSS_SELECTOR, '[data-role-explanation]').map(n => n.get_attribute('textContent')).join('');
        this.assertIn('Role: Policy', explanation);
        this.assertIn('Role: Controller', explanation);
        this.choose('aws_iam_policy.update');
        this.assertEqual(this.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-action-section]').dataset.actionSection"), 'update');
        this.load(fixture, '--changed-only');
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-action-section="unchanged"]'));
        this.assertFalse(this.search('aws_iam_policy.unchanged'));
        this.load(fixture, '--focus', 'aws_iam_policy.create_a', '--focus-depth', '0');
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-action-section]')), 1);
        this.assertIn('Create (1)', this.find('[data-action-section]').get_attribute('textContent'));
        this.assertFalse(this.search('aws_iam_policy.create_b'));
    }
    test_iam_memberships_and_direct_policies_keep_search_and_links() {
        this.load('tests/fixtures/iam-plan.json', '--icon-mode', 'service-name');
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-placement="logical"]')), 11);
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-role="identity"]')), 3);
        this.assertIn('AWS Identity and Access Management', this.driver.page_source);
        this.assertNotIn('PRIVATE_', this.driver.page_source);
        this.choose('aws_iam_group_membership.team');
        const row = this.find('[data-viewer-selected]');
        this.assertIn('Member users', row.get_attribute('textContent'));
        const link = this.find('[data-viewer-selected] a[data-reference-attribute="users"]');
        const target = link.get_attribute('href').split('#').at(-1);
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", link);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), target);
        this.choose('aws_iam_group_policy_attachment.read');
        this.assertEqual(this.driver.execute_script("return document.querySelector('[data-viewer-selected]').closest('[data-action-section]').dataset.actionSection"), 'update');
        this.load('tests/fixtures/iam-plan.json', '--focus', 'aws_iam_group_membership.team', '--focus-depth', '0');
        this.assertIn('Unavailable', this.find('#viewport').text);
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '#viewport a[href]')), 0);
    }
    test_organizations_root_provenance_and_offline_navigation() {
        this.load('tests/fixtures/organizations-plan.json', '--icon-mode', 'service-name');
        this.assertNotIn('PRIVATE_', this.driver.page_source);
        this.assertIn('AWS Organizations', this.driver.page_source);
        this.choose('aws_organizations_organizational_unit.engineering');
        const link = this.find('[data-viewer-selected] a[data-reference-attribute="parent_id"]');
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", link);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-component-kind'), 'organization_root');
        this.assertIn('Source Organization', this.find('#selection').text);
        this.choose('aws_organizations_organization.main');
        this.assertIn('Derived Roots', this.find('#selection').text);
        this.assertNotIn('Logical Component membership', this.find('#selection').text);
        this.load('tests/fixtures/organizations-plan.json', '--focus', 'aws_organizations_organizational_unit.engineering', '--focus-depth', '0');
        this.assertIn('Unavailable', this.find('#viewport').text);
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-component-kind="organization_root"]'));
        this.load('--state', 'one=tests/fixtures/organizations-plan.json', '--state', 'two=tests/fixtures/organizations-plan.json');
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-component-kind="organization_root"]')), 2);
        this.assertTrue(this.driver.execute_script("return Array.from(document.querySelectorAll('a[data-reference-attribute=parent_id]')).every(a => { const target = document.getElementById(a.getAttribute('href').slice(1)); return target && target.closest('[data-state-id]') === a.closest('[data-state-id]'); })"));
    }
    test_identity_provider_references_preserve_offline_navigation() {
        this.load('tests/fixtures/identity-providers-plan.json', '--icon-mode', 'service-name');
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-role="identity"]')), 5);
        this.assertIn('Role: Identity', this.find('#viewport').text);
        this.assertNotIn('Role: Policy', this.find('#viewport').text);
        this.assertIn('AWS Identity and Access Management', this.driver.page_source);
        this.assertNotIn('PRIVATE_', this.driver.page_source);
        for (const suffix of ['workload', 'workforce']) {
            this.choose('aws_iam_role.' + suffix);
            const link = this.find('[data-viewer-selected] a[data-reference-attribute="assume_role_policy"]');
            const target = link.get_attribute('href').split('#').at(-1);
            this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", link);
            this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), target);
            this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-placement'), 'logical');
            this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-role'), 'identity');
        }
        this.choose('aws_iam_role.literal');
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected] a[data-reference-attribute]'));
        this.load('tests/fixtures/identity-providers-plan.json', '--focus', 'aws_iam_role.workload', '--focus-depth', '0');
        this.assertIn('Unavailable', this.find('#viewport').text);
    }
    test_organization_policy_attachments_keep_identity_and_root_navigation() {
        this.load('tests/fixtures/organization-policies-plan.json');
        this.assertNotIn('PRIVATE_', this.driver.page_source);
        this.choose('aws_organizations_policy.guardrails');
        this.assertIn('SERVICE_CONTROL_POLICY (SCP)', this.find('[data-viewer-selected]').text);
        this.choose('aws_organizations_policy.tags');
        this.assertIn('TAG_POLICY', this.find('[data-viewer-selected]').text);
        this.choose('aws_organizations_policy_attachment.root');
        const link = this.find('[data-viewer-selected] a[data-reference-attribute="target_id"]');
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", link);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-component-kind'), 'organization_root');
        this.assertEqual(len(this.search('aws_organizations_policy_attachment.')), 4);
        this.load('tests/fixtures/organization-policies-plan.json', '--focus', 'aws_organizations_policy_attachment.account', '--focus-depth', '0');
        this.assertIn('Unavailable', this.find('#viewport').text);
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, '#viewport a[data-reference-attribute]'));
    }
    test_verified_organization_account_scope_navigation_preserves_json_unknown() {
        this.driver.get(pathToFileURL(join(ROOT, 'examples/organization-deployments.html')).href);
        this.assertNotIn('PRIVATE_', this.driver.page_source);
        this.choose('aws_organizations_account.production');
        const link = this.find('[data-viewer-selected] a[data-reference-attribute="deployment_scope"]');
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", link);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-account-panel'), 'true');
        this.assertIn('AWS Account deployment scope', this.find('#selection').text);
        const back = this.find('[data-viewer-selected] a[data-deployment-member]');
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", back);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'), 'aws_organizations_account.production');
        this.load('tests/fixtures/organization-deployments-plan.json');
        this.assertFalse(this.driver.find_elements(By.CSS_SELECTOR, 'a[data-reference-attribute="deployment_scope"]'));
        this.assertIn('no visible verified match', this.find('#viewport').text);
    }
    test_escaped_long_addresses_modes_and_individual_logical_targets() {
        let address = 'module.app["a.b"].aws_iam_role.item["</script><script>window.pwned=1</script>&\\\"' + 'x'.repeat(100) + '"]';
        let plan = {'format_version': '1.2', 'resource_changes': [ {'address': address, 'type': 'aws_iam_role', 'change': {'actions': ['create'], 'after': {'secret': 'DO_NOT_EXPOSE'}}}, {'address': 'aws_iam_role.item', 'type': 'aws_iam_role'}, {'address': 'data.aws_iam_role.item', 'type': 'aws_iam_role', 'mode': 'data'}, ]};
        this.load('-', { input_plan: plan });
        this.assertIsNone(this.driver.execute_script('return window.pwned'));
        this.assertNotIn('DO_NOT_EXPOSE', this.driver.page_source);
        this.assertEqual(len(this.search('managed')), 2);
        this.assertEqual(len(this.search('data')), 1);
        let selected = this.choose('module.app');
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-terraform-address'), address);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('data-placement'), 'logical');
        this.driver.execute_script("const row=document.getElementById(arguments[0]); const group=document.createElementNS('http://www.w3.org/2000/svg','g'); row.parentNode.insertBefore(group,row); group.append(row); group.setAttribute('data-type-group','aws_iam_role');", selected);
        this.find('#clear').click();
        this.assertEqual(selected, this.choose('module.app'));
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]')), 1);
    }
    test_rds_global_membership_preserves_state_and_embedded_selection() {
        this.load('--state', 'one=tests/fixtures/rds-global-plan.json', '--state', 'two=tests/fixtures/rds-global-plan.json');
        this.choose('one:aws_rds_cluster.east');
        let link = this.find('[data-viewer-selected] a[data-reference-attribute="global_cluster_identifier"]');
        let target = this.driver.execute_script("return arguments[0].getAttribute('href').slice(1)", link);
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", link);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), target);
        this.assertIn('one', this.find('#selection').text);
        this.assertIn('aws_rds_global_cluster.world[0]', this.find('#selection').text);
        this.choose('two:aws_rds_cluster_instance.west[0]');
        this.assertIn('aws_rds_cluster_instance.west[0]', this.find('#selection').text);
        let broken = this.driver.execute_script("return Array.from(document.querySelectorAll('#viewport a[href]')).filter(a=>!document.getElementById(a.getAttribute('href').slice(1))).length");
        this.assertEqual(broken, 0);
        this.load('tests/fixtures/rds-global-plan.json', '--focus', 'aws_rds_cluster.east', '--focus-depth', '0');
        this.assertEqual(this.driver.find_elements(By.CSS_SELECTOR, 'a[data-reference-attribute="global_cluster_identifier"]').length, 0);
    }
    test_filtered_and_unresolved_targets_do_not_gain_navigation() {
        this.load('tests/fixtures/viewer-plan.json', '--focus', 'aws_ecs_service.app', '--focus-depth', '0');
        let resources = this.driver.execute_script("return JSON.parse(document.getElementById('viewer-data').textContent)");
        this.assertEqual(len(resources), len(this.driver.find_elements(By.CSS_SELECTOR, '#results button')));
        let broken = this.driver.execute_script("return Array.from(document.querySelectorAll('#viewport a[href]')).filter(a=>!document.getElementById(a.getAttribute('href').slice(1))).length");
        this.assertEqual(broken, 0);
    }
    test_duplicate_state_addresses_deposed_objects_and_embedded_routes() {
        this.load('--state', 'one=tests/fixtures/deposed-plan.json', '--state', 'two=tests/fixtures/deposed-plan.json');
        let resources = this.driver.execute_script("return JSON.parse(document.getElementById('viewer-data').textContent)");
        let first = resources.find(r => r['state'] == 'one' && r['deposed']);
        let second = resources.find(r => r['state'] == 'two' && r['address'] == first['address'] && r['deposed'] == first['deposed']);
        this.assertNotEqual(first['id'], second['id']);
        this.assertEqual(first['id'], this.choose('one:' + first['address'] + ' ' + first['deposed']));
        this.assertEqual(second['id'], this.choose('two:' + second['qualified'] + ' ' + second['deposed']));
        this.load('tests/fixtures/routes-plan.json');
        let route = this.find('[data-embedded-route]');
        this.assertEqual(route.get_attribute('id'), this.choose(route.get_attribute('data-terraform-address')));
        this.assertIn('Outgoing rendered edges', this.find('#selection').text);
        let next_hop = this.find('[data-embedded-route] [data-relationship-select]');
        let entry_id = this.driver.execute_script("return arguments[0].getAttribute('href').slice(1)", next_hop);
        this.driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))", next_hop);
        this.assertEqual(this.find('[data-viewer-selected]').get_attribute('id'), entry_id);
        let paths = this.driver.find_elements(By.CSS_SELECTOR, 'path[data-viewer-edge]');
        this.assertEqual(len(paths), 1);
        this.assertEqual(paths[0].get_attribute('data-directionality'), 'directed');
        this.assertTrue(paths[0].get_attribute('marker-end'));
        this.assertEqual(len(this.driver.find_elements(By.CSS_SELECTOR, '[data-relationship-marker] [data-viewer-related]')), 2);
    }
}

async function main() {
    let driver;
    const temp = mkdtempSync(join(tmpdir(), 'planorama-viewer-'));
    let failed = 0;
    try {
        driver = await Driver.start();
        driver.execute_cdp_cmd('Network.enable', {});
        driver.execute_cdp_cmd('Network.emulateNetworkConditions', {offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0});
        const suite = new ViewerTests(driver, temp);
        const cases = Object.getOwnPropertyNames(ViewerTests.prototype).filter(name => name.startsWith('test_'));
        for (const name of cases) {
            try {
                try { suite[name](); } finally { suite.check_logs(); }
                console.log('PASS ' + name);
            } catch (error) { failed++; console.error('FAIL ' + name, error); }
        }
        console.log(cases.length + ' cases, ' + failed + ' failures');
    } finally {
        try { if (driver) await driver.quit(); }
        finally { rmSync(temp, {recursive: true, force: true}); }
    }
    if (failed) process.exitCode = 1;
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { ViewerTests };
