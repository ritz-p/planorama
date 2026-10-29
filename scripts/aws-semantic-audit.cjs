'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const read = name => fs.readFileSync(path.join(root, name), 'utf8').replace(/\r\n/g, '\n');
const inventory = JSON.parse(read('tests/fixtures/aws-inventory/6.70.0.json'));
const decisions = JSON.parse(read('docs/aws-semantic-decisions.json'));
const evidenceFile = 'docs/aws-semantic-evidence.json';
function extract(block, prefix = '', result = []) {
 for (const [name, attr] of Object.entries(block.attributes || {})) {
  if (attr.sensitive || attr.write_only) continue;
  const field = prefix + name;
  if ((attr.optional || attr.required) && !attr.sensitive && !attr.write_only && /(?:^|_)(?:arn|arns|id|ids|name|names|subnets|security_groups|role|bucket|source|target)$/.test(name) && !['id','arn','name','tags','tags_all','description'].includes(name)) result.push(field);
  if (attr.nested_type) extract(attr.nested_type, field + '.', result);
 }
 for (const [name, child] of Object.entries(block.block_types || {})) extract(child.block, prefix + name + '.', result);
 return result.sort();
}
function buildEvidence(schemaBytes) {
 const hash = crypto.createHash('sha256').update(schemaBytes).digest('hex');
 if (hash !== inventory.schemaSha256) throw Error('Pinned schema SHA-256 mismatch');
 const schema = JSON.parse(schemaBytes).provider_schemas['registry.terraform.io/hashicorp/aws'].resource_schemas;
 if (Object.keys(schema).sort().join() !== inventory.resources.map(r=>r.type).sort().join()) throw Error('Managed schema denominator mismatch');
 return {version:inventory.version, schemaSha256:hash, candidates:Object.fromEntries(inventory.resources.map(r=>[r.type,extract(schema[r.type].block)]))};
}
function validate(evidence) {
 if (evidence.version !== inventory.version || evidence.schemaSha256 !== inventory.schemaSha256) throw Error('Stale evidence');
 if (Object.keys(evidence.candidates).sort().join() !== inventory.resources.map(r=>r.type).sort().join()) throw Error('Evidence inventory mismatch');
 const known = new Set(inventory.resources.map(r=>r.type));
 for (const [type, decision] of Object.entries(decisions.types)) {
  if (!known.has(type)) throw Error('Unknown decision type: '+type);
  if (!['P1', 'P2', 'P3'].includes(decision.priority) || !decision.gap || !decision.status) throw Error('Incomplete decision: '+type);
  if (decision.issue !== undefined && (!Number.isInteger(decision.issue) || decision.issue <= 0)) throw Error('Invalid issue: '+type);
 }
 for (const [type, fields] of Object.entries(evidence.candidates)) {
  if (!Array.isArray(fields) || fields.some(field => typeof field !== 'string' || !/^[a-z0-9_.]+$/.test(field)) || new Set(fields).size !== fields.length) throw Error('Invalid evidence: '+type);
 }
}
function query(evidence, selector) {
 validate(evidence);
 const selected = inventory.resources.filter(r => r.type === selector || r.service === selector);
 if (!selected.length) throw Error('Unknown managed type or service: '+selector);
 return selected.map(r => ({...r, candidates:evidence.candidates[r.type],
  decision:decisions.types[r.type] || {status:'Deferred', priority:'P3', gap:'Dedicated semantics require an explicit endpoint contract and use case.'}}));
}
if (require.main === module) {
 const args=process.argv.slice(2);
 if (args.length === 2 && args[0]==='--schema') {
  const data=buildEvidence(fs.readFileSync(args[1]));
  validate(data);
  fs.writeFileSync(path.join(root,evidenceFile),JSON.stringify(data,null,2)+'\n');
 } else if (args.length === 1 && args[0]==='--check') {
  validate(JSON.parse(read(evidenceFile)));
 } else if (args.length === 1 && !args[0].startsWith('--')) {
  process.stdout.write(JSON.stringify(query(JSON.parse(read(evidenceFile)),args[0]),null,2)+'\n');
 } else throw Error('Usage: node scripts/aws-semantic-audit.cjs TYPE_OR_SERVICE | --schema SCHEMA | --check');
}
module.exports={extract,buildEvidence,validate,query};
