'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {extract,buildEvidence,render}=require('../../scripts/aws-semantic-audit.cjs');
const read=p=>JSON.parse(fs.readFileSync(p));
test('candidate extraction respects nested collections and sensitive boundaries',()=>{
 assert.deepEqual(extract({attributes:{role_arn:{optional:true,type:'string'},computed_id:{computed:true},secret_id:{optional:true,sensitive:true},write_id:{optional:true,write_only:true},secret:{sensitive:true,nested_type:{attributes:{target_id:{optional:true}}}}},block_types:{network:{block:{attributes:{subnets:{required:true,type:['set','string']},security_groups:{optional:true}}}}}}),['network.security_groups','network.subnets','role_arn']);
});
test('untrusted schema bytes cannot replace pinned evidence',()=>{
 assert.throws(()=>buildEvidence(Buffer.from('{}')),/SHA-256/);
});
test('register covers every exact managed type once and rejects incomplete evidence',()=>{
 const inventory=read('tests/fixtures/aws-inventory/6.70.0.json');
 const evidence=read('docs/aws-semantic-evidence.json');
 const output=render(evidence);
 const names=[...output.matchAll(/^\| \[`(aws_[^`]+)`\]/gm)].map(m=>m[1]);
 assert.deepEqual(names.sort(),inventory.resources.map(r=>r.type).sort());
 assert.equal(output,fs.readFileSync('docs/aws-semantic-gaps.md','utf8'));
 delete evidence.candidates.aws_msk_cluster;
 assert.throws(()=>render(evidence),/inventory mismatch/);
});
test('representative evidence includes nested network paths without asserting implementation',()=>{
 const {candidates}=read('docs/aws-semantic-evidence.json');
 assert.ok(candidates.aws_msk_cluster.includes('broker_node_group_info.client_subnets'));
 assert.ok(candidates.aws_apprunner_service.includes('network_configuration.egress_configuration.vpc_connector_arn'));
 const decisions=read('docs/aws-semantic-decisions.json').types;
 assert.equal(decisions.aws_msk_cluster.status,'Basic-only');
 assert.equal(decisions.aws_dynamodb_table.status,'Partial');
 assert.match(decisions.aws_dynamodb_table.gap,/NOT implemented/);
});
