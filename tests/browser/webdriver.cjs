const { Worker, MessageChannel, receiveMessageOnPort } = require('node:worker_threads');
const { spawn } = require('node:child_process');
const { performance } = require('node:perf_hooks');

const elementKey = 'element-6066-11e4-a52e-4f735466cecf';
const Keys = { ENTER: '\uE007', ARROW_DOWN: '\uE015', ARROW_RIGHT: '\uE014', ESCAPE: '\uE00C', END: '\uE010', SPACE: '\uE00D' };
const By = { CSS_SELECTOR: 'css selector', XPATH: 'xpath', TAG_NAME: 'tag name' };

function wait(condition, description, timeout = 10000) {
  const deadline = performance.now() + timeout;
  do {
    const value = condition();
    if (value) return value;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 25);
  } while (performance.now() < deadline);
  throw new Error(`Timed out waiting for ${description}`);
}

class Element {
  constructor(driver, id) { this.driver = driver; this.id = id; }
  toJSON() { return { [elementKey]: this.id }; }
  command(method, suffix, body) { return this.driver.command(method, `/element/${this.id}/${suffix}`, body); }
  click() { this.command('POST', 'click', {}); }
  clear() { this.command('POST', 'clear', {}); }
  send_keys(...keys) { this.command('POST', 'value', { text: keys.join('') }); }
  get_attribute(name) {
    return this.driver.execute_script('const p=arguments[0][arguments[1]]; return typeof p === "string" ? p : arguments[0].getAttribute(arguments[1]);', this, name);
  }
  get text() { return this.command('GET', 'text'); }
  get tag_name() { return this.command('GET', 'name'); }
  get accessible_name() { return this.command('GET', 'computedlabel'); }
  is_enabled() { return this.command('GET', 'enabled'); }
  find_element(using, value) { return this.command('POST', 'element', { using, value }); }
  find_elements(using, value) { return this.command('POST', 'elements', { using, value }); }
}

class Driver {
  static async start(width = 1400, height = 1000) {
    const driver = new Driver();
    try {
      driver.process = spawn('/usr/bin/chromedriver', ['--port=0'], { stdio: ['ignore', 'pipe', 'pipe'] });
      const port = await new Promise((resolve, reject) => {
        let output = '';
        const timer = setTimeout(() => reject(new Error(`ChromeDriver startup timeout: ${output}`)), 10000);
        const finish = (error, value) => { clearTimeout(timer); error ? reject(error) : resolve(value); };
        driver.process.once('error', error => finish(error));
        driver.process.once('exit', code => finish(new Error(`ChromeDriver exited ${code}: ${output}`)));
        const read = chunk => {
          output += chunk;
          const match = output.match(/started successfully on port (\d+)/);
          if (match) finish(null, Number(match[1]));
        };
        driver.process.stdout.on('data', read);
        driver.process.stderr.on('data', read);
      });
      const { port1, port2 } = new MessageChannel();
      driver.port = port1;
      driver.signal = new Int32Array(new SharedArrayBuffer(4));
      driver.worker = new Worker(require.resolve('./webdriver-worker.cjs'), {
        workerData: { port: port2, signal: driver.signal, url: `http://127.0.0.1:${port}` }, transferList: [port2],
      });
      const session = driver.request('POST', '/session', { capabilities: { alwaysMatch: {
        browserName: 'chrome', 'goog:chromeOptions': { args: ['--headless', '--no-sandbox', '--disable-dev-shm-usage', `--window-size=${width},${height}`] },
        'goog:loggingPrefs': { browser: 'ALL', performance: 'ALL' },
      } } });
      driver.session = session.sessionId;
      driver.command('POST', '/timeouts', { implicit: 0, pageLoad: 30000, script: 10000 });
      return driver;
    } catch (error) {
      await driver.quit();
      throw error;
    }
  }
  request(method, path, body) {
    Atomics.store(this.signal, 0, 0);
    this.port.postMessage({ method, path, body: body === undefined ? undefined : JSON.parse(JSON.stringify(body)) });
    if (Atomics.wait(this.signal, 0, 0, 35000) === 'timed-out') throw new Error(`WebDriver timeout: ${method} ${path}`);
    const result = receiveMessageOnPort(this.port)?.message;
    if (!result || result.error) throw new Error(result?.error || 'WebDriver worker exited without a response');
    return this.decode(result.value);
  }
  decode(value) {
    if (Array.isArray(value)) return value.map(item => this.decode(item));
    if (value && typeof value === 'object') {
      if (value[elementKey]) return new Element(this, value[elementKey]);
      return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, this.decode(item)]));
    }
    return value;
  }
  command(method, path, body) { return this.request(method, `/session/${this.session}${path}`, body); }
  get(url) { this.command('POST', '/url', { url }); }
  get page_source() { return this.command('GET', '/source'); }
  get switch_to() { return { active_element: this.command('GET', '/element/active') }; }
  find_element(using, value) { return this.command('POST', '/element', { using, value }); }
  find_elements(using, value) { return this.command('POST', '/elements', { using, value }); }
  execute_script(script, ...args) { return this.command('POST', '/execute/sync', { script, args }); }
  execute_cdp_cmd(cmd, params) { return this.command('POST', '/goog/cdp/execute', { cmd, params }); }
  get_log(type) { return this.command('POST', '/se/log', { type }); }
  screenshot() { return Buffer.from(this.command('GET', '/screenshot'), 'base64'); }
  drag(element, x, y) {
    this.command('POST', '/actions', { actions: [{ type: 'pointer', id: 'mouse', parameters: { pointerType: 'mouse' }, actions: [
      { type: 'pointerMove', origin: element, x: 0, y: 0 }, { type: 'pointerDown', button: 0 },
      { type: 'pointerMove', origin: 'pointer', x, y, duration: 250 }, { type: 'pointerUp', button: 0 },
    ] }] });
    this.command('DELETE', '/actions');
  }
  async quit() {
    try { if (this.session) this.command('DELETE', ''); }
    finally {
      this.port?.close();
      if (this.worker) await this.worker.terminate();
      if (this.process?.pid && this.process.exitCode === null && this.process.signalCode === null) {
        const exited = new Promise(resolve => this.process.once('exit', resolve));
        this.process.kill();
        const timer = setTimeout(() => this.process.kill('SIGKILL'), 2000);
        try { await exited; } finally { clearTimeout(timer); }
      }
    }
  }
}

function Select(element) {
  const select = (property, value) => {
    const options = element.find_elements(By.CSS_SELECTOR, 'option');
    const option = options.find(item => property === 'text' ? item.text === value : item.get_attribute('value') === value);
    if (!option) throw new Error(`Missing select option: ${value}`);
    option.click();
  };
  return { select_by_value: value => select('value', value), select_by_visible_text: value => select('text', value) };
}

module.exports = { Driver, By, Keys, Select, wait };
