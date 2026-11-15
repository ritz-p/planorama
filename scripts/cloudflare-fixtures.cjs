const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '..');
const cf = 'registry.terraform.io/cloudflare/cloudflare';
const account = 'a'.repeat(32), zone = 'b'.repeat(32);
const ref = (...references) => ({references: references.flatMap(r => [r, r.slice(0, r.lastIndexOf('.'))])});
function dns() {
const defs=[
 ['cloudflare_account.team',{id:account,name:'Example team',standalone:true},{}],
 ['cloudflare_zone.example',{id:zone,name:'example.invalid',account:{id:account},type:'full'},{account:{references:['cloudflare_account.team.id']}}],
 ['cloudflare_dns_record.www',{id:'c'.repeat(32),zone_id:zone,name:'www.example.invalid',type:'CNAME',content:'origin.example.invalid',ttl:1,proxied:true},{zone_id:{references:['cloudflare_zone.example.id']}}],
 ['cloudflare_zone_dnssec.example',{id:zone,zone_id:zone,status:'active',key_tag:12345,public_key:'PRIVATE_DNSSEC_PAYLOAD'},{zone_id:{references:['cloudflare_zone.example.id']}}],
 ['cloudflare_zone.other',{id:'d'.repeat(32),name:'example.invalid',account:{id:'e'.repeat(32)},type:'full'},{}],
 ['cloudflare_dns_record.other',{id:'f'.repeat(32),zone_id:'d'.repeat(32),name:'www.example.invalid',type:'A',content:'192.0.2.1',ttl:300},{zone_id:{references:['cloudflare_zone.other.id']}}],
];
const resources=defs.map(([address,after])=>({address,mode:'managed',type:address.split('.')[0],name:address.split('.')[1],provider_name:cf,change:{actions:['create'],before:null,after,after_unknown:{},after_sensitive:{}}}));
const configuration={provider_config:{cloudflare:{name:'cloudflare',full_name:cf},'cloudflare.other':{name:'cloudflare',full_name:cf,alias:'other'}},root_module:{resources:resources.map((r,i)=>({address:r.address,type:r.type,name:r.name,mode:r.mode,provider_config_key:i<4?'cloudflare':'cloudflare.other',expressions:defs[i][2]}))}};

return {format_version:'1.2',terraform_version:'1.9.8',resource_changes:resources,configuration};
}
function base(count) {
 const v=dns();v.resource_changes=v.resource_changes.slice(0,count);v.configuration.root_module.resources=v.configuration.root_module.resources.slice(0,count);
 function add(type,name,after,expressions={}) {
  const address=type+'.'+name;
  v.resource_changes.push({address,mode:'managed',type,name,provider_name:cf,change:{actions:['create'],before:null,after,after_unknown:{},after_sensitive:{}}});
  v.configuration.root_module.resources.push({address,mode:'managed',type,name,provider_config_key:'cloudflare',expressions});
 }
 return {v,add};
}
function edge() {
 const {v,add}=base(4),accountRef={account_id:ref('cloudflare_account.team.id')},zoneRef={zone_id:ref('cloudflare_zone.example.id')};
add('cloudflare_load_balancer_monitor','https',{id:'1'.repeat(32),account_id:account,type:'https',path:'/health',expected_codes:'200',header:{Authorization:['PRIVATE_MONITOR_HEADER']}},accountRef);
add('cloudflare_load_balancer_pool','primary',{id:'2'.repeat(32),account_id:account,name:'primary',monitor:'1'.repeat(32),origins:[{name:'app',address:'origin.example.invalid',enabled:true}]},{...accountRef,monitor:ref('cloudflare_load_balancer_monitor.https.id')});
add('cloudflare_load_balancer_pool','backup',{id:'3'.repeat(32),account_id:account,name:'backup',monitor_group:'0'.repeat(32),origins:[{name:'app',address:'192.0.2.1',enabled:true}]},{...accountRef,monitor_group:ref('cloudflare_load_balancer_monitor_group.health.id')});
add('cloudflare_load_balancer','public',{id:'4'.repeat(32),zone_id:zone,name:'app.example.invalid',default_pools:['2'.repeat(32),'3'.repeat(32)],fallback_pool:'3'.repeat(32),region_pools:{WNAM:['2'.repeat(32)]},proxied:true},{...zoneRef,default_pools:ref('cloudflare_load_balancer_pool.primary.id','cloudflare_load_balancer_pool.backup.id'),fallback_pool:ref('cloudflare_load_balancer_pool.backup.id'),region_pools:ref('cloudflare_load_balancer_pool.primary.id')});
add('cloudflare_ruleset','waf',{id:'5'.repeat(32),zone_id:zone,name:'Custom WAF rules',kind:'custom',phase:'http_request_firewall_custom',rules:[{action:'block',expression:'PRIVATE_RULE_EXPRESSION'}]},zoneRef);
add('cloudflare_list','redirects',{id:'7'.repeat(32),account_id:account,name:'redirects',kind:'redirect',items:[{redirect:{source_url:'https://example.invalid/old',target_url:'https://example.invalid/new',status_code:301}}]},accountRef);
add('cloudflare_ruleset','redirects',{id:'6'.repeat(32),account_id:account,name:'Bulk redirects',kind:'root',phase:'http_request_redirect',rules:[{action:'redirect',expression:'true',action_parameters:{from_list:{name:'redirects',key:'http.request.full_uri'}}}]},{...accountRef,rules:ref('cloudflare_list.redirects.name')});
add('cloudflare_page_rule','legacy_redirect',{id:'8'.repeat(32),zone_id:zone,target:'example.invalid/old/*',actions:{forwarding_url:{url:'https://example.invalid/new',status_code:301}}},zoneRef);
add('cloudflare_ruleset','entrypoint',{id:'9'.repeat(32),zone_id:zone,name:'Zone WAF entrypoint',kind:'zone',phase:'http_request_firewall_custom',rules:[{action:'execute',expression:'true',action_parameters:{id:'5'.repeat(32)}}]},{...zoneRef,rules:ref('cloudflare_ruleset.waf.id')});
add('cloudflare_load_balancer_monitor_group','health',{id:'0'.repeat(32),account_id:account,description:'Shared health monitors',members:[{monitor_id:'1'.repeat(32),enabled:true,monitoring_only:false,must_be_healthy:true}]},{...accountRef,members:ref('cloudflare_load_balancer_monitor.https.id')});
 return v;
}
function applications() {
 const {v,add}=base(2),admin={account_id:ref('cloudflare_account.team.id')};
 const worker='10000000-0000-4000-8000-000000000001',version='20000000-0000-4000-8000-000000000002',database='30000000-0000-4000-8000-000000000003',kv='4'.repeat(32);
add('cloudflare_r2_bucket','assets',{account_id:account,id:'assets',name:'assets',jurisdiction:'eu'},admin);
add('cloudflare_workers_kv_namespace','cache',{account_id:account,id:kv,title:'cache'},admin);
add('cloudflare_d1_database','content',{account_id:account,id:database,uuid:database,name:'content'},admin);
add('cloudflare_worker','api',{account_id:account,id:worker,name:'api'},admin);
const bindings=[{type:'r2_bucket',name:'ASSETS',bucket_name:'assets',jurisdiction:'eu'},{type:'kv_namespace',name:'CACHE',namespace_id:kv},{type:'d1',name:'DB',id:database},{type:'secret_text',name:'TOKEN',text:'PRIVATE_BINDING_SECRET'}];
const storageRefs=ref('cloudflare_r2_bucket.assets.name','cloudflare_workers_kv_namespace.cache.id','cloudflare_d1_database.content.id');
bindings.push({type:'service',name:'BACKEND',service:'backend'},{type:'service',name:'BACKEND_RPC',service:'backend',entrypoint:'Handler'},{type:'service',name:'DOWNSTREAM',service:'downstream'});
bindings.push({type:'hyperdrive',name:'DATABASE',id:'77777777-7777-4777-8777-777777777777'});
const bindingRefs={references:[...storageRefs.references,...ref('cloudflare_worker.backend.name','cloudflare_workers_script.downstream.script_name','cloudflare_hyperdrive_config.database.id').references]};
add('cloudflare_worker_version','api',{account_id:account,id:version,worker_id:worker,bindings,modules:[{name:'main.js',content_type:'application/javascript+module',content_base64:'PRIVATE_WORKER_CODE'}]},{...admin,worker_id:ref('cloudflare_worker.api.id'),bindings:bindingRefs});
add('cloudflare_workers_deployment','api',{account_id:account,id:'deployment',script_name:'api',strategy:'percentage',versions:[{version_id:version,percentage:75},{version_id:'88888888-8888-4888-8888-888888888888',percentage:25}]},{...admin,script_name:ref('cloudflare_worker.api.name'),versions:ref('cloudflare_worker_version.api.id','cloudflare_worker_version.canary.id')});
add('cloudflare_workers_script','legacy',{account_id:account,id:'legacy',script_name:'legacy',content:'PRIVATE_WORKER_CODE',bindings},{...admin,bindings:bindingRefs});
add('cloudflare_workers_route','api',{id:'5'.repeat(32),zone_id:zone,pattern:'api.example.invalid/*',script:'api'},{zone_id:ref('cloudflare_zone.example.id'),script:ref('cloudflare_worker.api.name')});
add('cloudflare_workers_custom_domain','legacy',{id:'6'.repeat(32),account_id:account,hostname:'legacy.example.invalid',service:'legacy',zone_id:zone},{...admin,service:ref('cloudflare_workers_script.legacy.id'),zone_id:ref('cloudflare_zone.example.id')});
add('cloudflare_pages_project','site',{account_id:account,id:'site',name:'site',subdomain:'site.pages.dev',production_branch:'main',deployment_configs:{preview:{kv_namespaces:{CACHE:{namespace_id:kv}}},production:{d1_databases:{DB:{id:database}},r2_buckets:{ASSETS:{name:'assets',jurisdiction:'eu'}},services:{BACKEND:{service:'backend',entrypoint:'Handler'},LEGACY:{service:'downstream'}}}}},{...admin,deployment_configs:{references:[...storageRefs.references,...ref('cloudflare_worker.backend.name','cloudflare_workers_script.downstream.script_name').references]}});
add('cloudflare_pages_domain','site',{account_id:account,id:'www.example.invalid',name:'www.example.invalid',project_name:'site'},{...admin,project_name:ref('cloudflare_pages_project.site.name')});
add('cloudflare_worker','backend',{account_id:account,id:'90000000-0000-4000-8000-000000000009',name:'backend'},admin);
add('cloudflare_workers_script','downstream',{account_id:account,id:'downstream',script_name:'downstream',content:'PRIVATE_DOWNSTREAM_CODE'},admin);
add('cloudflare_dns_record','pages',{id:'7'.repeat(32),zone_id:zone,name:'www.example.invalid',type:'CNAME',content:'site.pages.dev',proxied:true,ttl:1},{zone_id:ref('cloudflare_zone.example.id'),content:ref('cloudflare_pages_project.site.subdomain')});
add('cloudflare_hyperdrive_config','database',{account_id:account,id:'77777777-7777-4777-8777-777777777777',name:'database',origin:{database:'PRIVATE_DATABASE',host:'PRIVATE_ORIGIN.invalid',password:'PRIVATE_PASSWORD',scheme:'postgres',user:'PRIVATE_USER'}},admin);
add('cloudflare_worker_version','canary',{account_id:account,id:'88888888-8888-4888-8888-888888888888',worker_id:worker,modules:[{name:'main.js',content_type:'application/javascript+module',content_base64:'PRIVATE_CANARY_CODE'}]},{...admin,worker_id:ref('cloudflare_worker.api.id')});

 return v;
}
function queues() {
 const {v,add}=base(1),admin={account_id:ref('cloudflare_account.team.id')};
 const jobs='1'.repeat(32),audit='2'.repeat(32),kv='3'.repeat(32),pull='8'.repeat(32),notifications='9'.repeat(32);
 const bindings=[{type:'kv_namespace',name:'CACHE',namespace_id:kv},{type:'service',name:'BACKEND',service:'backend'},{type:'queue',name:'JOBS',queue_name:'jobs'},{type:'queue',name:'RETRY',queue_name:'jobs'},{type:'queue',name:'AUDIT',queue_name:'audit'}];
 const bindingRefs=ref('cloudflare_workers_kv_namespace.cache.id','cloudflare_worker.backend.name','cloudflare_queue.jobs.queue_name','cloudflare_queue.audit.queue_name');
 add('cloudflare_worker','backend',{account_id:account,id:'10000000-0000-4000-8000-000000000001',name:'backend'},admin);
 add('cloudflare_workers_script','legacy',{account_id:account,id:'legacy',script_name:'legacy',content:'PRIVATE_WORKER_CODE',bindings},{...admin,bindings:bindingRefs});
 add('cloudflare_workers_kv_namespace','cache',{account_id:account,id:kv,title:'cache'},admin);
 add('cloudflare_queue','jobs',{account_id:account,id:jobs,queue_id:jobs,queue_name:'jobs',consumers:[{script_name:'PRIVATE_COMPUTED_CONSUMER',type:'worker'}],producers:[{script:'PRIVATE_COMPUTED_PRODUCER',type:'worker'}]},admin);
 add('cloudflare_queue','audit',{account_id:account,id:audit,queue_id:audit,queue_name:'audit'},admin);
 add('cloudflare_worker_version','backend',{account_id:account,id:'20000000-0000-4000-8000-000000000002',worker_id:'10000000-0000-4000-8000-000000000001',bindings,modules:[{name:'main.js',content_type:'application/javascript+module',content_base64:'PRIVATE_WORKER_CODE'}]},{...admin,worker_id:ref('cloudflare_worker.backend.id'),bindings:bindingRefs});
 add('cloudflare_queue_consumer','worker',{account_id:account,consumer_id:'4'.repeat(32),queue_id:jobs,type:'worker',script_name:'backend',dead_letter_queue:'audit'},{...admin,queue_id:ref('cloudflare_queue.jobs.queue_id'),script_name:ref('cloudflare_worker.backend.name'),dead_letter_queue:ref('cloudflare_queue.audit.queue_name')});
 add('cloudflare_queue_consumer','legacy',{account_id:account,consumer_id:'5'.repeat(32),queue_id:audit,type:'worker',script_name:'legacy'},{...admin,queue_id:ref('cloudflare_queue.audit.id'),script_name:ref('cloudflare_workers_script.legacy.script_name')});
 add('cloudflare_queue_consumer','pull',{account_id:account,consumer_id:'6'.repeat(32),queue_id:pull,type:'http_pull',settings:{visibility_timeout_ms:30000}},{...admin,queue_id:ref('cloudflare_queue.pull.id')});
 add('cloudflare_queue_consumer','notification',{account_id:account,consumer_id:'7'.repeat(32),queue_id:notifications,type:'notification',settings:{webhooks:[{id:'PRIVATE_WEBHOOK_ID'}]}},{...admin,queue_id:ref('cloudflare_queue.notifications.queue_id')});
 add('cloudflare_queue','pull',{account_id:account,id:pull,queue_id:pull,queue_name:'pull'},admin);
 add('cloudflare_queue','notifications',{account_id:account,id:notifications,queue_id:notifications,queue_name:'notifications'},admin);
 return v;
}
function zeroTrust() {
 const {v,add}=base(2),admin={account_id:ref('cloudflare_account.team.id')},zoneRef={zone_id:ref('cloudflare_zone.example.id')};
 const id=n=>`${n}0000000-0000-4000-8000-00000000000${n}`;
add('cloudflare_zero_trust_access_identity_provider','corporate',{id:id(1),account_id:account,name:'Corporate login',type:'oidc',config:{client_id:'PRIVATE_CLIENT_ID',client_secret:'PRIVATE_IDENTITY_SECRET',auth_url:'https://login.example.invalid/auth',token_url:'https://login.example.invalid/token',certs_url:'https://login.example.invalid/certs'}},admin);
add('cloudflare_zero_trust_access_policy','staff',{id:id(2),account_id:account,name:'Staff access',decision:'allow',include:[{email:{email:'PRIVATE_POLICY_EMAIL'}}]},admin);
add('cloudflare_zero_trust_access_application','portal',{id:id(3),zone_id:zone,name:'Portal',type:'self_hosted',domain:'portal.example.invalid',allowed_idps:[id(1)],policies:[{id:id(2),precedence:1}]},{...zoneRef,allowed_idps:ref('cloudflare_zero_trust_access_identity_provider.corporate.id'),policies:ref('cloudflare_zero_trust_access_policy.staff.id')});
add('cloudflare_zero_trust_tunnel_cloudflared','office',{id:id(4),account_id:account,name:'Office',config_src:'cloudflare',tunnel_secret:'PRIVATE_TUNNEL_SECRET'},admin);
v.resource_changes.at(-1).change.after_sensitive={tunnel_secret:true};
add('cloudflare_zero_trust_tunnel_cloudflared_config','office',{id:id(4),account_id:account,tunnel_id:id(4),config:{ingress:[{hostname:'portal.example.invalid',service:'http://192.0.2.10:8080'},{service:'http_status:404'}]}},{...admin,tunnel_id:ref('cloudflare_zero_trust_tunnel_cloudflared.office.id')});
add('cloudflare_zero_trust_tunnel_cloudflared_virtual_network','office',{id:id(5),account_id:account,name:'Office routing'},admin);
add('cloudflare_zero_trust_tunnel_cloudflared_route','office',{id:id(6),account_id:account,tunnel_id:id(4),network:'192.0.2.0/24',virtual_network_id:id(5)},{...admin,tunnel_id:ref('cloudflare_zero_trust_tunnel_cloudflared.office.id'),virtual_network_id:ref('cloudflare_zero_trust_tunnel_cloudflared_virtual_network.office.id')});
add('cloudflare_zero_trust_access_group','employees',{id:id(7),account_id:account,name:'Employees',include:[{email:{email:'PRIVATE_GROUP_EMAIL'}}]},{});
add('cloudflare_zero_trust_access_group','trusted',{id:id(8),account_id:account,name:'Trusted devices',include:[{ip:{ip:'PRIVATE_GROUP_IP'}}]},{});
const policy=v.resource_changes.find(r=>r.type==='cloudflare_zero_trust_access_policy');
policy.change.after.include.push({group:{id:id(7)}});
policy.change.after.require=[{group:{id:id(8)}}];
policy.change.after.exclude=[{group:{id:id(7)}}];
const policyConfig=v.configuration.root_module.resources.find(r=>r.address===policy.address);
policyConfig.expressions={...policyConfig.expressions,include:ref('cloudflare_zero_trust_access_group.employees.id'),require:ref('cloudflare_zero_trust_access_group.trusted.id'),exclude:ref('cloudflare_zero_trust_access_group.employees.id')};
add('cloudflare_dns_record','portal',{id:'9'.repeat(32),zone_id:zone,name:'portal.example.invalid',type:'CNAME',content:`${id(4)}.cfargotunnel.com`,proxied:true,ttl:1},{...zoneRef,content:ref('cloudflare_zero_trust_tunnel_cloudflared.office.id')});

 return v;
}
function fixtures() {
 return new Map([['dns',dns()],['edge',edge()],['applications',applications()],['queues',queues()],['zero-trust',zeroTrust()]]);
}
const encode=value=>JSON.stringify(value,null,2)+'\n';
function outputs() {
 const inventory=JSON.parse(fs.readFileSync(path.join(root,'tests/fixtures/cloudflare-inventory/5.28.1.json')));
 const files=new Map([...fixtures()].map(([name,value])=>['tests/fixtures/cloudflare-'+name+'-plan.json',encode(value)]));
 const provenance={kind:'synthetic',terraformVersion:'1.9.8',provider:cf,providerVersion:inventory.version,providerCommit:inventory.commit,schemaSha256:inventory.schemaSha256,generator:'scripts/cloudflare-fixtures.cjs',coreExpressionShapeEvidence:'tests/fixtures/cloudflare-core-shapes.json',adversarialFixtures:[{file:'tests/fixtures/cloudflare-foundation-plan.json',kind:'hand-authored-adversarial',sha256:crypto.createHash('sha256').update(fs.readFileSync(path.join(root,'tests/fixtures/cloudflare-foundation-plan.json'))).digest('hex')}],limits:['Not captured from a Cloudflare API deployment or a provider-generated plan.','Known identifiers and create actions are synthesized to exercise reference and lifecycle presentation.','Payload markers are artificial privacy canaries, not credentials or executable Worker modules.','The separate foundation fixture intentionally contains adversarial fields and unsupported identities.'],fixtures:[...files].map(([file,bytes])=>({file,sha256:crypto.createHash('sha256').update(bytes).digest('hex')}))};
 files.set('tests/fixtures/cloudflare-provenance.json',encode(provenance));
 return files;
}
if(require.main===module) {
 if(process.argv.length>3 || (process.argv[2] && process.argv[2]!=='--check')) throw new Error('Usage: node scripts/cloudflare-fixtures.cjs [--check]');
 for(const [file,bytes] of outputs()) {
  const target=path.join(root,file);
  if(process.argv[2]==='--check') {if(fs.readFileSync(target,'utf8')!==bytes)throw new Error('Regenerate '+file);}
  else fs.writeFileSync(target,bytes);
 }
 console.log('Cloudflare synthetic fixtures and provenance are current');
}
module.exports={fixtures,outputs};
