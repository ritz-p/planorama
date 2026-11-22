const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const root = path.resolve(__dirname, '..');
const fixtures = path.join(root, 'tests/fixtures/cloudflare-matrix');
const matrix = require(path.join(fixtures, 'matrix.json'));
const cores = require('../tests/fixtures/compatibility/matrix.json').terraform;
const original = path.join(root, 'tests/fixtures/cloudflare-captured');
const cache = path.join(root, 'output/cloudflare-matrix');
const provider = 'registry.terraform.io/cloudflare/cloudflare';
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const read = file => JSON.parse(fs.readFileSync(file));
const write = (file, data) => fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
const phase = process.argv[2];
assert(['prepare','capture','check'].includes(phase), 'Expected prepare, capture or check');
const mirror = path.join(cache, 'mirror');
const config = path.join(cache, 'terraform.rc');

function archive(file, url, expected) {
  fs.mkdirSync(path.dirname(file), {recursive:true});
  if (!fs.existsSync(file)) execFileSync('curl',['--fail','--location','--silent','--show-error',url,'-o',file]);
  assert.equal(hash(fs.readFileSync(file)), expected, 'Unexpected archive checksum for ' + file);
}
if (phase === 'prepare') {
  for (const core of matrix.terraform) {
    const dir = path.join(cache,core);
    const file = path.join(dir,'terraform.zip');
    archive(file, 'https://releases.hashicorp.com/terraform/'+core+'/terraform_'+core+'_linux_amd64.zip', cores[core]);
    execFileSync('unzip',['-oq',file,'-d',dir]);
  }
  for (const [version, checksum] of Object.entries(matrix.provider)) {
    const name = 'terraform-provider-cloudflare_'+version+'_linux_amd64.zip';
    archive(path.join(mirror,provider,name), 'https://github.com/cloudflare/terraform-provider-cloudflare/releases/download/v'+version+'/'+name, checksum);
  }
  fs.writeFileSync(config,'disable_checkpoint = true\nprovider_installation {\n filesystem_mirror {\n  path = '+JSON.stringify(mirror)+'\n }\n}\n');
} else {
  const baseline = read(path.join(original,'create.json'));
  const addresses = plan => plan.resource_changes.map(c => [c.address,c.provider_name,c.change.actions]).sort();
  const baselineSchema = read(path.join(original, 'schema.json')).resource_schemas;
  const orderedSchema = Object.fromEntries(Object.keys(baselineSchema).sort().map(t => [t, baselineSchema[t]]));
  const schemas = new Map([['5.28.1',hash(JSON.stringify(orderedSchema))]]);
  for (const core of matrix.terraform) for (const version of Object.keys(matrix.provider)) {
    const inherited = core === '1.9.8' && version === '5.28.1';
    const dir = path.join(fixtures,core+'-'+version);
    const input = fs.readFileSync(path.join(root,matrix.source),'utf8')
      .replace('required_version = "= 1.9.8"', 'required_version = "= '+core+'"')
      .replace('version = "= 5.28.1"', 'version = "= '+version+'"');
    const lock = version === '5.28.1' ? path.join(original,'.terraform.lock.hcl') : path.join(fixtures,version+'.lock.hcl');
    const planFile = inherited ? path.join(original,'create.json') : path.join(dir,'plan.json');
    if (phase === 'capture' && !inherited) {
      fs.mkdirSync(dir,{recursive:true});
      const cwd = fs.mkdtempSync(path.join(os.tmpdir(),'planorama-cloudflare-'));
      const run = args => execFileSync(path.join(cache,core,'terraform'), args, {cwd, maxBuffer:128*1024*1024, timeout:180000,
        env:{PATH:process.env.PATH, HOME:process.env.HOME, CHECKPOINT_DISABLE:'1', TF_CLI_CONFIG_FILE:config,
          TF_IN_AUTOMATION:'1', CLOUDFLARE_BASE_URL:'http://127.0.0.1:1/', CLOUDFLARE_API_TOKEN:'fixture-only'.padEnd(40,'0')}});
      try {
        fs.writeFileSync(path.join(cwd,'main.tf'),input);
        fs.copyFileSync(lock,path.join(cwd,'.terraform.lock.hcl'));
        run(['init','-backend=false','-input=false','-lockfile=readonly']);
        const selection = JSON.parse(run(['version','-json']));
        assert.equal(selection.terraform_version,core);
        assert.deepEqual(selection.provider_selections,{[provider]:version});
        run(['plan','-refresh=false','-input=false','-out=fixture.tfplan']);
        const plan = JSON.parse(run(['show','-json','fixture.tfplan']));
        delete plan.timestamp;
        plan.relevant_attributes?.sort((a,b)=>Buffer.compare(Buffer.from(JSON.stringify(a)),Buffer.from(JSON.stringify(b))));
        write(planFile,plan);
        const full = JSON.parse(run(['providers','schema','-json'])).provider_schemas[provider].resource_schemas;
        const types = [...new Set(plan.resource_changes.map(r=>r.type))].sort();
        const excerpt = Object.fromEntries(types.map(t=>[t,full[t]]));
        const cleaned = JSON.parse(JSON.stringify(excerpt,(k,v)=>['description','description_kind'].includes(k)?undefined:v));
        write(path.join(dir,'capture.json'),{terraform:core, provider:version, planSha256:hash(fs.readFileSync(planFile)),
          inputSha256:hash(input), lockSha256:hash(fs.readFileSync(lock)), schemaSha256:hash(JSON.stringify(cleaned))});
      } finally { fs.rmSync(cwd,{recursive:true,force:true}); }
    }
    const plan = read(planFile);
    assert.equal(plan.terraform_version,core);
    assert.equal(plan.configuration.provider_config.cloudflare.version_constraint,version);
    assert.equal(plan.format_version,'1.2');
    assert.equal(plan.complete,true);
    assert.equal(plan.errored,false);
    assert.deepEqual(addresses(plan),addresses(baseline));
    const dns = plan.configuration.root_module.resources.find(r=>r.type==='cloudflare_dns_record');
    assert.deepEqual(dns.expressions.zone_id.references,['cloudflare_zone.example.id','cloudflare_zone.example']);
    if (!inherited) {
      const evidence = read(path.join(dir,'capture.json'));
      const {schemaSha256, ...contract} = evidence;
      assert.deepEqual(contract,{terraform:core,provider:version,planSha256:hash(fs.readFileSync(planFile)),
        inputSha256:hash(input),lockSha256:hash(fs.readFileSync(lock))});
      assert.match(schemaSha256,/^[a-f0-9]{64}$/);
      if (schemas.has(version)) assert.equal(schemas.get(version),schemaSha256);
      schemas.set(version,schemaSha256);
    }
    console.log(core,version,plan.resource_changes.length,'managed resources');
  }
}
